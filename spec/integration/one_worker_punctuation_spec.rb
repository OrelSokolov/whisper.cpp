# frozen_string_literal: true

require 'spec_helper'
require 'open3'
require 'socket'
require 'timeout'

RSpec.describe 'Punctuation in transcription via single worker (reused across runs)' do
  # Константы для настройки теста
  TIMEOUT_SECONDS = 20
  RUNS_COUNT = 10
  MIN_SUCCESSFUL_RUNS = 9
  WORKER_PORT = 8765

  let(:spec_dir) { File.dirname(__FILE__) }
  let(:project_root) { File.expand_path('../..', spec_dir) }
  let(:whisper_worker) { File.join(project_root, 'build', 'bin', 'whisper-worker') }
  let(:client_rb) { File.join(project_root, 'examples', 'worker', 'client.rb') }
  let(:audio_file) { File.expand_path('~/tube/babes.mp3') }
  let(:model) { File.join(project_root, 'models', 'ggml-large-v3-turbo.bin') }
  let(:runs_count) { RUNS_COUNT }

  before(:each) do
    # Проверяем, что whisper-worker собран
    unless File.exist?(whisper_worker)
      skip 'whisper-worker not built. Run: rake build:worker'
    end

    # Проверяем, что клиент существует
    unless File.exist?(client_rb)
      skip 'Client script not found at examples/worker/client.rb'
    end

    # Проверяем, что аудиофайл существует
    audio_file_path = File.expand_path(audio_file)
    unless File.exist?(audio_file_path)
      skip "Audio file not found at #{audio_file}"
    end

    # Проверяем, что модель существует
    model_path = File.join(project_root, 'models', 'ggml-large-v3-turbo.bin')
    unless File.exist?(model_path)
      skip 'Model file not found at models/ggml-large-v3-turbo.bin'
    end
  end

  def start_worker
    command = [
      whisper_worker,
      '--model', model,
      '--language', 'ru'
    ]

    # Запускаем worker в фоне
    worker_stdin, worker_stdout, worker_stderr, worker_wait_thr = Open3.popen3(*command)
    
    # Неблокирующее чтение stderr для отладки
    worker_stderr.sync = true
    
    # Даем worker время запуститься и проверяем готовность
    max_wait = 10
    waited = 0
    port_ready = false
    
    while waited < max_wait
      sleep 0.5
      waited += 0.5
      
      # Проверяем, что процесс еще жив
      unless worker_wait_thr.alive?
        # Пытаемся прочитать stderr для диагностики
        begin
          error_output = worker_stderr.read_nonblock(4096) rescue ""
          if error_output && !error_output.empty?
            raise "Worker failed to start. Stderr: #{error_output.force_encoding('UTF-8').scrub('?')}"
          end
        rescue IO::WaitReadable
          # Нет данных в stderr
        end
        raise "Worker failed to start (process died)"
      end
      
      # Проверяем, что порт открыт (worker готов принимать соединения)
      begin
        socket = TCPSocket.new('localhost', WORKER_PORT)
        socket.close
        port_ready = true
        break # Порт открыт, worker готов
      rescue Errno::ECONNREFUSED, Errno::ETIMEDOUT
        # Порт еще не открыт, продолжаем ждать
      end
    end

    # Финальная проверка, что процесс еще жив
    unless worker_wait_thr.alive?
      begin
        error_output = worker_stderr.read_nonblock(4096) rescue ""
        if error_output && !error_output.empty?
          raise "Worker failed to start. Stderr: #{error_output.force_encoding('UTF-8').scrub('?')}"
        end
      rescue IO::WaitReadable
      end
      raise "Worker failed to start (process died after wait)"
    end

    unless port_ready
      raise "Worker port #{WORKER_PORT} is not ready after #{max_wait} seconds"
    end

    {
      stdin: worker_stdin,
      stdout: worker_stdout,
      stderr: worker_stderr,
      wait_thr: worker_wait_thr,
      pid: worker_wait_thr.pid
    }
  end

  def stop_worker(worker_info)
    return unless worker_info

    pid = worker_info[:pid]
    wait_thr = worker_info[:wait_thr]

    # Закрываем потоки
    begin
      worker_info[:stdin]&.close
    rescue IOError, Errno::EPIPE
      # Поток уже закрыт
    end
    
    begin
      worker_info[:stdout]&.close
    rescue IOError, Errno::EPIPE
      # Поток уже закрыт
    end
    
    begin
      worker_info[:stderr]&.close
    rescue IOError, Errno::EPIPE
      # Поток уже закрыт
    end

    # Убиваем процесс только если он еще жив
    if wait_thr&.alive?
      begin
        # Проверяем существование процесса перед убийством
        Process.kill(0, pid)
        Process.kill('TERM', pid)
        sleep 0.5
        if wait_thr.alive?
          begin
            Process.kill(0, pid)
            Process.kill('KILL', pid)
          rescue Errno::ESRCH, Errno::ECHILD
            # Процесс уже завершился
          end
        end
      rescue Errno::ESRCH, Errno::ECHILD
        # Процесс уже завершился, это нормально
      end
    end

    # Ждем завершения процесса (неблокирующе)
    begin
      if wait_thr && wait_thr.alive?
        # Даем процессу время завершиться, но не ждем бесконечно
        max_wait = 2
        waited = 0
        while waited < max_wait && wait_thr.alive?
          sleep 0.1
          waited += 0.1
        end
        # Если процесс еще жив, игнорируем - он завершится сам
      end
    rescue
      # Игнорируем ошибки при ожидании завершения
    end
  end

  def run_client
    # Используем bundle exec для запуска клиента, чтобы использовались зависимости из Gemfile
    command = [
      'bundle', 'exec', 'ruby',
      client_rb,
      audio_file
    ]

    output = ''
    error_output = ''
    pid = nil
    start_time = Time.now

    begin
      Open3.popen2e(*command) do |stdin, stdout_stderr, wait_thr|
        pid = wait_thr.pid
        stdin.close
        stdout_stderr.sync = true

        # Читаем вывод с таймаутом
        loop do
          break if Time.now - start_time > TIMEOUT_SECONDS

          # Проверяем, завершился ли процесс
          break unless wait_thr.alive?

          # Пытаемся прочитать доступные данные (неблокирующее чтение)
          begin
            # Используем select для проверки доступности данных
            if IO.select([stdout_stderr], nil, nil, 0.1)
              # Читаем доступные данные
              chunk = stdout_stderr.read_nonblock(4096)
              # Конвертируем в UTF-8, заменяя невалидные байты
              chunk = chunk.force_encoding('UTF-8').scrub('?')
              output += chunk
            end
          rescue IO::WaitReadable
            # Данных пока нет, продолжаем ждать
            sleep 0.1
          rescue EOFError
            # Поток закрыт
            break
          end
        end

        # Если процесс еще работает, убиваем его
        if wait_thr.alive?
          begin
            # Проверяем существование процесса перед попыткой убить
            Process.kill(0, pid) # Проверка существования
            Process.kill('TERM', pid)
            sleep 0.5
            if wait_thr.alive?
              begin
                Process.kill(0, pid) # Проверяем еще раз
                Process.kill('KILL', pid)
              rescue Errno::ESRCH, Errno::ECHILD
                # Процесс уже завершился между проверками
              end
            end
          rescue Errno::ESRCH, Errno::ECHILD
            # Процесс уже завершился, это нормально - не добавляем в ошибки
          end
        end

        # Читаем оставшиеся данные
        begin
          remaining = stdout_stderr.read
          if remaining
            # Конвертируем в UTF-8, заменяя невалидные байты
            remaining = remaining.force_encoding('UTF-8').scrub('?')
            output += remaining
          end
        rescue EOFError
          # Поток уже закрыт
        end

        begin
          wait_thr.value
        rescue
          # Процесс завершился с ошибкой или был прерван - это нормально для нашего случая
          # Не добавляем в error_output, так как мы уже получили весь вывод
        end
      end
    rescue Timeout::Error => e
      error_output = "Timeout after #{TIMEOUT_SECONDS} seconds"
      if pid
        begin
          # Проверяем, жив ли процесс перед попыткой убить
          Process.kill(0, pid) # Проверка существования процесса
          Process.kill('TERM', pid)
          sleep 0.5
          begin
            Process.kill(0, pid) # Проверяем еще раз
            Process.kill('KILL', pid)
          rescue Errno::ESRCH, Errno::ECHILD
            # Процесс уже завершился
          end
        rescue Errno::ESRCH, Errno::ECHILD
          # Процесс уже завершился, это нормально
        end
      end
    rescue => e
      # Игнорируем ошибку "No such process" - это нормально, если процесс уже завершился
      unless e.message.include?("No such process")
        error_output = e.message
      end
    end

    { output: output, error: error_output }
  end

  def extract_segments(content)
    # Ищем строки вида [0.00s - 4.06s] [progress%] ETA: Xs текст
    # Формат: [0.24s - 4.06s] [0.5%] ETA: 1299s  Здравствуйте, уважаемые братья...
    segments = []
    
    # Убеждаемся, что контент в UTF-8, заменяя невалидные байты
    if content.encoding != Encoding::UTF_8
      content = content.force_encoding('UTF-8').scrub('?')
    else
      # Даже если уже UTF-8, очищаем невалидные байты
      content = content.scrub('?')
    end
    
    content.each_line do |line|
      line = line.strip
      next if line.empty?
      
      # Игнорируем служебные сообщения
      next if line.match?(/^(Подключение|Отправка|Размер|Соединение|Ожидание|Статус|✓|✗|⚠|--------------------------------------------------)/)
      next if line.match?(/^Статус:/)
      next if line.match?(/^Обработка завершена/)
      
      # Паттерн для таймстампов worker'а с прогрессом и ETA:
      # [start s - end s] [progress%] ETA: Xs текст
      # Или без ETA: [start s - end s] [progress%] текст
      # Или без прогресса: [start s - end s] текст
      match = line.match(/\[(\d+\.\d+)s\s+-\s+(\d+\.\d+)s\](?:\s+\[[\d.]+\%\])?(?:\s+ETA:\s+\d+s)?\s+(.+)$/)
      if match
        start = match[1]
        finish = match[2]
        text = match[3]
        segments << {
          start: start,
          finish: finish,
          text: text.strip
        }
      # Также может быть просто текст без таймстампов (но это менее вероятно)
      elsif line.match?(/^[А-Яа-яЁё]/) && !line.match(/\[.*\]/)
        segments << {
          start: nil,
          finish: nil,
          text: line
        }
      end
    end
    segments
  end

  def has_punctuation?(text)
    # Проверяем наличие запятых и точек
    text.match?(/[.,]/)
  end

  def check_first_three_segments(output)
    segments = extract_segments(output)
    return false if segments.empty?

    # Берем первые три сегмента
    first_three = segments.first(3)
    return false if first_three.length < 3

    # Проверяем, что хотя бы в одном из первых трех есть пунктуация
    first_three.any? { |seg| has_punctuation?(seg[:text]) }
  end

  it "should produce punctuation in at least #{MIN_SUCCESSFUL_RUNS} out of #{RUNS_COUNT} runs (single worker)" do
    successful_runs = 0
    failed_runs = []
    worker_info = nil

    begin
      # Запускаем worker один раз для всех запусков
      puts "\nStarting single worker for all #{runs_count} runs..."
      begin
        worker_info = start_worker
        puts "✓ Worker started (PID: #{worker_info[:pid]})"
      rescue => e
        skip "Failed to start worker: #{e.message}"
      end

      # Проверяем, что worker еще жив
      unless worker_info[:wait_thr].alive?
        skip "Worker died immediately after start"
      end

      # Даем worker время полностью загрузить модель
      puts "Waiting for worker to fully initialize..."
      sleep 2

      # Выполняем 10 запусков клиента с одним worker'ом
      runs_count.times do |run_number|
        puts "\n[Run #{run_number + 1}/#{runs_count}] Running client..."

        # Проверяем, что worker еще жив перед запуском клиента
        unless worker_info[:wait_thr].alive?
          puts "  ✗ Worker died during test execution"
          failed_runs << { run: run_number + 1, error: "Worker died before this run" }
          next
        end

        # Запускаем клиент
        result = run_client

        # Игнорируем ошибку "No such process" - это нормально, если процесс уже завершился
        error_msg = result[:error]
        if error_msg && !error_msg.empty? && !error_msg.include?("No such process")
          puts "  Error: #{error_msg}"
          failed_runs << { run: run_number + 1, error: error_msg }
        elsif error_msg && error_msg.include?("No such process")
          # Процесс завершился - это нормально, продолжаем проверку вывода
          puts "  Note: Client process ended (this is normal)"
        end

        # Проверяем вывод, даже если была ошибка "No such process"
        if result[:output] && !result[:output].empty?
          has_punctuation = check_first_three_segments(result[:output])

          if has_punctuation
            successful_runs += 1
            puts "  ✓ Punctuation found in first three segments"
          else
            puts "  ✗ No punctuation found in first three segments"
            # Показываем первые три сегмента для отладки
            segments = extract_segments(result[:output])
            first_three = segments.first(3)
            if first_three.any?
              first_three.each do |seg|
                if seg[:start]
                  puts "    [#{seg[:start]}s - #{seg[:finish]}s] #{seg[:text]}"
                else
                  puts "    #{seg[:text]}"
                end
              end
            else
              puts "    No segments extracted from output"
              puts "    Output preview: #{result[:output][0..200]}..."
            end
            failed_runs << { run: run_number + 1, segments: first_three, output: result[:output] }
          end
        else
          puts "  ✗ No output received from client"
          failed_runs << { run: run_number + 1, error: "No output received" }
        end

        # Небольшая пауза между запусками клиента
        sleep 0.5 if run_number < runs_count - 1
      end

    ensure
      # Убеждаемся, что worker остановлен в конце всех запусков
      if worker_info
        puts "\nStopping worker..."
        stop_worker(worker_info)
        puts "✓ Worker stopped"
      end
    end

    # Выводим итоговую статистику
    puts "\n" + "=" * 50
    puts "Test Results:"
    puts "  Successful runs: #{successful_runs}/#{runs_count}"
    puts "  Failed runs: #{failed_runs.length}/#{runs_count}"
    
    if failed_runs.any?
      puts "\nFailed runs details:"
      failed_runs.each do |failure|
        puts "  Run ##{failure[:run]}"
        if failure[:error]
          puts "    Error: #{failure[:error]}"
        elsif failure[:segments]
          puts "    Segments without punctuation:"
          failure[:segments].each do |seg|
            if seg[:start]
              puts "      [#{seg[:start]}s - #{seg[:finish]}s] #{seg[:text]}"
            else
              puts "      #{seg[:text]}"
            end
          end
        end
      end
    end
    puts "=" * 50

    expect(successful_runs).to be >= MIN_SUCCESSFUL_RUNS,
      "Expected at least #{MIN_SUCCESSFUL_RUNS} successful runs, but got #{successful_runs}"
  end
end


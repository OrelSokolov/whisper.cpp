# frozen_string_literal: true

require 'spec_helper'
require 'io/wait'

RSpec.describe 'Punctuation in transcription', type: :integration do
  TIMEOUT_SECONDS = 20
  RUNS_COUNT = 10
  MIN_SUCCESSFUL_RUNS = 9

  let(:spec_dir) { File.dirname(__FILE__) }
  let(:project_root) { File.expand_path('../..', spec_dir) }
  let(:whisper_cli) { File.join(project_root, 'build', 'bin', 'whisper-cli') }
  let(:audio_file) { File.expand_path('~/tube/babes.mp3') }
  let(:model) { File.join(project_root, 'models', 'ggml-large-v3-turbo.bin') }
  let(:timeout_seconds) { TIMEOUT_SECONDS }
  let(:runs_count) { RUNS_COUNT }
  let(:min_successful_runs) { MIN_SUCCESSFUL_RUNS }

  before(:all) do
    spec_dir = File.dirname(__FILE__)
    project_root = File.expand_path('../..', spec_dir)
    whisper_cli_path = File.join(project_root, 'build', 'bin', 'whisper-cli')
    audio_file_path = File.expand_path('~/tube/babes.mp3')
    model_path = File.join(project_root, 'models', 'ggml-large-v3-turbo.bin')

    unless File.exist?(whisper_cli_path)
      skip 'whisper-cli not built. Run: rake build:cli'
    end

    unless File.exist?(audio_file_path)
      skip 'Test audio file not found at ~/tube/babes.mp3'
    end

    unless File.exist?(model_path)
      skip 'Model file not found at models/ggml-large-v3-turbo.bin'
    end
  end

  def run_whisper_cli
    command = [
      whisper_cli,
      '-f', audio_file,
      '--model', model,
      '--language', 'ru'
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
          break if Time.now - start_time > timeout_seconds

          # Проверяем, завершился ли процесс
          break unless wait_thr.alive?

          # Пытаемся прочитать доступные данные (неблокирующее чтение)
          begin
            # Используем select для проверки доступности данных
            if IO.select([stdout_stderr], nil, nil, 0.1)
              # Читаем доступные данные
              chunk = stdout_stderr.read_nonblock(4096)
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
          Process.kill('TERM', pid)
          sleep 0.5
          if wait_thr.alive?
            Process.kill('KILL', pid)
          end
        end

        # Читаем оставшиеся данные
        begin
          remaining = stdout_stderr.read
          output += remaining if remaining
        rescue EOFError
          # Поток уже закрыт
        end

        wait_thr.value
      end
    rescue Timeout::Error => e
      error_output = "Timeout after #{timeout_seconds} seconds"
      if pid
        begin
          Process.kill('TERM', pid)
          sleep 0.5
          Process.kill('KILL', pid) if Process.waitpid(pid, Process::WNOHANG).nil?
        rescue Errno::ESRCH, Errno::ECHILD
          # Процесс уже завершился
        end
      end
    rescue => e
      error_output = e.message
    end

    { output: output, error: error_output }
  end

  def extract_timestamps(content)
    # Ищем строки вида [00:00:00.240 --> 00:00:04.060]   текст
    timestamps = []
    content.each_line do |line|
      # Ищем паттерн таймстампа в строке
      if line.match(/\[(\d{2}:\d{2}:\d{2}\.\d{3})\s+-->\s+(\d{2}:\d{2}:\d{2}\.\d{3})\]\s+(.+)$/)
        start = $1
        finish = $2
        text = $3
        timestamps << {
          start: start,
          finish: finish,
          text: text.strip
        }
      end
    end
    timestamps
  end

  def has_punctuation?(text)
    # Проверяем наличие запятых и точек
    text.match?(/[.,]/)
  end

  def check_first_three_timestamps(output)
    timestamps = extract_timestamps(output)
    return false if timestamps.empty?

    # Берем первые три таймстампа
    first_three = timestamps.first(3)
    return false if first_three.length < 3

    # Проверяем, что хотя бы в одном из первых трех есть пунктуация
    first_three.any? { |ts| has_punctuation?(ts[:text]) }
  end

  it "should produce punctuation in at least #{MIN_SUCCESSFUL_RUNS} out of #{RUNS_COUNT} runs" do
    successful_runs = 0
    failed_runs = []

    runs_count.times do |run_number|
      puts "\n[Run #{run_number + 1}/#{runs_count}] Running whisper-cli..."
      result = run_whisper_cli

      if result[:error] && !result[:error].empty?
        puts "  Error: #{result[:error]}"
        failed_runs << { run: run_number + 1, error: result[:error] }
        next
      end

      has_punctuation = check_first_three_timestamps(result[:output])

      if has_punctuation
        successful_runs += 1
        puts "  ✓ Punctuation found in first three timestamps"
      else
        puts "  ✗ No punctuation found in first three timestamps"
        # Показываем первые три таймстампа для отладки
        timestamps = extract_timestamps(result[:output])
        first_three = timestamps.first(3)
        first_three.each do |ts|
          puts "    [#{ts[:start]} --> #{ts[:finish]}] #{ts[:text]}"
        end
        failed_runs << { run: run_number + 1, timestamps: first_three }
      end
    end

    puts "\n" + "=" * 60
    puts "Results: #{successful_runs}/#{runs_count} runs had punctuation"
    puts "=" * 60

    if failed_runs.any?
      puts "\nFailed runs details:"
      failed_runs.each do |failure|
        puts "  Run #{failure[:run]}:"
        if failure[:error]
          puts "    Error: #{failure[:error]}"
        elsif failure[:timestamps]
          failure[:timestamps].each do |ts|
            puts "    [#{ts[:start]} --> #{ts[:finish]}] #{ts[:text]}"
          end
        end
      end
    end

    expect(successful_runs).to be >= min_successful_runs,
      "Expected at least #{min_successful_runs} runs with punctuation, but got #{successful_runs}"
  end
end


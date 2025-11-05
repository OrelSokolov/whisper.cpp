#!/usr/bin/env ruby
# frozen_string_literal: true
#
# Whisper Worker Ruby Client
# 
# Клиент для отправки аудио файлов на WebSocket сервер whisper-worker
# и получения результатов транскрипции в реальном времени.
#
# Использование:
#   bundle exec ruby client.rb [опции] <файл>
#   или
#   ruby client.rb [опции] <файл>
#
# Примеры:
#   bundle exec ruby client.rb audio.wav
#   ruby client.rb --host 192.168.1.100 --port 8765 audio.mp3
#
# Требования:
#   bundle install
#   или
#   gem install websocket-client-simple json

require 'json'
require 'optparse'

# Проверка наличия необходимой библиотеки
begin
  require 'websocket-client-simple'
rescue LoadError
  puts "Ошибка: требуется библиотека websocket-client-simple"
  puts "Установите её командой: gem install websocket-client-simple"
  exit 1
end

# Параметры командной строки
options = {
  host: 'localhost',
  port: 8765,
  file: nil,
  no_timestamps: false
}

OptionParser.new do |opts|
  opts.banner = "Использование: #{$0} [опции] <файл>"
  
  opts.on('--host HOST', 'Хост сервера (по умолчанию: localhost)') do |host|
    options[:host] = host
  end
  
  opts.on('--port PORT', Integer, 'Порт сервера (по умолчанию: 8765)') do |port|
    options[:port] = port
  end
  
  opts.on('--no-timestamps', 'Вывод без таймстампов (только текст)') do
    options[:no_timestamps] = true
  end
  
  opts.on('-h', '--help', 'Показать эту справку') do
    puts opts
    exit
  end
end.parse!

# Получить имя файла из аргументов
if ARGV.empty?
  puts "Ошибка: не указан файл для отправки"
  puts "Использование: #{$0} [опции] <файл>"
  exit 1
end

options[:file] = ARGV[0]

# Проверка существования файла
unless File.exist?(options[:file])
  puts "Ошибка: файл '#{options[:file]}' не найден"
  exit 1
end

# URL WebSocket сервера
ws_url = "ws://#{options[:host]}:#{options[:port]}"

puts "Подключение к #{ws_url}..."
puts "Отправка файла: #{options[:file]}"
puts "Размер файла: #{File.size(options[:file])} байт"
puts "-" * 50

begin
  # Thread-safe флаги для отслеживания состояния
  connection_open = true
  processing_complete = false
  mutex = Mutex.new
  
  # Переменные для измерения RTT
  start_time = nil
  end_time = nil
  
  # Подключение к WebSocket серверу
  ws = WebSocket::Client::Simple.connect(ws_url)
  
  # Обработчик открытия соединения
  ws.on :open do
    puts "✓ Соединение установлено"
    puts "Отправка аудио файла..."
    
    # Читаем файл
    audio_data = File.binread(options[:file])
    
    # Фиксируем время начала отправки файла для RTT (перед фактической отправкой)
    mutex.synchronize { start_time = Time.now }
    
    # websocket-client-simple может не поддерживать прямой binary send
    # Попробуем отправить через send с опцией binary
    begin
      # Попытка отправки как binary
      if ws.respond_to?(:send_binary)
        ws.send_binary(audio_data)
      else
        # Fallback: отправка как обычные данные
        audio_data.force_encoding('BINARY')
        ws.send(audio_data)
      end
    rescue => e
      puts "Предупреждение при отправке: #{e.message}"
      # Попробуем обычную отправку
      audio_data.force_encoding('BINARY')
      ws.send(audio_data)
    end
    
    puts "✓ Файл отправлен (#{audio_data.bytesize} байт)"
    puts "Ожидание результатов транскрипции..."
    puts "-" * 50
    
    # Небольшая задержка перед отправкой команды
    sleep 0.2
    
    # Отправляем команду для начала обработки
    ws.send('process')
  end
  
  # Обработчик получения сообщений
  ws.on :message do |event|
    data = event.data
    
    # Пытаемся парсить JSON
    begin
      json = JSON.parse(data)
      
      case json['type']
      when 'segment'
        # Выводим сегмент транскрипции
        if options[:no_timestamps]
          # Только текст без таймстампов и метаинформации
          puts json['text']
        else
          # Полный вывод с таймстампами и прогрессом
          if json['start'] && json['end']
            progress_str = json['progress'] ? " [#{format('%.1f', json['progress'])}%]" : ''
            eta_str = json['eta'] ? " ETA: #{format('%d', json['eta'])}s" : ''
            puts "[#{format('%.2f', json['start'])}s - #{format('%.2f', json['end'])}s]#{progress_str}#{eta_str} #{json['text']}"
          else
            puts json['text']
          end
          
          # Если есть информация о спикере
          if json['speaker']
            puts "  (спикер: #{json['speaker']})"
          end
        end
        
      when 'status'
        puts "Статус: #{json['message']}"
        
      when 'complete'
        # Фиксируем время завершения для RTT
        mutex.synchronize do
          end_time = Time.now
          processing_complete = true
        end
        
        # Вычисляем и выводим RTT
        rtt_seconds = mutex.synchronize do
          if start_time && end_time
            (end_time - start_time).round(3)
          else
            nil
          end
        end
        
        puts "-" * 50
        puts "✓ Обработка завершена"
        if rtt_seconds
          puts "RTT: #{rtt_seconds} секунд (от начала отправки файла до завершения расшифровки)"
        end
        # Не закрываем сразу, дадим серверу закрыть соединение
        
      when 'error'
        puts "✗ Ошибка: #{json['message']}"
        mutex.synchronize { processing_complete = true }
        # Не закрываем сразу, дадим серверу закрыть соединение
        
      else
        puts "Неизвестный тип сообщения: #{json['type']}"
      end
      
    rescue JSON::ParserError => e
      # Если не JSON, выводим как есть
      puts data unless data.strip.empty?
    end
  end
  
  # Обработчик закрытия соединения
  ws.on :close do |event|
    mutex.synchronize { connection_open = false }
    is_complete = mutex.synchronize { processing_complete }
    unless is_complete
      puts "-" * 50
      if event
        puts "⚠ Соединение закрыто сервером (код: #{event.code}, причина: #{event.reason})"
      else
        puts "⚠ Соединение закрыто"
      end
    end
  end
  
  # Обработчик ошибок
  ws.on :error do |error|
    # Игнорируем ошибку "stream closed" если обработка уже завершена
    is_complete = mutex.synchronize { processing_complete }
    unless is_complete && error.message.include?("stream closed")
      puts "✗ Ошибка WebSocket: #{error.message}"
    end
    mutex.synchronize { connection_open = false }
  end
  
  # Ждем завершения обработки или закрытия соединения
  loop do
    sleep 0.1
    is_complete = mutex.synchronize { processing_complete }
    is_open = mutex.synchronize { connection_open }
    break if is_complete || !is_open
  end
  
  # Если обработка завершена, даем время серверу закрыть соединение
  is_complete = mutex.synchronize { processing_complete }
  is_open = mutex.synchronize { connection_open }
  if is_complete && is_open
    sleep 0.5
    ws.close if mutex.synchronize { connection_open }
  end
  
rescue Interrupt
  puts "\n\nПрервано пользователем"
  ws.close if ws
  exit 1
rescue Errno::ECONNREFUSED
  puts "✗ Ошибка: не удалось подключиться к серверу #{ws_url}"
  puts "Убедитесь, что сервер whisper-worker запущен"
  exit 1
rescue => e
  puts "✗ Ошибка: #{e.class}: #{e.message}"
  puts e.backtrace.first(5).join("\n")
  exit 1
end


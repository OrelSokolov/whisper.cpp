#!/usr/bin/env python3
"""
Тестовый клиент для whisper.cpp streaming API
Использование: python3 test_streaming_client.py <audio_file>
"""

import requests
import json
import sys
import time

def stream_transcription(audio_file_path, server_url="http://127.0.0.1:8080"):
    """
    Отправляет аудио файл на сервер и получает streaming результаты
    """
    endpoint = f"{server_url}/inference-stream"
    
    print(f"📤 Uploading: {audio_file_path}")
    print(f"🌐 Server: {endpoint}")
    print("=" * 70)
    
    # Засекаем время начала
    start_time = time.time()
    first_segment_time = None
    
    try:
        # Открываем файл и отправляем
        with open(audio_file_path, 'rb') as audio_file:
            files = {'file': (audio_file_path, audio_file, 'audio/wav')}
            data = {
                'temperature': '0.0',
                'temperature_inc': '0.2',
                'language': 'auto'
            }
            
            # Stream=True для получения SSE событий в реальном времени
            response = requests.post(
                endpoint, 
                files=files, 
                data=data, 
                stream=True,
                timeout=300
            )
            
            if response.status_code != 200:
                print(f"❌ Error: Server returned {response.status_code}")
                print(response.text)
                return
            
            # Обрабатываем SSE поток
            print("🎧 Receiving streaming results...\n")
            
            for line in response.iter_lines():
                if line:
                    line = line.decode('utf-8')
                    
                    # SSE формат: "data: {...}"
                    if line.startswith('data: '):
                        json_str = line[6:]  # убираем "data: "
                        try:
                            event = json.loads(json_str)
                            
                            # Засекаем время первого сегмента
                            if first_segment_time is None and event.get('type') == 'segment':
                                first_segment_time = time.time()
                            
                            handle_event(event)
                        except json.JSONDecodeError as e:
                            print(f"⚠️  Failed to parse: {e}")
                            print(f"   Raw: {line}")
            
            # Подсчитываем общее время
            end_time = time.time()
            total_time = end_time - start_time
            
            print(f"\n⏱️  Общее время запроса: {total_time:.2f}s")
            if first_segment_time:
                ttfs = first_segment_time - start_time  # Time To First Segment
                print(f"⚡ Время до первого сегмента: {ttfs:.2f}s")
    
    except FileNotFoundError:
        print(f"❌ Error: File not found: {audio_file_path}")
    except requests.exceptions.ConnectionError:
        print(f"❌ Error: Could not connect to server at {server_url}")
        print("   Make sure whisper-server is running!")
    except KeyboardInterrupt:
        print("\n\n⚠️  Interrupted by user")
    except Exception as e:
        print(f"❌ Error: {e}")

def handle_event(event):
    """Обрабатывает SSE события от сервера"""
    event_type = event.get('type')
    
    if event_type == 'start':
        print(f"🎬 Started processing: {event.get('filename', 'unknown')}\n")
    
    elif event_type == 'segment':
        index = event.get('index', '?')
        text = event.get('text', '')
        
        if 'start' in event and 'end' in event:
            start = event['start']
            end = event['end']
            print(f"[{start:6.2f}s → {end:6.2f}s]  {text}")
        else:
            print(f"Segment {index}: {text}")
    
    elif event_type == 'done':
        print("\n" + "=" * 70)
        print("✅ Transcription completed!")
    
    elif event_type == 'error':
        print(f"\n❌ Server error: {event.get('message', 'unknown error')}")

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python3 test_streaming_client.py <audio_file>")
        print("\nExample:")
        print("  python3 test_streaming_client.py samples/jfk.wav")
        sys.exit(1)
    
    audio_file = sys.argv[1]
    
    # Опционально можно указать URL сервера
    server = sys.argv[2] if len(sys.argv) > 2 else "http://127.0.0.1:8080"
    
    stream_transcription(audio_file, server)


# DEB пакет для whisper.cpp

Этот репозиторий содержит конфигурацию для создания DEB пакета whisper.cpp для архитектуры amd64.

## Быстрый старт

### Установка зависимостей

```bash
sudo apt-get update
sudo apt-get install -y devscripts debhelper build-essential cmake \
    pkg-config libssl-dev libvulkan-dev glslang-tools rustc cargo git patchelf
```

### Сборка пакета

```bash
./build-deb.sh
```

Результирующий `.deb` файл будет находиться в родительской директории.

### Установка пакета

```bash
sudo dpkg -i ../whisper-cpp_*.deb
sudo apt-get install -f  # Если нужно установить зависимости
```

## Содержимое пакета

Пакет включает все основные бинарники whisper.cpp:

- **whisper-cli** - командная строка для транскрипции
- **whisper-server** - HTTP сервер
- **whisper-worker** - WebSocket worker (C++ версия)
- **whisper-worker-rs** - WebSocket worker (Rust версия)
- **whisper-bench** - инструмент для бенчмарков
- **whisper-stream** - стриминг инструмент
- **whisper-command** - инструмент команд
- **whisper-system-info** - информация о системе
- **whisper-vad-speech-segments** - VAD сегментация
- **quantize** - квантование моделей

А также библиотеки:
- **libwhisper.so** - основная библиотека
- **libggml.so** - библиотека ggml
- Заголовочные файлы для разработки

## Особенности

- **Vulkan GPU поддержка включена по умолчанию** - пакет собирается с поддержкой GPU ускорения через Vulkan
- Автоматический fallback на CPU если Vulkan недоступен
- Требуется `libvulkan1` для работы (устанавливается автоматически как зависимость)

## Использование после установки

```bash
# Транскрипция аудио (будет использовать GPU если доступен Vulkan)
whisper-cli -m models/ggml-base.bin audio.wav

# Запуск HTTP сервера
whisper-server --port 8080

# Запуск Rust WebSocket worker
whisper-worker-rs -m models/ggml-base.bin
```

## Подробная документация

Полную документацию по сборке смотрите в файле `debian/README.DEB.md`.


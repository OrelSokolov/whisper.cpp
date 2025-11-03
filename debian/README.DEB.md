# Сборка DEB пакета для whisper.cpp

Этот пакет упаковывает whisper.cpp со всеми бинарниками, включая Rust версию worker, для архитектуры amd64.

## Требования

Для сборки пакета вам понадобятся следующие пакеты:

```bash
sudo apt-get update
sudo apt-get install -y \
    devscripts \
    debhelper \
    build-essential \
    cmake \
    pkg-config \
    libssl-dev \
    libvulkan-dev \
    glslang-tools \
    rustc \
    cargo \
    git \
    patchelf
```

## Сборка

Для сборки пакета выполните:

```bash
./build-deb.sh
```

Или вручную:

```bash
debuild -b -us -uc
```

Результирующий `.deb` файл будет находиться в родительской директории.

## Установка

После сборки установите пакет:

```bash
sudo dpkg -i ../whisper-cpp_*.deb
```

Если есть проблемы с зависимостями:

```bash
sudo apt-get install -f
```

## Содержимое пакета

Пакет включает:

### Библиотеки
- `libwhisper.so` - основная библиотека whisper.cpp
- `libggml.so` - библиотека ggml

### Бинарники
- `whisper-cli` - командная строка для транскрипции аудио
- `whisper-server` - HTTP сервер
- `whisper-worker` - WebSocket worker (C++ версия)
- `whisper-worker-rs` - WebSocket worker (Rust версия)
- `whisper-bench` - инструмент для бенчмарков
- `whisper-stream` - инструмент для стриминга
- `whisper-command` - инструмент команд
- `whisper-system-info` - информация о системе
- `whisper-vad-speech-segments` - сегментация речи с VAD
- `quantize` - инструмент для квантования моделей

### Заголовочные файлы
- `/usr/include/whisper.h` - заголовочный файл для разработки

### CMake файлы
- `/usr/lib/x86_64-linux-gnu/cmake/whisper/` - CMake конфигурация

### pkg-config файлы
- `/usr/lib/x86_64-linux-gnu/pkgconfig/whisper.pc`

## Использование

После установки вы можете использовать бинарники напрямую:

```bash
whisper-cli -m /path/to/model.bin audio.wav
whisper-server --port 8080
whisper-worker-rs -m /path/to/model.bin
```

## Особенности сборки

- **Vulkan поддержка включена по умолчанию** - пакет собирается с поддержкой GPU ускорения через Vulkan
- Rust версия worker (`whisper-worker-rs`) требует, чтобы библиотеки были доступны через стандартные пути или LD_LIBRARY_PATH
- Все бинарники будут доступны в `/usr/bin`
- Библиотеки будут установлены в `/usr/lib/x86_64-linux-gnu/`

## Зависимости

Пакет требует наличия Vulkan runtime для работы (`libvulkan1`). Если у вас нет GPU с поддержкой Vulkan, 
библиотека автоматически откатится на CPU режим, поэтому пакет будет работать на любой системе.


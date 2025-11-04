# Dockerfile для worker-rs с использованием DEB пакета
# Используем Ubuntu 24.04 для совместимости с пакетом, собранным на Ubuntu 25.04
FROM ubuntu:24.04

# Установка базовых зависимостей и dpkg для установки DEB пакета
RUN apt-get update && apt-get install -y \
    ca-certificates \
    dpkg \
    && rm -rf /var/lib/apt/lists/*

# Копирование DEB пакета
# DEB пакет создается командой 'rake build:deb' и сохраняется в ../whisper-cpp_*.deb
# Перед сборкой Docker образа нужно скопировать DEB пакет в корень проекта:
#   cp ../whisper-cpp_*.deb ./
COPY whisper-cpp_*_amd64.deb /tmp/whisper-cpp.deb

# Установка DEB пакета
RUN dpkg -i /tmp/whisper-cpp.deb || (apt-get update && apt-get install -f -y) && \
    # Проверка установки
    echo "=== Проверка установленных файлов ===" && \
    ls -la /usr/bin/whisper* || echo "Бинарники не найдены" && \
    dpkg -L whisper-cpp | grep whisper-worker-rs || echo "whisper-worker-rs не в пакете" && \
    rm -f /tmp/whisper-cpp.deb && \
    rm -rf /var/lib/apt/lists/*

# Установка runtime зависимостей для FFmpeg (если нужны)
RUN apt-get update && apt-get install -y \
    libssl3 \
    && (apt-get install -y libavcodec58 libavformat58 libavutil56 libswresample3 2>/dev/null || \
        apt-get install -y libavcodec59 libavformat59 libavutil57 libswresample4 2>/dev/null || \
        apt-get install -y libavcodec libavformat libavutil libswresample 2>/dev/null || true) \
    && rm -rf /var/lib/apt/lists/*

# Установка Vulkan runtime из стандартных репозиториев Ubuntu
RUN apt-get update && apt-get install -y \
    libvulkan1 \
    vulkan-tools \
    mesa-vulkan-drivers \
    && rm -rf /var/lib/apt/lists/*

# Создание группы render для доступа к GPU
# GID может быть передан через ARG RENDER_GID (по умолчанию 106 - стандартный для Ubuntu)
# Если на хосте другой GID, установите его через docker-compose или build arg
ARG RENDER_GID=106
RUN groupadd -g ${RENDER_GID} render 2>/dev/null || \
    (groupadd -f render && usermod -a -G render root) || true

# Копирование папки с моделями из корня проекта
COPY models /app/models

WORKDIR /app

# Переменные окружения для Vulkan
ENV VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/*.json
ENV VK_LAYER_PATH=/usr/share/vulkan/explicit_layer.d

# Обновление библиотек
RUN ldconfig

# Порт для WebSocket
EXPOSE 8765

# Запуск worker-rs (бинарник уже установлен из DEB пакета в /usr/bin/)
# Используем large-v3-turbo для лучшего качества и скорости
CMD ["/usr/bin/whisper-worker-rs", "-m", "models/ggml-large-v3-turbo.bin"]

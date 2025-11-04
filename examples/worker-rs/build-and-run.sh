#!/bin/bash
# Скрипт для сборки и запуска worker-rs в Docker

set -e

cd "$(dirname "$0")/../.."

echo "=== Проверка и копирование DEB пакета ==="
if [ -f ../whisper-cpp_*.deb ]; then
    echo "Копирование DEB пакета в корень проекта..."
    cp -v ../whisper-cpp_*.deb ./
else
    echo "DEB пакет не найден. Создание пакета..."
    rake build:deb
    cp -v ../whisper-cpp_*.deb ./
fi

echo ""
echo "=== Создание .env с GID группы render ==="
cd examples/worker-rs
RENDER_GID=$(getent group render | cut -d: -f3 || echo "991")
echo "RENDER_GID=$RENDER_GID" > .env
echo "Установлен RENDER_GID=$RENDER_GID"

echo ""
echo "=== Сборка Docker образа ==="
docker compose build

echo ""
echo "=== Запуск контейнера ==="
docker compose up


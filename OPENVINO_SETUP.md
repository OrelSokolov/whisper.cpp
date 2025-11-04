# Инструкция по установке OpenVINO в Ubuntu и настройке проекта whisper.cpp

## ⚠️ Важно: Совместимость с AMD

**OpenVINO на AMD системах:**
- ✅ **CPU (AMD Ryzen/EPYC)**: Работает, но производительность может быть ниже, чем на Intel CPU (OpenVINO оптимизирован для Intel)
- ❌ **GPU (AMD Radeon)**: НЕ поддерживается - OpenVINO GPU ускорение работает только на Intel GPU
- ✅ **Рекомендация для AMD**: Используйте устройство `CPU` в OpenVINO (флаг `-oved CPU`)

**Альтернатива для AMD GPU:**
Для использования AMD Radeon GPU в whisper.cpp рекомендуется использовать **Vulkan** (кроссплатформенное решение):
```bash
cmake -B build -DGGML_VULKAN=1
cmake --build build -j$(nproc)
```
Vulkan работает на AMD, Intel и NVIDIA GPU и обычно даёт лучшую производительность на AMD GPU, чем OpenVINO на CPU.

---

## Вариант 1: Установка через официальный пакет Intel (рекомендуется для CMake)

### Шаг 1: Установка OpenVINO Runtime

```bash
# Добавьте ключи репозитория Intel
wget -qO- https://apt.repos.intel.com/intel-gpg-keys/GPG-PUB-KEY-INTEL-SW-PRODUCTS.PUB | sudo gpg --dearmor -o /usr/share/keyrings/intel-gpg-archive-keyring.gpg

# Добавьте репозиторий OpenVINO (для Ubuntu 22.04, который совместим с Ubuntu 25.04)
echo "deb [signed-by=/usr/share/keyrings/intel-gpg-archive-keyring.gpg] https://apt.repos.intel.com/openvino/2025 ubuntu22 main" | sudo tee /etc/apt/sources.list.d/intel-openvino-2025.list

# Обновите список пакетов
sudo apt update

# Установите OpenVINO Runtime (для C++ разработки)
sudo apt install -y openvino

# Или установите полный пакет разработки (включая Python)
sudo apt install -y openvino-dev
```

### Шаг 2: Настройка переменных окружения

```bash
# Загрузите переменные окружения OpenVINO
source /opt/intel/openvino/setupvars.sh

# Для постоянной настройки добавьте в ~/.bashrc:
echo 'source /opt/intel/openvino/setupvars.sh' >> ~/.bashrc
```

### Шаг 3: Установка дополнительных зависимостей

```bash
sudo apt install -y libopencv-dev python3-opencv python3-pip python3-virtualenv libgfortran5
```

---

## Вариант 2: Установка через pip (проще, но требует настройки CMake)

Если вариант 1 не работает или вы предпочитаете pip:

```bash
# Создайте виртуальное окружение (опционально, но рекомендуется)
python3 -m venv ~/openvino_env
source ~/openvino_env/bin/activate

# Установите OpenVINO
pip install --upgrade pip
pip install openvino openvino-dev
```

**Важно:** При установке через pip нужно настроить CMake для поиска OpenVINO. См. раздел "Настройка CMake" ниже.

---

## Шаг 4: Проверка установки

```bash
# Проверка через Python
python3 -c "import openvino.runtime as ov; core = ov.Core(); print('Доступные устройства:', core.get_available_devices())"

# Проверка наличия библиотек
ls -la /opt/intel/openvino/lib/ 2>/dev/null || echo "OpenVINO не найден в /opt/intel/openvino/"
```

---

## Шаг 5: Настройка проекта whisper.cpp

### 5.1. Подготовка моделей OpenVINO

```bash
cd models

# Создайте виртуальное окружение для конвертации моделей
python3 -m venv openvino_conv_env
source openvino_conv_env/bin/activate

# Установите зависимости
python -m pip install --upgrade pip
pip install -r requirements-openvino.txt

# Сгенерируйте OpenVINO модель (например, для base.en)
python convert-whisper-to-openvino.py --model base.en

# Деактивируйте окружение
deactivate
```

Это создаст файлы:
- `ggml-base.en-encoder-openvino.xml`
- `ggml-base.en-encoder-openvino.bin`

Переместите их в папку с вашими моделями whisper.

### 5.2. Сборка проекта с поддержкой OpenVINO

```bash
cd /home/oleg/whisper.cpp

# Убедитесь, что переменные окружения OpenVINO загружены
source /opt/intel/openvino/setupvars.sh  # для варианта 1
# или
source ~/openvino_env/bin/activate  # для варианта 2

# Пересоберите проект с поддержкой OpenVINO
cmake -B build -DWHISPER_OPENVINO=ON
cmake --build build -j$(nproc) --config Release
```

### 5.3. Если CMake не находит OpenVINO

Если при сборке возникает ошибка `find_package(OpenVINO)`, выполните:

```bash
# Для варианта 1 (официальный пакет)
export OpenVINO_DIR=/opt/intel/openvino/cmake
cmake -B build -DWHISPER_OPENVINO=ON -DOpenVINO_DIR=$OpenVINO_DIR

# Для варианта 2 (pip)
# Найдите путь к установке OpenVINO
python3 -c "import openvino; import os; print(os.path.dirname(openvino.__file__))"
# Затем установите переменную окружения (замените путь на фактический)
export OpenVINO_DIR=$(python3 -c "import openvino; import os; print(os.path.dirname(openvino.__file__))")/cmake
cmake -B build -DWHISPER_OPENVINO=ON -DOpenVINO_DIR=$OpenVINO_DIR
```

---

## Шаг 6: Проверка работы

```bash
# Запустите whisper-cli с моделью, которая имеет OpenVINO файлы
./build/bin/whisper-cli -m models/ggml-base.en.bin -f samples/jfk.wav -oved CPU

# Проверьте информацию о системе
./build/examples/system-info/system-info 2>&1 | grep OPENVINO
# Должно показать: OPENVINO = Yes
```

---

## Устранение проблем

### Проблема: CMake не находит OpenVINO

**Решение:**
```bash
# Установите переменную окружения OpenVINO_DIR перед запуском cmake
export OpenVINO_DIR=/opt/intel/openvino/cmake
cmake -B build -DWHISPER_OPENVINO=ON
```

### Проблема: OpenVINO не видит устройства

**Решение:**
```bash
# Проверьте доступные устройства
python3 -c "import openvino.runtime as ov; core = ov.Core(); print(core.get_available_devices())"

# Убедитесь, что используете правильное устройство (CPU, GPU, AUTO)
# В whisper.cpp используйте флаг -oved для выбора устройства:
./build/bin/whisper-cli -m model.bin -f audio.wav -oved CPU
```

### Проблема: Ошибки при первой загрузке модели

**Нормально:** Первый запуск на устройстве может занять время, так как OpenVINO компилирует модель в формат, специфичный для устройства. Последующие запуски будут быстрее благодаря кэшированию.

---

## Дополнительная информация

- Официальная документация OpenVINO: https://docs.openvino.ai/
- Рекомендуемая версия OpenVINO для whisper.cpp: 2024.6.0 или новее
- Поддерживаемые устройства: 
  - **CPU**: x86_64 (Intel и AMD) - работает на обоих, но оптимизирован для Intel
  - **GPU**: Только Intel GPU (Intel Arc, встроенные Intel GPU)
  - **AUTO**: Автоматический выбор устройства
- **Для AMD систем**: 
  - Используйте `-oved CPU` для OpenVINO
  - Для GPU ускорения используйте Vulkan (`-DGGML_VULKAN=1`) вместо OpenVINO
- Производительность OpenVINO на AMD CPU может быть на 10-30% ниже, чем на Intel CPU из-за оптимизаций под Intel архитектуру


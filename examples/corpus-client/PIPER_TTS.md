# Создание датасета для Piper TTS

Это руководство показывает, как создать датасет для обучения Piper TTS.

## Требования Piper TTS

Piper TTS требует:
- **Формат**: WAV (PCM 16-bit)
- **Частота дискретизации**: 22050 Hz
- **Каналы**: Mono (1 канал)
- **Длина сегментов**: 0.5-30 секунд (оптимально 2-10 сек)

## Быстрый старт

### 1. Запустите Whisper сервер

```bash
cd examples/worker-rs
./target/release/whisper-worker-rs \
    --model ../../models/ggml-large-v3.bin \
    --language ru \
    --merge-timestamps
```

**Важно**: Флаг `--merge-timestamps` объединяет разбитые предложения в целые.

### 2. Создайте датасет (Piper режим по умолчанию)

```bash
cd examples/corpus-client

# Из YouTube
./target/release/corpus-client \
    --youtube-url "https://www.youtube.com/watch?v=..." \
    --output-dir ./piper_dataset

# Или из локального файла
./target/release/corpus-client \
    --audio-file audiobook.mp3 \
    --output-dir ./piper_dataset
```

**Piper TTS режим включён по умолчанию!**

Автоматические настройки:
- Формат: WAV 22050Hz mono
- Длина сегментов: 0.5-30 секунд
- Оптимальные параметры для Piper
- **Vowel hotfix**: автоматическое добавление 0.15s к сегментам, заканчивающимся на гласную (исправляет обрезку Whisper)
- Генерация `metadata.csv` в формате Piper

Для отключения оптимизаций Piper: добавьте `--no-piper`

## Результат

```
piper_dataset/
├── metadata.json          # JSON метаданные
├── metadata.csv           # Piper TTS формат
└── wavs/
    ├── 000001.wav        # 22050Hz, mono, 16-bit PCM
    ├── 000001.txt        # "Добро пожаловать в этот подкаст."
    ├── 000002.wav
    ├── 000002.txt
    └── ...
```

## Ручная настройка

Все параметры настраиваются при необходимости:

```bash
./target/release/corpus-client \
    --audio-file audio.mp3 \
    --output-dir ./custom_dataset \
    --format flac \
    --sample-rate 44100 \
    --min-duration 1.0 \
    --max-duration 10.0 \
    --no-piper  # Отключить моно и vowel hotfix
```

Примечание: Режим Piper включён по умолчанию (моно + vowel hotfix). Используйте `--no-piper` для отключения.

## Подготовка к обучению Piper

После создания датасета:

1. **Проверьте качество**:
   ```bash
   # Посмотрите количество сегментов
   ls piper_dataset/segments/*.wav | wc -l
   
   # Проверьте метаданные
   cat piper_dataset/metadata.json | jq
   
   # Прослушайте несколько случайных файлов
   mpv piper_dataset/segments/000042.wav
   cat piper_dataset/segments/000042.txt
   ```

2. **Проверьте metadata.csv**:
   
   Файл `metadata.csv` генерируется автоматически в формате Piper:
   ```bash
   head piper_dataset/metadata.csv
   ```
   
   Формат:
   ```
   000001.wav|Добро пожаловать в этот подкаст.
   000002.wav|Сегодня мы поговорим о машинном обучении.
   ```
   
   Этот файл готов для использования с Piper TTS!

3. **Запустите обучение Piper**:
   ```bash
   # Следуйте инструкциям Piper TTS
   # https://github.com/rhasspy/piper
   ```

## Советы по качеству

### Выбор источника

✅ **Хорошо**:
- Аудиокниги (один голос, чёткая речь)
- Подкасты (качественная запись)
- Лекции (профессиональный спикер)

❌ **Плохо**:
- Музыкальные видео (фоновая музыка)
- Интервью с перебиванием
- Видео с плохим качеством звука

### Фильтрация

Датасет автоматически фильтрует:
- Слишком короткие сегменты (< 1 сек)
- Слишком длинные сегменты (> 10 сек)
- Сегменты с музыкой `[♪]`
- Сегменты со звуковыми эффектами

Проверьте `piper_dataset/rejected/` для анализа отклонённых сегментов.

## Примеры

### Русский подкаст

```bash
./corpus-client \
    --youtube-url "https://www.youtube.com/watch?v=..." \
    --output-dir ./ru_podcast
```

### Английская аудиокнига

```bash
# Сервер
./whisper-worker-rs \
    --model ../../models/ggml-large-v3-turbo.bin \
    --language en \
    --merge-timestamps

# Клиент
./corpus-client \
    --audio-file audiobook.m4a \
    --output-dir ./en_audiobook
```

### Несколько источников → один датасет

```bash
# Создайте отдельные датасеты
./corpus-client --audio-file file1.mp3 --output-dir ./temp1
./corpus-client --audio-file file2.mp3 --output-dir ./temp2

# Объедините
mkdir -p combined_dataset/wavs
cp temp1/wavs/* combined_dataset/wavs/
cp temp2/wavs/* combined_dataset/wavs/

# Переименуйте с правильной нумерацией
cd combined_dataset/wavs
counter=1
for wav in *.wav; do
    txt="${wav%.wav}.txt"
    new_wav=$(printf "%06d.wav" $counter)
    new_txt=$(printf "%06d.txt" $counter)
    mv "$wav" "$new_wav"
    mv "$txt" "$new_txt"
    counter=$((counter + 1))
done

# Создайте metadata.csv
cd ..
for txt in wavs/*.txt; do
    wav="${txt%.txt}.wav"
    wav_name=$(basename "$wav")
    text=$(cat "$txt")
    echo "$wav_name|$text"
done > metadata.csv
```

## Размер датасета

Для обучения качественной модели Piper рекомендуется:
- **Минимум**: 30 минут чистой речи (~300-400 сегментов)
- **Оптимально**: 2-5 часов (~1500-3000 сегментов)
- **Идеально**: 10+ часов (5000+ сегментов)

Проверьте размер:
```bash
cat piper_dataset/metadata.json | jq '.total_dataset_duration'
# Результат в секундах, разделите на 3600 для часов
```

## Troubleshooting

### "Sample rate is wrong"

Проверьте файлы:
```bash
ffprobe piper_dataset/wavs/000001.wav 2>&1 | grep "Audio"
# Должно быть: 22050 Hz, mono, s16
```

### "Files are stereo, not mono"

Режим Piper включён по умолчанию (mono автоматически). Если файлы стерео, проверьте что не использовали `--no-piper`.

### "Segments too long/short"

Настройте вручную:
```bash
--min-duration 2.0 --max-duration 8.0
```

## См. также

- [Piper TTS](https://github.com/rhasspy/piper)
- [CORPUS.md](../../CORPUS.md) - Полная документация
- [CORPUS_QUICKSTART.md](../../CORPUS_QUICKSTART.md) - Быстрый старт


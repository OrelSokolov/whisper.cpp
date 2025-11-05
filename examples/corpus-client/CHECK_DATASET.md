# Проверка готового датасета

Быстрое руководство по проверке качества созданного датасета для Piper TTS.

## Структура датасета

После выполнения corpus-client вы должны увидеть:

```bash
ls -la dataset/

# Результат:
drwxr-xr-x  wavs/           # Аудио файлы
drwxr-xr-x  rejected/       # Отфильтрованные
-rw-r--r--  metadata.json   # JSON метаданные
-rw-r--r--  metadata.csv    # Piper TTS формат
```

## Быстрая проверка

### 1. Количество сегментов

```bash
# Подсчет WAV файлов
ls dataset/wavs/*.wav | wc -l

# Подсчет строк в metadata.csv
wc -l dataset/metadata.csv

# Должны совпадать!
```

### 2. Формат metadata.csv

```bash
# Показать первые 5 строк
head -5 dataset/metadata.csv

# Ожидаемый формат:
# 000001.wav|Первая фраза текста.
# 000002.wav|Вторая фраза текста.
```

**Проверьте:**
- ✅ Формат: `filename|text`
- ✅ Разделитель: `|` (вертикальная черта)
- ✅ Нет заголовков (CSV начинается сразу с данных)
- ✅ Сортировка по имени файла

### 3. Формат аудио

```bash
# Проверить параметры WAV файла
ffprobe dataset/wavs/000001.wav 2>&1 | grep "Audio"

# Ожидаемый результат:
# Stream #0:0: Audio: pcm_s16le ([1][0][0][0] / 0x0001), 22050 Hz, mono, s16, 352 kb/s
```

**Проверьте:**
- ✅ Кодек: `pcm_s16le` (16-bit PCM)
- ✅ Sample rate: `22050 Hz`
- ✅ Каналы: `mono` (1 канал)

### 4. Длительность сегментов

```bash
# Проверить длительность нескольких файлов
for wav in dataset/wavs/00000{1..5}.wav; do
    duration=$(ffprobe -v error -show_entries format=duration -of default=noprint_wrappers=1:nokey=1 "$wav")
    echo "$wav: ${duration}s"
done

# Все должны быть в диапазоне 0.5-30 секунд
```

### 5. Метаданные JSON

```bash
# Посмотреть статистику
cat dataset/metadata.json | jq

# Ключевые поля:
# - total_segments: общее количество
# - accepted_segments: сколько сохранено
# - rejected_segments: сколько отфильтровано
# - avg_duration: средняя длительность
# - total_dataset_duration: общая длительность
```

## Детальная проверка

### Проверка качества текста

```bash
# Поиск коротких текстов (потенциальные проблемы)
for txt in dataset/wavs/*.txt; do
    len=$(cat "$txt" | wc -c)
    if [ $len -lt 10 ]; then
        echo "Short text: $txt ($len chars)"
        cat "$txt"
    fi
done
```

### Проверка наличия музыки/шума

```bash
# Поиск маркеров музыки
grep -r "♪" dataset/wavs/*.txt
grep -r "\[" dataset/wavs/*.txt

# Не должно быть результатов!
```

### Проверка vowel hotfix

```bash
# С включенным --verbose можно увидеть в логах:
./corpus-client --audio-file audio.mp3 --verbose 2>&1 | grep "vowel"

# Результат:
# [DEBUG] Segment 42: 'Как дела' ends with vowel 'а', adjusting end time 2.5 -> 2.65 (+0.15s)
```

### Проверка rejected сегментов

```bash
# Посмотреть причины отклонения
# (если логи сохранены)
grep "rejected" logs.txt

# Или посмотреть rejected директорию
ls -la dataset/rejected/
```

## Случайная выборка

### Прослушать случайные сегменты

```bash
# 10 случайных файлов
ls dataset/wavs/*.wav | shuf | head -10 | while read wav; do
    txt="${wav%.wav}.txt"
    echo "=== $wav ==="
    cat "$txt"
    mpv "$wav"
    echo ""
done
```

### Проверить соответствие аудио и текста

```bash
# Открыть случайный сегмент
RANDOM_NUM=$(printf "%06d" $((RANDOM % 100 + 1)))
echo "Text:"
cat "dataset/wavs/${RANDOM_NUM}.txt"
echo ""
echo "Playing audio:"
mpv "dataset/wavs/${RANDOM_NUM}.wav"
```

## Статистика датасета

### Распределение длительности

```bash
# Создать гистограмму длительности
for wav in dataset/wavs/*.wav; do
    ffprobe -v error -show_entries format=duration \
        -of default=noprint_wrappers=1:nokey=1 "$wav"
done | awk '{
    if ($1 < 1) bins[0]++;
    else if ($1 < 2) bins[1]++;
    else if ($1 < 5) bins[2]++;
    else if ($1 < 10) bins[3]++;
    else if ($1 < 20) bins[4]++;
    else bins[5]++;
} END {
    print "< 1s:  ", bins[0];
    print "1-2s:  ", bins[1];
    print "2-5s:  ", bins[2];
    print "5-10s: ", bins[3];
    print "10-20s:", bins[4];
    print "> 20s: ", bins[5];
}'
```

### Общая статистика

```bash
# Используйте jq для красивого вывода
cat dataset/metadata.json | jq '{
    total_segments,
    accepted_segments,
    rejected_segments,
    acceptance_rate: (.accepted_segments / .total_segments * 100 | floor),
    avg_duration,
    total_hours: (.total_dataset_duration / 3600 | floor)
}'
```

## Проверка перед обучением

### Checklist

- [ ] metadata.csv существует и не пуст
- [ ] Все файлы в wavs/ имеют формат 22050Hz mono
- [ ] Количество WAV и TXT файлов совпадает
- [ ] Нет маркеров музыки/шума в текстах
- [ ] Все сегменты 0.5-30 секунд
- [ ] Общая длительность датасета >= 30 минут
- [ ] Случайная выборка звучит качественно

### Автоматическая проверка

```bash
#!/bin/bash
# check_dataset.sh

echo "Checking dataset integrity..."

# Check metadata.csv
if [ ! -f "dataset/metadata.csv" ]; then
    echo "❌ metadata.csv not found"
    exit 1
fi

# Count files
wav_count=$(ls dataset/wavs/*.wav 2>/dev/null | wc -l)
txt_count=$(ls dataset/wavs/*.txt 2>/dev/null | wc -l)
csv_lines=$(wc -l < dataset/metadata.csv)

echo "WAV files: $wav_count"
echo "TXT files: $txt_count"
echo "CSV lines: $csv_lines"

if [ $wav_count -ne $txt_count ]; then
    echo "❌ WAV and TXT count mismatch"
    exit 1
fi

if [ $wav_count -ne $csv_lines ]; then
    echo "⚠️  Warning: WAV count ($wav_count) != CSV lines ($csv_lines)"
fi

# Check format
echo "Checking audio format..."
sample=$(ls dataset/wavs/*.wav | head -1)
format_check=$(ffprobe "$sample" 2>&1 | grep "22050 Hz, mono")

if [ -z "$format_check" ]; then
    echo "❌ Wrong audio format"
    exit 1
fi

echo "✅ Dataset looks good!"
echo "Ready for Piper TTS training!"
```

## Troubleshooting

### metadata.csv пуст или отсутствует

```bash
# Регенерировать вручную
cd dataset
for txt in wavs/*.txt; do
    wav="${txt%.txt}.wav"
    wav_name=$(basename "$wav")
    text=$(cat "$txt")
    echo "$wav_name|$text"
done > metadata.csv
```

### Неправильная кодировка в metadata.csv

```bash
# Проверить кодировку
file dataset/metadata.csv

# Должно быть: UTF-8 Unicode text
```

### Количество файлов не совпадает

```bash
# Найти WAV без TXT
for wav in dataset/wavs/*.wav; do
    txt="${wav%.wav}.txt"
    if [ ! -f "$txt" ]; then
        echo "Missing: $txt"
    fi
done

# Найти TXT без WAV
for txt in dataset/wavs/*.txt; do
    wav="${txt%.txt}.wav"
    if [ ! -f "$wav" ]; then
        echo "Missing: $wav"
    fi
done
```

## См. также

- [PIPER_TTS.md](PIPER_TTS.md) - Руководство по Piper TTS
- [README.md](README.md) - Основная документация
- [Piper Training](https://github.com/rhasspy/piper/blob/master/TRAINING.md) - Обучение Piper


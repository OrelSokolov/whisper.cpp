# Integration Tests

This directory contains the integration test suite for whisper.cpp.

## Installing dependencies

```bash
bundle install
```

## Running tests

### Entire suite
```bash
rake spec:all
# or
bundle exec rspec
```

### Integration specs only
```bash
rake spec:integration
# or
bundle exec rspec spec/integration
```

### Punctuation (CLI)
```bash
rake spec:punctuation
# or
bundle exec rspec spec/integration/punctuation_spec.rb
```

### Punctuation (Worker)
```bash
rake spec:worker_punctuation
# or
bundle exec rspec spec/integration/worker_punctuation_spec.rb
```

### Punctuation (Worker, reused instance)
```bash
rake spec:one_worker_punctuation
# or
bundle exec rspec spec/integration/one_worker_punctuation_spec.rb
```

## Requirements

Before running the specs, make sure that:

1. `whisper-cli` is built: `rake build:cli` (required for CLI tests)
2. `whisper-worker` is built: `rake build:worker` (required for worker tests)
3. The model file exists at `models/ggml-large-v3-turbo.bin`
4. The sample audio file is available at `~/tube/babes.mp3`

## Test descriptions

### Punctuation Test (CLI)

Confirms that `whisper-cli` adds commas and periods correctly for Russian speech.

The spec:
- launches `whisper-cli` with fixed arguments;
- terminates the process after 20 seconds;
- inspects the first three timestamped segments for punctuation;
- repeats the run 10 times;
- expects punctuation in at least 9 out of 10 runs.

### Punctuation Test (Worker)

Validates punctuation when `whisper-worker` serves transcripts over the WebSocket API.

The spec:
- starts a fresh `whisper-worker` instance for each run;
- records the worker PID so it can be shut down cleanly;
- drives the worker via the Ruby client (`examples/worker/client.rb`);
- stops the client after 20 seconds;
- stops the worker after each run;
- checks the first three segments for commas or periods;
- requires punctuation in at least 9 out of 10 runs.

### Punctuation Test (Worker, reused instance)

Measures whether preserving worker state between requests affects quality. One worker handles all 10 client runs.

The spec:
- starts `whisper-worker` once at the beginning;
- keeps the worker alive across runs, capturing its PID for cleanup;
- performs 10 executions of the Ruby client (`examples/worker/client.rb`);
- interrupts each client after 20 seconds while the worker remains running;
- stops the worker only after all runs finish;
- inspects the first three segments for punctuation;
- requires punctuation in at least 9 out of 10 runs.

This regression test helps detect changes in worker context handling that degrade punctuation across sequential requests.


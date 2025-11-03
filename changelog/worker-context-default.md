# Improve punctuation consistency for worker transcription

## Summary

- keep Whisper decoding context enabled by default in `whisper-worker`, matching the CLI defaults;
- expose `--no-context` / `--keep-context` switches so deployments can opt out explicitly.

## Impact

Keeping context across segments significantly improves punctuation and capitalization stability for repeated worker runs, restoring the integration test expectation that at least 9/10 invocations produce punctuated output. The new flags make the behaviour explicit while retaining backwards compatibility for anyone who depended on contextless decoding.


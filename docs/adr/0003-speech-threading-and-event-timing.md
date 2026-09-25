# ADR-0003: Speech threading and event timing

- Status: accepted
- Date: 2026-09-25

## Context

Speech engines disagree about threads and timing. AVSpeechSynthesizer wants the main thread; WinRT speech is apartment-affine; espeak-ng is a process-wide singleton. Some engines report word boundaries with an audio-clock timestamp (espeak-ng `audio_position`), some report them as they happen, some (the Omnivox subprocess protocol, which is write-only) report nothing.

Star's playback layer (`star/tts/manager/_playback.py`, inventoried in `docs/star-parity.md`) has hard-won rules, and some bugs: the `on_done` handler has no generation check, all timers share one stop event, and a late "done" from the previous sentence can kill the current highlight.

## Decision

**Threading.**
- `SpeechBackend` has **no `Send` bound**. `SpeechService::spawn` takes a `Send` `BackendFactory` and creates the backend on the service's own speech thread, which calls every backend method.
- Engines that must live on the main thread declare `Caps::REQUIRES_MAIN_THREAD`. The TUI cannot use them; the GUI (wave 3) hosts them by running the service loop on its main thread.
- The application talks to the service through a command channel and reads `SpeechStatus` from a status channel (`try_status`, `statuses`). Nothing blocks the UI thread.

**Backend timing contract.**
- `speak` starts speech and **returns without waiting for audio**. Events for that utterance arrive during `speak` or later from `poll`, which the service calls every few milliseconds while speech is active. This is what lets `stop` and `pause` reach the backend mid-utterance on the same thread.
- Every utterance ends with exactly one `Finished` or `Cancelled`.

**Generations.**
- Every utterance carries `UtteranceId { generation, chunk }`. The service bumps the generation before every stop or restart (Star's rule).
- The service's `EventSink` drops events from stale generations **before** the service logic sees them, so a late `Finished` or `Word` can never move the highlight of a newer reading. Backends can ask `is_current(id)` to abandon stale work.

**Word timing.**
- `RawEvent::Word { byte_range, audio_ms }`. With `audio_ms` (an `AUDIO_CLOCK` engine), the highlight is scheduled at `audio_ms + latency_offset` on the playback clock, never fired on arrival. The default latency offset is 120 ms (Star's `espeak_highlight_offset_ms`), configurable in `[speech] latency_offset_ms`.
- Without word events, the **timer pacer** estimates one word every `60 / (effective_wpm × highlight_speed)` seconds, with Star's guards: callback timeout 1.5 s, callback dead after 6 s, at most 1 word ahead while paced and 4 unpaced (constants in `pacing::PacingConfig`, tests ported from `tests/test_highlight_pacing.py`).
- Byte ranges are mapped to document ranges through the utterance's `OffsetMap` (ADR-0005). Engines report UTF-8 byte offsets; espeak-ng's are converted from its own positions in the backend.

**Pause and resume.** Native pause when the engine has `PAUSE`; otherwise emulated by stopping and restarting from the last callback-confirmed word (resume may repeat a word, never skips one). Edge cases:
- pause before the first word: resume at the utterance start;
- a late word event after pause: dropped at the sink (the pause bumps the generation);
- cursor moved while paused: resume from the cursor;
- pause inside inserted speech ("heading level 2"): resume at the span's anchor;
- queued chunks are cancelled by id.

**Queue.** The service keeps two chunks of lookahead so engines with per-utterance latency do not gap between sentences. `queue.rs` and `pacing.rs` are pure and tested with a fake `Clock`.

**Omnivox.** The subprocess backend drives the Emacspeak speech-server protocol on stdin. It gets no events, so it uses the timer pacer; word-level highlighting with Omnivox is not promised. Later options: an upstream `--events` side channel, or in-process `omnivox-tts` crates where chunk durations are known.

## Consequences

- One thread per service; frontends poll or select on a channel. No async runtime.
- Tests use the `recording` backend and a fake clock to check ordering, cancellation, pause edge cases, and pacing without audio.
- A backend that blocks in `speak` until audio ends violates the contract: it must move playback to a helper thread and report through `poll`.

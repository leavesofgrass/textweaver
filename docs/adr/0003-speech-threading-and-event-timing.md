# ADR-0003: Speech threading and event timing

- Status: accepted
- Date: 2026-09-25
- Status update (Saturday, September 26, 2026): Later work kept these rules and added to them. Statuses carry a reading generation, and the app follows the `ReadingGeneration` that each `read` returns, so a status from an earlier reading can never move the highlight (Agents B2 and D3). Continuous reading is planned in windows of about ten minutes, not to the end of the document (Agent D4). Engines that own playback declare `PLAYBACK_EVENTS`, so their word events fire on arrival (ADR-0012). An engine that keeps failing stops the reading after three failures in a row; a crashed or stalled engine host is restarted and reading goes on from the last word heard. The Apple `avspeech` backend needs the application's main run loop, which the terminal reader and `tw serve` pump (ADR-0008).
- Status update (Saturday, September 26, 2026, Phase 2, Agent P2a): nothing on the speech thread waits for long. Engine hosts start inside `poll` (ADR-0012), speech-dispatcher's replies are read in `poll`, and each backend's voices are listed once when it starts into a `VoiceCache` (SAPI's in the background) that `SpeechService::voice_list` reads from any thread without a round trip; a voice asked for by name while the list loads is resolved when it arrives. `SpeechService::sync` is the round-trip barrier tests use. A dead speech thread is replaced in place: the app restarts speech once by itself, and on the Restart Speech command, with the current settings (`App::set_speech_starter`).
- Status update (Saturday, September 26, 2026, Phase 1, Agent P1a): crash restarts are capped and a speech-thread panic is caught and reported; see **Failures (Phase 1)** below.

## Context

Speech engines disagree about threads and timing. AVSpeechSynthesizer wants the main thread; WinRT speech is apartment-affine; espeak-ng is a process-wide singleton. Some engines report word boundaries with an audio-clock timestamp (espeak-ng `audio_position`), some report them as they happen, some (the Omnivox subprocess protocol, which is write-only) report nothing.

Star's playback layer (`star/tts/manager/_playback.py`, inventoried in `docs/history/star-parity.md`) has hard-won rules, and some bugs: the `on_done` handler has no generation check, all timers share one stop event, and a late "done" from the previous sentence can kill the current highlight.

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

**Failures (Phase 1).**
- An engine that crashes or stalls in a reading is reset, and the reading goes on from the last confirmed word (`SpeechStatus::Restarted`). Another restart needs progress *past* that word: the repeated word itself does not count, or an engine that crashes on the same text would restart forever. A sentence gets at most three restarts; the next crash stops the reading with a message.
- A panic on the speech thread is caught. The thread drops the backend, sends one last `BackendError` saying what happened, and ends. `SpeechService::is_alive()` turns false and `poll_status()` reports `ServiceStopped`, so a frontend can tell a dead thread from an empty queue, announce the failure without speech, and spawn a new service.

**Omnivox.** The subprocess backend drives the Emacspeak speech-server protocol on stdin. It gets no events, so it uses the timer pacer; word-level highlighting with Omnivox is not promised. Later options: an upstream `--events` side channel, or in-process `omnivox-tts` crates where chunk durations are known.

## Consequences

- One thread per service; frontends poll or select on a channel. No async runtime.
- Tests use the `recording` backend and a fake clock to check ordering, cancellation, pause edge cases, and pacing without audio.
- A backend that blocks in `speak` until audio ends violates the contract: it must move playback to a helper thread and report through `poll`.

## See also

- [Speech engines and voices](../speech.md): the engines and how highlighting works with each.
- [Speech pipeline, step by step](../site/speech-pipeline.html): an interactive walk from a key press to a highlighted word.
- [ADR-0012: The engine host](0012-engine-host.md): the playback client for out-of-process engines.
- [Architecture](../dev/architecture.md): the crate map, the threads, and the path from a file to a spoken, highlighted word.
- [Documentation index](../README.md)

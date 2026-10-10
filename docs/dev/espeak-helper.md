# The eSpeak NG helper

eSpeak NG can run in two places. The in-process backend, `textweaver_speech::backends::espeak` (the `espeak` feature), loads libespeak-ng into textweaver and lets the library play its own audio. The helper, the crate `textweaver-espeak`, runs the same library in a separate program, `textweaver-espeak-host`, on the engine-host protocol that Eloquence, SAPI 5, and DECtalk already use ([ADR-0012](../adr/0012-engine-host.md)). This page explains how the helper is built, how textweaver chooses between the two, and the measurement behind that choice. The user's side of it is in [the speech guide](../speech.md#speechespeak-espeak-ng).

## Why a helper

A helper program solves two problems that the in-process backend cannot. First, a library can only be loaded by a program of its own architecture. The eSpeak NG installer for Windows exists in 64-bit and 32-bit builds, and textweaver itself is built for x86-64 and, from beta 1, for ARM64. A host built for the library's architecture lets any textweaver use any installed eSpeak NG: the 32-bit host runs a 32-bit library under 64-bit textweaver, and the x64 host runs the x64 library under textweaver for ARM, through Windows' emulation. Second, a fault inside the engine, such as an access violation in its audio code, ends the helper and not the reader. The utterance in progress reports an error, and the next utterance starts a fresh helper.

## How it is built

The helper reuses the existing code rather than adding a third copy of anything:

- **The library and its calls.** The host drives libespeak-ng through the speech crate's own loader and declarations (`espeak/sys.rs` and `espeak/ffi.rs`), which the crate compiles under its `espeak-phonemes` feature without registering an in-process backend. Four functions were added there for the host: `retrieval_start`, `retrieval_voices`, `retrieval_params`, and `retrieval_synthesize`, the last of which streams each block of samples, with the words that start in it, to a callback as libespeak-ng produces it. The in-process backend's file synthesis now goes through the same streaming path. `choose_library` makes the host load exactly the file textweaver chose for it.
- **The protocol.** `textweaver_espeak::protocol` defines the eSpeak NG payloads on top of the shared frames in `textweaver_enginehost::protocol`: a Speak carries the text (or one character, spoken by name), a SetVoice carries the voice, rate, and pitch, a Ready lists the installed voices, and a Word carries the word's byte range in the text and its first sample. Because eSpeak NG reports each word's position itself, there are no index marks as there are for ECI and DECtalk. The protocol version stays 1: no existing frame changed.
- **The host loop and the backend.** The host loop (`host/mod.rs`) follows DECtalk's: a reader thread handles Stop through the shared stop epoch, and the main thread writes each block as soon as the engine thread hands it over. The backend (`backend.rs`) is DECtalk's backend with the index marks removed: it starts the host without blocking the speech thread, plays the audio through the shared playback client, raises each word when its first sample is played, and restarts a host that crashed or stopped responding.
- **Choosing the host.** `discovery.rs` looks for the library in the order the in-process backend loads it (the components folder, `TEXTWEAVER_ESPEAK_LIBRARY`, then the usual install paths), reads its PE or ELF header with the shared `textweaver_enginehost::arch` module (which also replaced the identical copies in the ECI and DECtalk crates), and picks `textweaver-espeak-host.exe` for a library of textweaver's own architecture, `textweaver-espeak-host-x86.exe` for a 32-bit one, and `textweaver-espeak-host-x64.exe` or `-arm64.exe` across 64-bit architectures.
- **Building.** `cargo xtask hosts` builds and installs the x64 and x86 hosts on Windows, with the other engine hosts, and the release packages carry them. The `hosts32` step of `scripts/dev-check.ps1` builds every 32-bit host, now including DECtalk's and this one. Linux and macOS build no eSpeak NG host.

## How textweaver chooses

`textweaver-engines` registers one `espeak` entry, replacing the speech crate's built-in one, and `[speech.espeak] helper` decides what it starts (`espeak_path`):

- `"never"` runs eSpeak NG in process, and nothing else;
- `"always"` runs it in the helper, and nothing else;
- `"auto"`, the default, prefers the helper on Windows when the helper and a library are found, and the in-process backend elsewhere; on either platform it falls back to the other path when the preferred one is not available.

Windows release packages are built without the `espeak` feature, so there the helper is the only path; development builds with `--features espeak` have both.

## The measurement

The decision rests on a short measurement, `cargo run -p textweaver-espeak --example measure -- 30`, run on Friday, October 9, 2026, on the owner's Windows machine (x86-64, the 64-bit eSpeak NG installer) while other agents were building, with the virtual output, so nothing was played. Each figure is the median over 30 utterances of an 83-character sentence, with the 90th percentile in parentheses.

| Measure | In process | Helper |
|---|---|---|
| Start of the backend, once per session | 162 ms | 347 ms |
| First samples available to textweaver | 0.3 ms (0.4) | 0.6 ms (0.9) |
| First sample taken by the output | not measurable in process | 9.2 ms (20.9) |
| Word event against its audio | arrives 1,199 ms early (83 ms early) | 4.4 ms early (10.1 ms late) |
| Word positions and times, same history | identical | identical |

Four conclusions follow. The pipe adds about a third of a millisecond before the first audio, which no listener can hear. The output figure for the helper is the silent output's timer and the polling interval, not the engine; the in-process backend has an equivalent cost in its own audio device, which the virtual output cannot show. Word timing favors the helper: its word events are tied to the samples the output has actually taken, whereas the in-process events arrive in a burst, up to the whole utterance ahead of the audio, and the highlight depends on the fixed `latency_offset_ms` (120 ms by default) matching the sound card. Both paths report the same words at the same milliseconds, as the real-engine test `helper_and_in_process_words_agree_exactly` checks, so the highlight's positions are exact either way. The one cost is starting the helper, about a fifth of a second once per session.

The helper is therefore at least as good on Windows, and it is the default there. On Linux and macOS, where eSpeak NG is a system library of the program's own architecture, the in-process backend stays the default and no host is built.

A note on the comparison. libespeak-ng carries a little state from one utterance to the next: speaking the same sentence twice in a row can move later words by a few milliseconds, in either path. Word timings therefore compare only between engines with the same history, which is why the measurement checks agreement before its timing loop.

## Tests

- `tests/it/fake_host.rs` runs the real host binary with its fake engine (`--engine fake`) and a recording event sink over the silent output: events in order with each word at its sample, voices and an unknown voice, Stop, a crash that fails one utterance while the next starts a new helper, a hung engine killed after the stall timeout, and a character spoken by name.
- `tests/it/real_engine.rs` runs against the installed libespeak-ng when there is one and skips otherwise: the helper speaks with words in order, and its words agree exactly with the in-process backend's.
- Unit tests cover the protocol, the fake engine, host selection by architecture, and `espeak_path` in `textweaver-engines`.

Set `TEXTWEAVER_ESPEAK_OUTPUT=virtual` for every run, so that nothing is played.

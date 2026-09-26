# ADR-0007: ETI-Eloquence through an ECI host process

- Status: accepted
- Date: 2026-09-25

## Context

Jon's preferred voice is ETI-Eloquence (Code Factory "Eloquence for Windows" 6.1, installed on his machine; Star defaulted `tts_prefer_voice` to "eloquence"). Three ways in were measured on 2026-09-25:

- **SAPI5.** Code Factory registers 64-bit and 32-bit SAPI5 voices ("Eloquence US English" and nine others). They speak, but through SAPI the engine reports one word-boundary event per sentence, at the wrong time, and no bookmark events. The default Microsoft voice reports every word. SAPI is therefore usable for Eloquence speech but not for word highlighting.
- **The `tts` crate on Windows** uses WinRT/OneCore voices; Eloquence is not among them.
- **ECI directly.** `eci.dll` (ECI 6.1) is a 32-bit library. A 32-bit Rust spike (`tools/eci-spike/`) loaded it, inserted an index mark before every word, synthesized 4.7 s of audio into its own buffer in 8 ms, and received every mark at the exact sample offset of its word.

## Decision

- A new crate, `textweaver-eci`, provides an `eci` speech backend built on ECI, loaded at run time with `libloading`, never linked.
- **ECI always runs in a separate host process**, `textweaver-eci-host`, built for the library's architecture: `i686-pc-windows-msvc` for Code Factory's 32-bit `eci.dll`, native x86_64 for Voxin's `libibmeci.so` on Linux. The host receives utterances and voice parameters on stdin and returns PCM audio and index marks (with sample offsets) on stdout in a small framed binary protocol. The separate process also keeps a proprietary engine's crash from taking down the reader and keeps the GPL program at arm's length from the proprietary library.
- **The backend plays audio in the main process** (rodio with its `playback` feature) and emits `RawEvent::Word { byte_range, audio_ms }` from the index offsets, so it has `WORD_EVENTS | AUDIO_CLOCK | PAUSE | PITCH | VOLUME | SYNTH_TO_FILE`. Pause and resume are native because textweaver owns playback. `synthesize_to_file` writes WAV directly from the host's PCM.
- Voice parameters map onto ECI voice parameters (speed, pitch baseline, pitch fluctuation, head size, roughness, breathiness, volume, gender) and the eight voice presets in `eci.ini`; rate is calibrated to report `effective_wpm()`.
- Eloquence normalizes numbers, abbreviations, and dates itself (it spoke "9:30 a.m." correctly). The backend declares that, and the speech service skips textweaver's overlapping normalization transforms for it, keeping punctuation and pronunciation handling.
- Selection: on a machine with Eloquence installed, the `eci` backend is the highest-priority automatic choice, and `prefer_voice` defaults to "eloquence" (Star's default).
- macOS ships Eloquence as system voices; those arrive through the native AVSpeech backend in wave 3. A SAPI5 backend (other SAPI voices, and Eloquence without the host) is also wave 3.

## Consequences

- Windows builds need the `i686-pc-windows-msvc` target to produce the host (`rustup target add i686-pc-windows-msvc`; `cargo xtask eci-host` builds it and places it next to the main binaries).
- The container cannot test real ECI; tests run against a fake ECI library or a fake host. Real-engine tests run on Jon's machine and are marked `#[ignore]` unless `TEXTWEAVER_ECI=1`.
- Eloquence is proprietary and user-installed; textweaver ships no part of it.

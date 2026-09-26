# ADR-0009: SAPI5 voices on Windows

- Status: accepted
- Date: 2026-09-25

## Context

Windows users have SAPI5 voices from many sources: Microsoft's desktop voices (David, Zira), voices bundled with reading software (for example VW Paul, Kate, and James), and eSpeak's SAPI5 wrapper. Some are registered only for 32-bit programs (under `WOW6432Node`), so a 64-bit process cannot load them.

A probe on 2026-09-25 (`System.Speech`, 64-bit and 32-bit PowerShell, output to WAV files) found that every one of these voices reports a word-boundary event for each word, with its character position and audio position: Microsoft David and Zira (64- and 32-bit), VW Paul, Kate, and James (32-bit only), and eSpeak (32-bit only). David and Zira expand "9:30 a.m." into four events pointing at the same source range; eSpeak and the VW voices report sub-token ranges ("Dr", "9", "30"). ETI-Eloquence through SAPI is the exception (one event per sentence, ADR-0007) and is not the route for Eloquence.

## Decision

- A crate, `textweaver-sapi`, provides a `sapi` backend. Each voice runs in a host process, `textweaver-sapi-host`, built twice: `x86_64-pc-windows-msvc` for 64-bit voices and `i686-pc-windows-msvc` for 32-bit-only voices. The backend lists the voices of both registries (64-bit first when a voice exists in both) and starts the matching host.
- The host drives `ISpVoice` through the `windows` crate, synthesizes into a memory stream, and reports `SPEI_WORD_BOUNDARY` events with their audio stream offsets, sending PCM and events over a pipe as the ECI host does (ADR-0007). The backend plays the audio (rodio) and emits `Word { byte_range, audio_ms }`, so SAPI voices get audio-clock highlighting, native pause and resume, and WAV export.
- Character positions from SAPI are UTF-16 offsets into the text the host sent; the backend maps them to UTF-8 byte ranges of the utterance.
- Rate, pitch, and volume map onto SAPI's `-10..10` rate and `0..100` volume and XML pitch markup, with a measured wpm calibration per voice family.
- Eloquence SAPI voices are listed but flagged as having no word timing; Eloquence is served by the `eci` backend.
- A host process isolates the reader from crashes in third-party engines, and lets 32-bit voices work in a 64-bit textweaver.

## Consequences

- Windows builds produce two host binaries (`cargo xtask sapi-host`); CI builds the 32-bit host on the Windows runner.
- The host protocol duplicates the ECI host's in Wave 1; the two are merged into one shared engine-host protocol at integration.
- Automated real-voice tests use Microsoft David, Zira, and eSpeak; voices bundled with other products are the user's to enable.

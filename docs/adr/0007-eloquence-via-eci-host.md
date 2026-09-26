# ADR-0007: ETI-Eloquence through an ECI host process

- Status: accepted
- Date: 2026-09-25

## Context

ETI-Eloquence is the preferred voice of textweaver's primary user and of many screen-reader users (Star defaulted `tts_prefer_voice` to "eloquence"). It is proprietary and reaches users through several products: Code Factory's "Eloquence for Windows" (SAPI5 plus its ECI library), Voxin on Linux, and bundles in screen readers and reading software. The routes were measured on 2026-09-25:

- **SAPI5.** Code Factory registers 64-bit and 32-bit SAPI5 voices ("Eloquence US English" and nine others). They speak, but through SAPI the engine reports one word-boundary event per sentence, at the wrong time, and no bookmark events. The default Microsoft voice reports every word. SAPI is therefore usable for Eloquence speech but not for word highlighting.
- **The `tts` crate on Windows** uses WinRT/OneCore voices; Eloquence is not among them.
- **Voxin on Linux** (a licensed Voxin 3.3 installation). Its `libibmeci.so` is a 64-bit ECI library (libvoxin 1.5.8) that runs the 32-bit engine in its own `voxind` process. Driven with the same calls from `tools/eci-spike/voxin_spike.py` inside the textweaver dev container, it reported every word's index mark at its audio position ("Smith" 400 ms, "library" 1,286 ms at its default rate).
- **ECI directly on Windows.** Code Factory's `eci.dll` (ECI 6.1) is a 32-bit library. A 32-bit Rust spike (`tools/eci-spike/`) loaded it, inserted an index mark before every word, synthesized 4.7 s of audio into its own buffer in 8 ms, and received every mark at the exact sample offset of its word.

## Decision

- A new crate, `textweaver-eci`, provides an `eci` speech backend built on ECI, loaded at run time with `libloading`, never linked.
- **ECI always runs in a separate host process**, `textweaver-eci-host`, built for the library's architecture: `i686-pc-windows-msvc` for Code Factory's 32-bit `eci.dll`, native x86_64 for Voxin's `libibmeci.so` on Linux. The host receives utterances and voice parameters on stdin and returns PCM audio and index marks (with sample offsets) on stdout in a small framed binary protocol. The separate process also keeps a proprietary engine's crash from taking down the reader and keeps the GPL program at arm's length from the proprietary library.
- **The backend plays audio in the main process** (rodio with its `playback` feature) and emits `RawEvent::Word { byte_range, audio_ms }` from the index offsets, so it has `WORD_EVENTS | AUDIO_CLOCK | PAUSE | PITCH | VOLUME | SYNTH_TO_FILE`. Pause and resume are native because textweaver owns playback. `synthesize_to_file` writes WAV directly from the host's PCM.
- Voice parameters map onto ECI voice parameters (speed, pitch baseline, pitch fluctuation, head size, roughness, breathiness, volume, gender) and the eight voice presets in `eci.ini`; rate is calibrated to report `effective_wpm()`.
- Eloquence normalizes numbers, abbreviations, and dates itself (it spoke "9:30 a.m." correctly). The backend declares that, and the speech service skips textweaver's overlapping normalization transforms for it, keeping punctuation and pronunciation handling.
- **Library discovery**, first existing file wins: `TEXTWEAVER_ECI_LIBRARY` or the backend option; an OpenEVV installation (OpenEVVWindows' SAPI5 installer at `%ProgramFiles%\OpenEVV\lib_64\eci.dll`, or its NVDA add-on at `%APPDATA%
vdaddons\openevv\synthDrivers\_openevv\lib_64\eci.dll`); Code Factory's `eci.dll`; on Linux, Voxin's `libibmeci.so`. The host's architecture follows the library's (PE header): x64 or x86. OpenEVV is supported through its 64-bit library only, because its 32-bit build exports cdecl where IBM's interface is stdcall. `tw backends` names the product found and why it was chosen.
- **OpenEVV** (github.com/Mudb0y/openevv; packaged for Windows by masonasons/OpenEVVWindows) reimplements the engine in C behind IBM's ECI interface, and its language data is IBM's, which its authors state they cannot license. textweaver therefore detects and uses a copy the user installed, by default, but never bundles, downloads, or tests against it. Whether to install it is the user's decision.
- **IBM's Embedded ViaVoice 4.3 SDK** (`evvWXP.exe`, still served from IBM's public download host) is not a free engine. Its readme and guides, read on 2026-09-25, mark it "Licensed Materials - Property of IBM" under IBM's customer and program license agreements, and describe the Windows build as "prototyping only; not supported for an end-user application". textweaver neither bundles it nor downloads it for users.
- **No automatic installation of an unlicensed engine.** Fetching OpenEVV (or IBM's SDK) on the user's behalf at install or run time would make textweaver the means of distributing data no one can license, just later than bundling would; textweaver does not do it. It detects what the user installed and points to the licensed routes: Apple's built-in Eloquence voices on macOS, Voxin on Linux, and Code Factory on Windows.
- **Community dictionaries.** The backend loads the community IBMTTS pronunciation dictionaries (github.com/eigencrow/IBMTTSDictionaries, CC0 1.0; main, root, and abbreviation dictionaries for US English and German), vendored at a pinned monthly release in `third_party/ibmtts-dictionaries/`, through ECI's dictionary calls (`eciNewDict`, `eciLoadDict` per volume, `eciSetDict`). On by default; the user can turn them off or point to their own directory. Hyphens reach the engine unchanged so hyphenated entries match.
- Selection: on a machine with Eloquence installed, the `eci` backend is the highest-priority automatic choice, and `prefer_voice` defaults to "eloquence" (Star's default).
- macOS ships Eloquence as system voices; those arrive through the native AVSpeech backend in wave 3. A SAPI5 backend (other SAPI voices, and Eloquence without the host) is also wave 3.

## Consequences

- Windows builds need the `i686-pc-windows-msvc` target to produce the host (`rustup target add i686-pc-windows-msvc`; `cargo xtask eci-host` builds it and places it next to the main binaries).
- Tests run against a fake host everywhere. Real-engine tests are `#[ignore]`d unless `TEXTWEAVER_ECI=1` and need a licensed ECI library: in development, a licensed Voxin installation mounted read-only into the container with `compose.voxin.yaml` (docs/docker.md). Windows real-engine tests run only where a licensed Code Factory installation exists.

## Licensing

- Eloquence is proprietary. textweaver ships no part of it and links nothing: the user supplies a licensed ECI library, found at its default install location or named in `TEXTWEAVER_ECI_LIBRARY`. Whether a given product's license permits use by other programs is between the user and that vendor; a copy bundled with another application is often licensed for that application only.
- Engine output (audio made with Eloquence or Voxin) is never committed to this repository. Local samples go to git-ignored paths.

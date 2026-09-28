# ADR-0021: DECtalk through a host process

- Status: accepted
- Date: 2026-09-25
- Status update (Saturday, September 26, 2026): Integrated. The backend is registered by the app, and `[speech.dectalk] library` is read from the settings. It has still not been tested against a licensed DECtalk.
- Status update (Saturday, September 26, 2026, Phases 1 and 2): the DECtalk host synthesizes a long utterance a sentence at a time, so Stop takes effect quickly (Agent P2a), and CI builds the 32-bit DECtalk host and runs the fake-host tests against it (Agent P2d). It has still not been tested against a licensed DECtalk.

## Context

DECtalk is the formant synthesizer many blind users grew up with ("Perfect Paul"). Star supported it two ways (`star/tts/dectalk.py`): in process through ctypes on a `DECtalk.dll` that Star's Windows build vendored (per architecture, with `dtalk_us.dic`), speaking straight to the sound card, and through a `say`/`dtalk` command-line program. Neither reported word positions, so Star's highlight followed a timer. Star's in-process route also wrote a licence blob into shared memory before starting the engine, taken from the community DECtalk NVDA driver, so that the vendored community DLL would start.

DECtalk is proprietary. The community source tree on GitHub is Fonix's code; its own licence file says it may be used only under a written licence from Fonix. A DECtalk someone bought (DECtalk Software from DEC, Force Computers, or Fonix; the Access Solutions runtime; an SDK licence) is a different matter: it is theirs to use.

The owner asked for DECtalk "like we did in star". textweaver already runs two proprietary engines out of process on the shared engine host (ETI-Eloquence, ADR-0007; SAPI5, ADR-0009; the host protocol, ADR-0012).

## Decision

- A crate, `textweaver-dectalk`, provides a `dectalk` backend. DECtalk always runs in a host process, `textweaver-dectalk-host`, on the shared engine-host protocol and playback client (ADR-0012), exactly as ECI does. The backend plays the audio and emits `Word { byte_range, audio_ms }` from sample offsets, so it declares `WORD_EVENTS | AUDIO_CLOCK | PAUSE | PITCH | VOLUME | SYNTH_TO_FILE | PLAYBACK_EVENTS` (and `TONES` with the device output). Pause is native. Crash and hang recovery come from the shared client.
- **The engine is driven through DECtalk's documented C API** (`ttsapi.h`), loaded with `libloading`, never linked: `TextToSpeechStartupEx` (else `TextToSpeechStartup`) with `WAVE_MAPPER` and `DO_NOT_USE_AUDIO_DEVICE`, speech-to-memory with `TextToSpeechOpenInMemory(WAVE_FORMAT_1M16)` (11,025 Hz, 16-bit mono), and per utterance `TextToSpeechAddBuffer`, `TextToSpeechSpeak(TTS_FORCE)`, `TextToSpeechSync`, and `TextToSpeechReturnBuffer`. A two-minute buffer holds a sentence; when DECtalk fills one it hands it to the `TextToSpeechStartupEx` callback, which copies it and adds it back. The callback recognizes the buffer by its address, so it does not depend on message numbers.
- **Word timing from index marks.** The host puts `[:index mark n]` before every word, the same words and highlight ranges as ECI's (whitespace runs, punctuation trimmed), and reads each mark's sample number from the buffer's `TTS_INDEX_T` array. A *start mark* before the first word tells the host where the utterance begins in DECtalk's sample count, so offsets are right whether DECtalk counts samples from the stream or from each buffer, and whether the count restarts. Index values stay below 32,768.
- **Voice, rate, and pitch are inline commands**: `[:n<letter>]` selects one of the nine speakers (Paul, Harry, Frank, Dennis, Betty, Ursula, Wendy, Rita, Kit), `[:rate N]` sets words per minute (75 to 600; DECtalk's unit, so it is also the effective rate and needs no calibration), and `[:dv ap N]` shifts the speaker's average pitch by `2^(semitones/12)`. Volume is a playback gain. Voice ids are `dectalk:<name>`; Star's saved names ("Paul", "p") are accepted.
- **Text** is encoded as Latin-1; curly quotes, dashes, and the ellipsis become ASCII; other characters become spaces. `[` and `]` become `(` and `)`, so text can never inject a DECtalk command or phonemic input. DECtalk's own number and abbreviation rules are not declared as native normalization: textweaver's normalization runs first, as for the built-in engines.
- **Calling convention.** DECtalk for Windows is usually a 32-bit DLL, run by `textweaver-dectalk-host-x86.exe`. Its exports may be `cdecl` or `stdcall`. The host takes `stdcall` when the exports carry decorated names, and otherwise measures it: the first `TextToSpeechStartup` call goes through an assembly thunk that saves and restores the stack pointer and reports whether the library popped its arguments. `--convention` overrides it. 64-bit Windows and Linux have one convention.
- **Discovery, user-installed only**, first match wins: `TEXTWEAVER_DECTALK_LIBRARY`; the backend option `DectalkConfig::library` (the app maps a `[speech.dectalk] library` setting onto it); then `DECtalk\DECtalk.dll`, `Fonix\DECtalk\DECtalk.dll`, and `Fonix DECtalk\DECtalk.dll` under both Program Files folders and `dectalk.dll` in `System32` and `SysWOW64` on Windows, and `libtts.so` or `libdectalk.so` in `/usr/lib`, `/usr/local/lib`, and `/opt/dectalk/lib` on Linux. A file counts only if it contains `TextToSpeechStartup`; a library the user names that does not is reported, not skipped. The PE or ELF header picks the x64 or x86 host. The host changes into the library's folder before loading it, because DECtalk opens its dictionary from the current directory.
- **Selection.** Priority 300: below Eloquence (1000) and SAPI (500), above the built-in engines. It is chosen automatically only where neither is available; users who want DECtalk choose it.

## Licensing

- textweaver ships no DECtalk, links none, and never downloads or fetches one, at install time or at run time.
- textweaver does not vendor, build, or test against the community DECtalk source (its licence reserves it to Fonix's licensees). If a user points `TEXTWEAVER_DECTALK_LIBRARY` at a library, textweaver uses it as that user's choice, as it does with OpenEVV (ADR-0007).
- textweaver does not write Star's shared-memory licence blob or any other licence data. A licensed DECtalk handles its own licensing; one that does not start is reported as not starting.
- Engine output (audio made with DECtalk) is never committed; local samples go to git-ignored paths.

## Consequences

- Windows packaging builds two more hosts (`textweaver-dectalk-host.exe` and `textweaver-dectalk-host-x86.exe`); Linux builds the native host. A 32-bit Linux `libtts.so` would need a host built for `i686-unknown-linux-gnu`, which `cargo xtask hosts` does not build.
- Tests never touch DECtalk. The backend's suite runs the real host with an in-host fake engine. The FFI code runs against a stand-in library (`examples/fake_dectalk.rs`, a `cdylib` that exports the `TextToSpeech*` functions and speaks a square wave), covering start-up fallbacks, the buffer callback across many buffers, both sample-numbering conventions, and the text DECtalk receives; the same suite passed with a 32-bit host and 32-bit `cdecl` and `stdcall` builds of the stand-in (2026-09-25). Live tests are `#[ignore]`d unless `TEXTWEAVER_DECTALK=1`.
- Not yet verified against a licensed DECtalk: the structure layouts, the callback's signature and thread, and the index sample numbering follow DECtalk's published API reference; no licensed DECtalk is installed on the development machine. The live tests are the check when one is.
- Export (ADR-0011) gets exact word cues from DECtalk through `synthesize_utterance`.

## See also

- [Using DECtalk](../dectalk.md): the user guide.
- [Speech engines and voices](../speech.md): how DECtalk is chosen.
- [ADR-0012: The engine host](0012-engine-host.md): the shared host protocol.
- [Documentation index](../README.md)

# ADR-0012: One engine-host protocol and playback client

- Status: accepted
- Date: 2026-09-25
- Status update (Saturday, September 26, 2026): Implemented. DECtalk (ADR-0021) is the third engine on this protocol, still at version 1. A crashed or stalled host is restarted, and the speech service resumes from the last word heard and says "Speech restarted" (Agent D4).

## Context

Two speech engines run outside textweaver's process: ETI-Eloquence through its ECI library (ADR-0007) and SAPI5 voices (ADR-0009). Each runs in a *host* process that synthesizes into memory and streams PCM and word positions back over a pipe, and each backend plays the audio itself so word events follow the playback clock. Wave 1 built the two independently, with a note to merge them: `textweaver-eci` and `textweaver-sapi` each had a copy of the frame encoder and decoder, the host process handling (spawn, stderr logging, reader thread, shutdown), the sample feed and outputs, the utterance queue that turns positions into `Started`/`Word`/`Finished`/`Cancelled` events, and a WAV writer. The copies had already drifted: only ECI recovered from a hung host, only SAPI checked the host's protocol version and handled audio arriving out of turn, and SAPI did not declare `PLAYBACK_EVENTS`, so the speech service held each of its word events back by the latency offset meant for engines whose audio it cannot see, although the backend already emitted them as the word was heard.

## Decision

A crate, `textweaver-enginehost`, holds everything the hosts and their backends share. Engine crates keep only what is their own: their request and reply payloads, their engine code in the host, discovery, voices, and parameter mapping.

- `protocol`: framing, the shared messages, the version check, and a `Message` trait engine crates implement for their request and reply types.
- `process::HostProcess`: starts a host (no console window on Windows), logs its stderr, decodes its replies on a reader thread, sends requests, tracks when it last spoke, and shuts it down.
- `playback::Playback`: the audio-clock client. It queues utterances and captures, pushes audio into the feed in utterance order (audio for a later utterance waits for its turn), maps word positions through the engine's own per-utterance state, emits events per ADR-0003 against the samples actually played, pauses natively, stops, and fails what a dead host owed.
- `audio`: the sample feed (the playback clock), the device output through rodio (feature `playback`, off by default so hosts never link an audio stack), and a silent timed output for tests.
- `wav`: WAV writing and the volume gain applied to synthesized audio.
- `serve`: the host side: a request reader thread that handles `Stop` itself by bumping a stop epoch (so a stop reaches synthesis in progress) and stamps every other request with the epoch, and a frame writer several threads can share.

### Wire format

A frame is `len: u32 LE` followed by `len` bytes: a one-byte tag and its payload. Integers are little-endian; a string is a `u32` byte length and UTF-8; a list is a `u32` count and its items; a sample list is a `u32` count and 16-bit mono samples. Requests (backend to host, on stdin) use tags `0x01..=0x7f`; replies (host to backend, on stdout) use `0x81..=0xff`. A frame longer than 16 MiB, a zero length, an unknown tag, a bad value, or trailing bytes is an error.

| Tag  | Message | Shared payload | ECI | SAPI |
|------|---------|----------------|-----|------|
| 0x01 | Speak | `token: u64` (never 0), then the engine's utterance | pieces: `u32` count, each `kind: u8` (0: text `str`, 1: index mark `u32`) | text `str`, pitch `i8` (`-10..=10`) |
| 0x02 | Stop | none | | |
| 0x03 | (engine) | | SetVoice: dialect `u32`, preset `u8` | SetVoice: token id `str` |
| 0x04 | (engine) | | SetVoiceParam: param `u8`, value `i32` | SetRate: rate `i8` |
| 0x05 | Quit | none | | |
| 0x81 | Ready | `protocol: u16`, `sample_rate: u32`, then the engine's fields | version `str`, dialects `u32` list, default dialect `u32`, presets (name `str`, 8 × `i32`) list | architecture `str` (`x64`, `x86`), engine `str` (`sapi`, `fake`) |
| 0x82 | Audio | `token: u64`, samples | | |
| 0x83 | Word | `token: u64`, the engine's position, `sample: u64` last | Mark: index `u32` | Word: UTF-16 start `u32`, length `u32` |
| 0x84 | End | `token: u64`, `status: u8` (0 done, 1 aborted, 2 failed), `samples: u64` | | |
| 0x85 | Error | `token: u64` (0 = none), message `str` | | |
| 0x86 | (engine) | | Dictionary: dialect `u32`, volume `u8`, status `i32`, path `str` | Voice: token id, name, language, gender, vendor (`str` each) |

Tags `0x06..=0x7f` and `0x87..=0xff` are free for new messages.

DECtalk (ADR-0021) is a third engine on this protocol. Its Speak and Mark payloads are laid out as ECI's; its SetVoice (0x03) is speaker `u8`, rate `u16` (wpm), pitch `u16` (Hz, 0 = the speaker's own); its Ready adds version `str` and engine `str` (`dectalk`, `fake`). It changes no existing frame, so the version stays 1.

Conversation: the host announces itself with `Ready` (ECI may precede it with `Dictionary` reports; a host that cannot start sends one `Error` instead). Each `Speak` is answered by any number of `Audio` and `Word` frames and exactly one `End`, preceded by `Error` when synthesis failed. `Word` samples count from the utterance's first sample. `Stop` aborts the utterance in progress and every `Speak` read before it; each still gets its `End` (aborted). The host exits on `Quit` or end of input. SAPI's voice listing is a one-shot run (`--list-voices`): `Ready`, one `Voice` per token, exit.

### Versioning

- One version number, `PROTOCOL_VERSION`, covers the shared layout and every engine's payloads. It is 1: the Wave 1 hosts' bytes are unchanged, so hosts built before this ADR keep working.
- Any change to the layout of an existing frame bumps it. Adding a message with a new tag also bumps it, because an older peer rejects unknown tags.
- `Ready` always begins with the version and sample rate, so a backend can read them (`ReadyHeader::peek`) even from a host whose engine fields it cannot parse. A backend refuses a host whose version differs and says to rebuild the hosts with `cargo xtask hosts`.
- An undecodable reply ends the conversation: the backend treats the host as dead and starts a new one on the next request.

### Host lifecycle

- A backend starts its host on creation (ECI; SAPI's x64 host) or on first use (SAPI's x86 host) and waits up to 10 s for `Ready` (for ECI, up to 10 s after each dictionary report that precedes it, since loading dictionaries on a cold engine takes a while).
- A host that exits, breaks its pipe, or sends an undecodable frame is dead. A host that stays silent while it owes audio is hung: after 10 s for ECI (configurable, `EciConfig::stall_timeout`; Eloquence hangs on some inputs) and 60 s for SAPI (whose host already gives up on a silent voice after 30 s and reports an error). A hung host is killed.
- What a dead or hung host owed ends with `Error` then `Finished` (utterances) or an error (captures). The next request starts a new host: restart after a crash or a hang.
- Shutdown sends `Quit`, waits 500 ms, then kills the process. A host that has already failed (its output closed, a bad frame, a broken input pipe) is killed at once, without the wait.
- Hosts never outlive textweaver (Phase 1):
  - When a host's input ends without `Quit` (textweaver exited or crashed), the host bumps its stop epoch, so synthesis in progress aborts and queued requests are skipped, and it exits within 1.5 s even if its engine is stuck (`serve::AtEnd::Exit`).
  - On Windows every host joins one Job Object created with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`; only textweaver holds its handle, so the system ends the hosts when textweaver ends.
  - On Linux each host asks for `SIGKILL` when its parent dies (`prctl(PR_SET_PDEATHSIG)`, set between `fork` and `exec`). The signal follows the thread that started the host, so a host must be started on the thread that keeps it (the speech thread does).
- A host's stderr is always drained: lines that are not UTF-8 are logged with replacement characters. Hosts write to stderr only through `serve::log_line`, which ignores a closed stderr instead of panicking (a panic in a COM or C callback would abort the host).
- A request over the 16 MiB frame limit is refused by the backend before it is written ("this text is too long to speak in one piece"), and a host that is sent one anyway reads past it and keeps working.
- An audio output that stops taking samples while audio waits and nothing is paused is reopened after 1 s (backing off to 8 s), in or out of a reading.

### Capabilities

Both backends own playback, so both declare `PLAYBACK_EVENTS`: their word events arrive as the word is heard and the service fires them on arrival. `TONES` is declared only for the device output. SAPI reports capabilities for the selected voice: Code Factory's Eloquence voices drop `WORD_EVENTS` and `AUDIO_CLOCK` (ADR-0007), and Eloquence voices (Code Factory's and OpenEVV's) add `NATIVE_NORMALIZATION`.

### Packaging

`cargo xtask hosts` builds every host for the current platform (Windows: ECI and SAPI, x64 and x86; elsewhere: the native ECI host) as release builds without default features, and installs them with the community dictionaries next to the debug and release binaries, and into `--dest DIR` for a release package. CI and the release job call it. `cargo xtask eci-host` and `cargo xtask sapi-host` remain for building one engine's hosts.

## Consequences

- A fix to host handling or playback lands once and reaches both engines; a third out-of-process engine needs only its payloads, its host engine, and its discovery.
- The engine crates' public APIs are unchanged: `audio`, `wav`, and the framing items in `protocol` are re-exported from this crate.
- The fake-host suites of both engines now exercise the shared client, and the shared crate has its own tests: framing, the playback queue against a hand-driven clock, and a test binary that runs itself as a toy host to check spawning, crashes, hangs, bad frames, version refusal, and shutdown.
- Changing a frame now means bumping one version and rebuilding every host (`cargo xtask hosts`).

## See also

- [Speech engines and voices](../speech.md): the engines that run in hosts.
- [Installing textweaver](../install.md#windows): the hosts must stay next to the programs.
- [Speech pipeline, step by step](../site/speech-pipeline.html): where the host sits in the path from text to speech.
- [Architecture](../architecture.md): the crate map, the threads, and the path from a file to a spoken, highlighted word.
- [Documentation index](../README.md)

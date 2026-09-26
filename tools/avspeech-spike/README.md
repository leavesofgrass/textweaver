# Apple speech probes

Swift scripts run on GitHub's macOS 14 and 15 runners on 2026-09-25 to decide how textweaver drives Apple's voices (ADR-0008). They play nothing aloud (live speech at volume 0) and write only to temporary directories. Run one on a Mac with `swift tools/avspeech-spike/probe3.swift`; probes 5 and 6 take a scenario argument and are best compiled once (`swiftc probe6.swift -o probe6 && ./probe6 voice`).

- `probe.swift`: lists voices and tries `AVSpeechSynthesizer.write(_:toBufferCallback:toMarkerCallback:)`. Result: the Eloquence voices ship with macOS (112 on macOS 15.7, 80 on macOS 14; Reed, Shelley, Rocko, Sandy, Flo, Eddy, Grandma, Grandpa per language); buffer synthesis works (Eloquence at 16 kHz); the marker callback delivered **no** word markers for any voice.
- `probe2.swift`: word callbacks through the `AVSpeechSynthesizer` delegate (`willSpeakRangeOfSpeechString`) during `speak()` and during `write()`, and through `NSSpeechSynthesizer` (`willSpeakWord`). Result: every route reports every word for Eloquence Reed and Samantha. Eloquence reports sub-token pieces ("Dr", "9", "30", "a", "m"). First-word latency on macOS 15: NSSpeechSynthesizer 217 ms, AVSpeechSynthesizer `speak()` 865 ms (macOS 14: 213 ms).
- `probe3.swift`: in `write()` mode, word callbacks interleave with buffer callbacks, so the sample count at each callback is the word's audio offset (macOS 15: Reed "Smith" at sample 6,611 = 413 ms, matching live speech; macOS 14 reports in coarser chunks). Creating and driving the synthesizer on a background thread with its own run loop behaves the same **while the main thread runs its run loop** (see probe 5).

Added by Agent F (Wave 1):

- `probe4.swift`: `NSSpeechSynthesizer` created and driven on a background thread that pumps its own run loop, with the main thread asleep. Result (macOS 14.8 and 15.7): it speaks, and `isSpeaking` goes false at the end, but **no** `willSpeakWord` and no `didFinishSpeaking` ever arrive; `startSpeaking(_:to:)` writes the file but never reports finishing. AIFF-C output, 22,050 Hz, 16-bit big-endian. The pitch base property reads 44 for Reed and Samantha; the rate reads 175.
- `probe5.swift`: which thread runs the callbacks, each scenario in a fresh process. Results, identical on macOS 14 and 15:
  - `NSSpeechSynthesizer` delegate callbacks run on the **main thread, through the main run loop**: they arrive when the main thread runs its run loop, not when it only services the main dispatch queue (`dispatchMain()`), and never when it sleeps.
  - `AVSpeechSynthesizer` buffers and delegate callbacks go through the **main dispatch queue**: they arrive when the main thread runs its run loop or `dispatchMain()`, never when it sleeps (a `write()` then produces no buffers at all).
  - The **Speech Synthesis Manager** (`SpeakCFString` with `kSpeechWordCFCallBack` and `kSpeechSpeechDoneCallBack`, the C API that `NSSpeechSynthesizer` wraps) calls its callbacks on its own threads: every word and the end arrive with the main thread asleep.
- `probe6.swift`: the Speech Synthesis Manager as a backend, main thread asleep. Results (macOS 14 and 15):
  - Voice selection: `NSSpeechSynthesizer.attributes(forVoice:)` gives `VoiceNumericID` (no creator); walking `CountVoices`/`GetIndVoice` finds the `VoiceSpec` (Reed: creator 875705394, id 1043283216). The first walk takes 110 to 240 ms (engine start), later ones under 1 ms.
  - Words: Reed reports the same sub-token pieces as `NSSpeechSynthesizer`; Samantha reports whole tokens with punctuation ("Dr.", "9:30", "a.m.", "crème,"). First word 91 to 104 ms after `SpeakCFString` warm, about 600 ms cold.
  - `PauseSpeechAt(kEndOfWord)` pauses at the end of the word (status `OutputPaused = 1`, no words while paused); `ContinueSpeech` resumes.
  - `StopSpeech` **does** fire the done callback, so a stopped channel's late "done" could end the next utterance; the backend closes the channel on stop instead.
  - `kSpeechOutputToFileURLProperty` writes AIFF-C much faster than real time and still reports every word.
  - Pitch base is a note number: 32, 44, 56 gave 122, 179, 339 zero crossings per voiced second (an octave per 12).

## What textweaver does with this

- `nsspeech` drives the Speech Synthesis Manager directly (`crates/textweaver-apple/src/macos/ssm.rs`), so it needs nothing from the main thread.
- `avspeech` drives `AVSpeechSynthesizer.write` on the speech thread; the application's main thread must run its run loop (`textweaver_apple::run_main_loop_until`, or `pump_main_loop` each tick). Without it, each utterance ends after five seconds with an error that says so.

## Measurements (Rust voice tests, `crates/textweaver-apple/tests/voices.rs`)

Printed by `cargo test -p textweaver-apple --test voices` with `TEXTWEAVER_APPLE=1`; the `Apple speech` workflow runs them on macOS 14 and 15 and CI's `macos-latest` job runs them too. The report in the Agent F Wave 1 summary quotes the numbers.

# ADR-0008: Apple speech on macOS

- Status: accepted
- Date: 2026-09-25
- Status update (Saturday, September 26, 2026): Integrated in 0.1.0-alpha.2. As built, `nsspeech` drives Apple's classic engine through its C API (the Speech Synthesis Manager), because `NSSpeechSynthesizer` delivers its callbacks only on the main run loop; `avspeech`'s callbacks arrive through the main dispatch queue, so it works only while the application pumps the main run loop, which the terminal reader and `tw serve` do; `tw speak` does not, so live `avspeech` playback from `tw speak` is expected to fail (untested on a Mac). The `[speech.apple] backend` setting chooses between them. Neither has been tested with VoiceOver on a real Mac.

## Context

textweaver needs a macOS build that speaks with Apple's system voices. Since macOS 13 those include ETI-Eloquence (Reed, Shelley, Rocko, Sandy, Flo, Eddy, Grandma, Grandpa, in ten or more languages), licensed by Apple and free to users. Reed is the primary user's voice. Some screen-reader users prefer the older `NSSpeechSynthesizer` over `AVSpeechSynthesizer` because it responds faster.

Probes on GitHub's macOS 14 and 15 runners (`tools/avspeech-spike/`) found:

- `AVSpeechSynthesizer.write(_:toBufferCallback:toMarkerCallback:)` synthesizes into buffers, but its marker callback delivered no word markers for any voice.
- The `AVSpeechSynthesizer` delegate's `willSpeakRangeOfSpeechString` fires for every word, during live `speak()` and during `write()`. In `write()` mode the word callbacks interleave with the buffer callbacks, so the samples received so far give each word's exact audio offset (macOS 15; macOS 14 reports in coarser chunks). This works on a background thread with its own run loop.
- `NSSpeechSynthesizer`'s `willSpeakWord` fires for every word during live speech, with the lowest first-word latency measured (217 ms against 865 ms for `AVSpeechSynthesizer.speak()` on macOS 15). It takes rate in words per minute natively and can pause at a word boundary.
- Eloquence voices report sub-token ranges ("Dr" for "Dr.", "9" and "30" for "9:30"); every range lies inside one source word, so the offset map resolves them to the right word.

## Decision

- A macOS-only crate, `textweaver-apple` (empty on other platforms), provides two backends; the user chooses between them, and both are listed by `tw backends`:
  - **`nsspeech`** (`NSSpeechSynthesizer`): the system plays the audio; word events fire on arrival; native pause at a word boundary, rate in wpm, pitch and volume; `synthesize_to_file` through `startSpeakingString:toURL:`. Chosen for responsiveness.
  - **`avspeech`** (`AVSpeechSynthesizer`): `write()` into buffers, word callbacks paired with the running sample count, playback by textweaver (as in ADR-0007), so word events carry `audio_ms` on the audio clock; native pause and resume because textweaver owns playback; WAV export. Chosen for exact highlighting.
- Both run on the speech service's thread, which pumps its own run loop in `poll` (ADR-0003); neither needs the process's main thread.
- Bindings use the `objc2` family (`objc2`, `objc2-foundation`, `objc2-app-kit`, `objc2-avf-audio`, `block2`), macOS-only dependencies.
- The default voice is Eloquence Reed (`com.apple.eloquence.en-US.Reed`) when present; the default backend is chosen after measuring first-word latency and highlight accuracy on a real Mac.
- Builds: `aarch64-apple-darwin` and `x86_64-apple-darwin`, combined into a universal binary by a release job. Code signing and notarization need an Apple Developer ID and are decided separately.

## Consequences

- Speech tests run on the CI macOS runners, which have the Eloquence voices and can synthesize (no audio is played in tests).
- The VoiceOver experience of the terminal UI in Terminal.app needs testing on a real Mac.

## See also

- [Speech engines and voices](../speech.md): choosing `nsspeech` or `avspeech`.
- [Getting ETI-Eloquence](../eloquence.md#mac): Eloquence voices on macOS.
- [Using textweaver with a screen reader](../screen-readers.md): VoiceOver notes.
- [Documentation index](../README.md)

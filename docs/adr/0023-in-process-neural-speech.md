# ADR-0023: Piper voices and Whisper dictation in-process on RTen

- Status: accepted
- Date: 2026-09-26 (Saturday, September 26, 2026)
- Supersedes, in part: ADR-0013 (dictation through a Whisper program), whose subprocess design is now the fallback.

## Context

Wave 3 asked for neural voices (Piper) and in-process dictation (Whisper), pure Rust where that is viable (the Wave 3 pure-Rust research, kept outside the repository).

- **Piper** voices are VITS models exported to ONNX, one `.onnx` file plus an `.onnx.json`, phonemized with eSpeak NG. tract and candle cannot run the VITS graphs yet. RTen, Robert Knight's pure-Rust ONNX runtime (0.26), can; `rten-examples/src/piper.rs` shows how.
- **Whisper**: the Wave 3 plan named candle. candle builds the Oniguruma C library through `tokenizers`, so it is not pure Rust. RTen runs Whisper too (`rten-examples/src/whisper.rs`), with onnx-community's int8 exports. The owner approved RTen over candle on Saturday, September 26, 2026.

One runtime then covers Piper, Whisper, and (for W3d) OCR.

## Decision

### Piper (`textweaver-piper`)

- **Runtime:** RTen 0.26 in-process. Models are read into memory (`Model::load`), not memory-mapped (RTen's `load_mmap` is `unsafe`). `rten*` crates build at `opt-level = 3` in the dev profile.
- **Phonemes:** our libespeak-ng run-time loader gains `espeak_TextToPhonemes` (IPA mode), behind a new `espeak-phonemes` feature of `textweaver-speech` that registers no espeak-ng backend. When the library is missing (or has no such call), the pure-Rust `espeak-ng` crate (0.2, GPL-3.0-or-later, English data built in and written once to `<data>/piper/espeak-ng-data`) phonemizes instead. The text is split into clauses here, and each clause's punctuation is put back into the phoneme string as Piper's own phonemizer does (`, ` `; ` `: ` `.` `!` `?`).
- **Word timing** comes from the model. The duration predictor's rounded output (`w_ceil` in Piper's code) is the graph's only `Ceil` node, `/Ceil_output_0`, and is asked for as a second output of the same run. Each phoneme id records which text word it came from; a word starts at the sum of the frames before its first id, times the audio's samples per frame. libespeak-ng joins some function words ("in the" becomes `ɪnðə`); the joined phonemes are shared between the words by length, so every word still gets a start. A voice without the node speaks with no word events, and the service's timer paces the highlight.
- **Synthesis in chunks:** a sentence each, with the first clause of a long first sentence on its own, so the first audio comes sooner.
- **Playback** goes through `textweaver-enginehost`'s shared playback client (as ECI, SAPI, and DECtalk), fed by a worker thread instead of a host process. The backend declares `WORD_EVENTS | AUDIO_CLOCK | PLAYBACK_EVENTS | PAUSE | PITCH | VOLUME | SYNTH_TO_FILE` (and `TONES` on the device).
- **Rate** maps to the length scale (a voice at scale 1 reads about 210 words per minute), clamped to 0.3 to 3.0. **Pitch**, which Piper cannot do, is a resampling of the audio with the length scale raised by the same factor, so the rate stays.
- **Voices and licences:** the catalogue is `voices.json` from `rhasspy/piper-voices`. Licences differ per voice and are read from each voice's `MODEL_CARD`; the app says the licence before a download. `Licence::may_bundle` refuses non-commercial and unrecognized licences; textweaver bundles no voice.
- **Downloads** happen only after the user says yes, file by file into a temporary folder, each checked against Hugging Face's published hash (`lfs.oid`, the SHA-256, for the model; the git blob SHA-1 for the small files), and moved into `<data>/piper/voices/<key>/` only when all match. Requests carry a neutral User-Agent and nothing personal.
- **Fallback:** a voice RTen cannot load is spoken by the `piper` program when one is installed (`TEXTWEAVER_PIPER` or `PATH`), one chunk per run, with estimated timing.
- **Registry:** `backend_description`, `probe`, and `factory`, like the other engine crates, so the entry moves with the registry into `textweaver-engines` (W3c). Priority 200: below Eloquence, SAPI, and DECtalk; above eSpeak NG.

### Whisper (`textweaver-dictation`, feature `rten`)

- **Runtime:** RTen, with onnx-community's int8 exports (`encoder_model_int8.onnx`, `decoder_model_merged_int8.onnx`, `tokenizer.json`). The mel filter bank is computed (Slaney scale, checked against librosa's table), not shipped.
- Three things the example did not need:
  - onnx-community's merged decoder declares `use_cache_branch` with one dimension; rten-generate feeds a scalar. We feed it ourselves.
  - Their `tokenizer.json` predates `ignore_merges`, which rten-text requires; it is filled in at load.
  - **Their int8 weights use all 8 bits.** RTen's int8 kernels on x86-64 without VNNI (AVX2 only: AMD Zen 3, Intel before Ice Lake) use `vpmaddubsw`, which saturates, and Whisper produced nonsense ("s s s s") on the owner's Ryzen 5 5600G. RTen's own quantizer keeps weights to 7 bits for this reason. So at load every int8 weight of a `MatMulInteger` or `ConvInteger` is halved and its scale doubled, in place in the protobuf bytes (`onnx_patch`). Transcription is then correct.
- **Audio:** WAV files are read here (8 to 32-bit PCM, float, extensible headers); resampling to 16 kHz is rubato's FFT resampler; earshot (pure Rust, model built in) finds speech and splits it at pauses of about 600 ms, so silence is skipped and each utterance is transcribed on its own.
- **Microphone:** rodio's `recording` feature (feature `mic`), on its own thread.
- **Fallbacks:** the Whisper programs (ADR-0013) stay. **candle was never built** in textweaver, so there is no candle feature to keep; this ADR records the switch from the plan's candle to RTen instead.

## Measurements

On the owner's desktop (AMD Ryzen 5 5600G, 6 cores), Saturday, September 26, 2026, debug build with RTen at `opt-level = 3`, `en_US-joe-medium`:

- Model load: about 0.1 s (0.5 to 0.8 s while other builds loaded the machine).
- Real-time factor: 0.13 with libespeak-ng phonemes, 0.18 with the pure-Rust phonemizer, on a quiet machine. Under full load from five other agents' builds it rose to 1.0 and 1.35.
- Time to first audio (warm model): 77 ms for "Next heading."; 274 ms for a 60-character first clause; 284 ms cold on the first paragraph.
- Word timing: for words that follow a pause, the predicted start was 26 to 34 ms from where the sound starts on average (worst 130 to 180 ms), over 26 to 30 pauses in 154 words. Every word was timed.
- The two phonemizers differed in 2 of 167 phonemes on the test clauses (word joins aside).

Whisper base.en int8 on the same machine, under load: 2.3 s of speech ("Please add a note about chapter three.") was transcribed correctly. See `docs/dev/releasing.md` and the ADR's status updates for quiet-machine latency.

## Consequences

- One pure-Rust runtime for voices, dictation, and OCR; no ONNX Runtime, no C++ build.
- The int8 weight patch costs one bit of weight precision, which RTen's own tooling accepts; if onnx-community publishes 7-bit weights, or RTen gains AVX-VNNI and a non-saturating AVX2 path, the patch changes nothing and can go.
- `textweaver-app` depends on `textweaver-piper` (RTen and the English phonemizer data are always built). Build time rose by a few minutes cold.
- Tests never play audio: the real-voice and real-model tests are ignored unless a voice or model path is given, and write nothing to the sound device.

## See also

- [ADR-0012: the engine host and playback client](0012-engine-host.md)
- [ADR-0013: dictation through a Whisper program](0013-dictation.md)
- [Speech engines and voices](../speech.md)

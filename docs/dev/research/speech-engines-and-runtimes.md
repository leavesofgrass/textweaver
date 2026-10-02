# Speech engines, neural voices, and runtimes: what to adopt next

This report surveys the open neural voices, the Rust inference runtimes, and the speech-path engineering that textweaver could adopt after 0.1.0-alpha.7, with performance as the first criterion. It is for the maintainers who decide what goes into alpha.8 and alpha.9, and for anyone running a spike on a new voice or runtime. Written Friday, October 2, 2026, from the repository and the sources listed at the end. Numbers measured on other people's machines are marked "reported"; what could not be checked is marked "unverified".

## Where textweaver stands

- Piper voices run in process on RTen 0.26.0 (`Cargo.toml`, `Cargo.lock`). Word timing is the duration predictor's `w_ceil` tensor, read as a second graph output (`crates/textweaver-piper/src/model.rs`). Rate maps to the length scale; pitch is a rubato windowed-sinc resample with 128 taps (`crates/textweaver-piper/src/synth.rs`).
- Synthesis is a sentence at a time, with the first clause of a long first sentence on its own (`crates/textweaver-piper/src/text.rs`), on a worker thread, played through the shared playback client (`crates/textweaver-piper/src/backend.rs`, `crates/textweaver-enginehost/src/playback.rs`).
- The device output is rodio 0.22.2 over cpal 0.17.3 (`crates/textweaver-enginehost/src/audio.rs`). No buffer size is set, so rodio's default applies: it "configures a default buffer size of 100ms latency regardless of the system default" [26].
- The service keeps two chunks of lookahead in the engine and normalizes one more (`crates/textweaver-speech/src/queue.rs`, `DEFAULT_LOOKAHEAD = 2`; `service.rs`, `refill`). Pause is native for Piper (`Caps::PAUSE` in `backend.rs`). A rate or pitch change restarts, because Piper lacks `LIVE_RATE` (`respeak_with_new_params` in `service.rs`).
- Measured on a Ryzen 5 5600G (ADR-0023): real-time factor 0.13 with libespeak-ng phonemes and 0.18 with the pure-Rust phonemizer; first audio 77 ms for two words and 274 ms for a 60-character clause, before the device buffer.
- `synth.rs` mentions `cargo xtask piper-bench`, but `xtask/src/main.rs` has no such subcommand; `cargo xtask bench` times "open to first speech" with a silent bench-clock backend (`xtask/src/bench.rs`), not Piper. `crates/textweaver-piper/src/phonemes.rs` has no phoneme cache.

## 1. Open neural voices that fit a student's laptop

The bar: no GPU, no network after the download, a licence textweaver can at least download under, and some way to time words. The last point is what divides the field. VITS-style models (Piper, MeloTTS) and StyleTTS 2 descendants (Kokoro, Kitten) have a duration predictor whose output gives each phoneme's length, so a word's start is a sum. Flow-matching and language-model voices (Supertonic, ZipVoice, Pocket TTS, Chatterbox, Orpheus, OuteTTS) emit audio with no alignment, so the highlight would fall back to the timer pacer of ADR-0003.

| Voice | Licence (code / weights) | Size | CPU real-time factor (reported) | Word timing | Phonemizer | Languages | ONNX for a pure-Rust runtime |
|---|---|---|---|---|---|---|---|
| Piper (OHF-Voice/piper1-gpl 1.8.0, September 4, 2026) [1][2] | GPL-3.0 / per voice (CC0, CC BY, some non-commercial) | 20 to 140 MB per voice | 0.13 on Ryzen 5 5600G (ADR-0023) | yes, `w_ceil` durations | espeak-ng | about 30 | yes; runs on RTen today |
| Kokoro-82M (hexgrad) [3][4] | Apache-2.0 / Apache-2.0 | 82M; ONNX 326 MB fp32, 92 MB int8 (reported [20]) | 0.16 on M3 Pro (reported [5]) | yes, `pred_dur` in the pipeline [4]; ONNX export with a duration output in kokoro-onnx [6] | misaki G2P, espeak-ng fallback | 9 | yes; `rten-examples/src/kokoro.rs` exists [7] |
| Supertonic 3 (Supertone) [8][9] | MIT / OpenRAIL-M | 99M | 0.20 at 16 threads (reported [9]) | none; "does not include" alignment [8] | own frontend, no espeak-ng stated | 31 | yes; sherpa-onnx int8 export [10] |
| Kitten TTS (KittenML) [11][12] | Apache-2.0 / Apache-2.0 | nano 15M (25 MB int8), micro 40M, mini 80M | nano int8 0.12, mini 0.26 on M4 Pro via ort (reported [12]) | not documented | espeak-ng, required | English | yes |
| Matcha-TTS [13] | MIT | small (count not stated) | "very fast", ODE steps configurable | yes, alignments | not stated (icefall exports use espeak-ng, unverified) | English models; others in sherpa-onnx | yes, with vocoder embedded |
| MeloTTS [14] | MIT | not stated | "CPU real-time" | VITS lineage, durations exist (exposure unverified) | per language (unverified) | EN, ES, FR, ZH, JA, KO | sherpa-onnx exports (unverified) |
| Pocket TTS (Kyutai) [15] | MIT / MIT | 100M | about 6x real time on 2 cores of an M4 (reported); 200 ms first chunk | no; a community fork adds it | tokenizer, no phonemes | 7 | community export only |
| ZipVoice (k2-fsa) [16] | Apache-2.0 | 123M | not stated | no | pypinyin for Chinese | ZH, EN | yes, with int8 |
| Chatterbox-Nano / Turbo [17] | MIT | 110M / 350M | Nano 3x real time on 8 cores (reported) | no | tokenizer | EN (multilingual 500M model) | not documented |
| OuteTTS 1.0 [19] | Apache-2.0 | 0.6B, 1B | llama.cpp on CPU, below real time (unverified) | no | LLM tokens | several | GGUF, not ONNX |
| StyleTTS 2 | MIT code; Kokoro's parent lineage [3] | about 100M | similar to Kokoro | yes, duration predictor | espeak-ng | EN | community exports (unverified) |
| Orpheus 150M to 3B, Dia 1.6B, Zonos 1.6B and 8B [18]; Parler-TTS 880M [20]; NeuTTS Air 0.5B [21] | Apache-2.0 (NeuTTS: not stated) | GPU class | below real time on a laptop CPU | no | tokens | | no |
| F5-TTS 336M, Fish Speech, XTTS-v2 467M [20] | CC-BY-NC 4.0, CC BY-NC-SA 4.0, Coqui Public Model License | GPU class | | no | | | no; non-commercial licences `may_bundle` refuses |

Notes on the table:

- Piper's upstream moved from rhasspy/piper (archived October 2025, MIT) to OHF-Voice/piper1-gpl (GPL-3.0), which added Japanese (1.7.0) and Thai (1.8.0) phonemizers [1][2]. Piper has no SSML; `--sentence-silence` is the only pause control [22].
- Kokoro's pipeline computes `start_ts` and `end_ts` per token from `pred_dur`, at 600 frames per second of 24 kHz audio [4]. The stock ONNX export outputs only `waveform` [7], but kokoro-onnx's `scripts/export.py` adds a duration output, and `kokoro-v1.1-zh.onnx` ships with one [6]. That is the trick textweaver already plays with Piper's `w_ceil`.
- Supertonic's weights are OpenRAIL-M, a licence with use restrictions that `Licence::may_bundle` in `crates/textweaver-piper/src/catalog.rs` does not recognize, and it has no alignment anyway.
- sherpa-onnx (Apache-2.0, C++ with Rust bindings) packages Piper, Matcha-TTS, Kokoro, ZipVoice, Pocket TTS, and Supertonic exports [23]. Its catalogue is the best index of which models have working ONNX graphs; its C++ runtime is not one textweaver would link.

### Which two or three are worth a spike

1. **Kokoro-82M on RTen.** Apache-2.0 weights that `may_bundle` can accept, nine languages, word timing from a duration output, and an RTen example in the tree (`rten-examples/src/kokoro.rs`, which takes IPA and the `af_heart.bin` style vector) [7]. The spike: export with `scripts/export.py` from https://github.com/thewh1teagle/kokoro-onnx (duration output), load on RTen 0.26, feed IPA from `crates/textweaver-piper/src/phonemes.rs`, and map durations to words as `synth.rs` does. Risks: 326 MB in memory at fp32 (reported), and an int8 export whose weights use all 8 bits hits the AVX2 saturation ADR-0023 patched for Whisper. misaki's English G2P would need a Rust port (`misaki-rs` is on crates.io as a dependency of `kokoro-en`), or Kokoro takes espeak IPA at a quality cost.
2. **Kitten TTS nano.** Apache-2.0, 25 MB at int8, English, espeak-ng input like Piper, 0.12 real-time factor through ort (reported) [12]. The only candidate small enough to ship inside a Linux tarball for machines where a 60 MB download is a problem. The spike: load the nano int8 ONNX on RTen and look for a duration tensor. Repository: https://github.com/KittenML/KittenTTS.
3. **Matcha-TTS on RTen** only if the first two fail on operator coverage: MIT, alignments available, ONNX export with the vocoder embedded [13]. Lower voice quality than Kokoro.

Not worth a spike for the highlight: Supertonic, Pocket TTS, ZipVoice, and Chatterbox-Nano. They are fast enough, but they time nothing.

## 2. Inference runtimes from Rust

| Runtime | Pure Rust | VITS and Kokoro graphs | int8 | fp16 | CPU threading | Model load | Notes |
|---|---|---|---|---|---|---|---|
| RTen 0.26.0, August 29, 2026 [24][25] | yes | yes (Piper runs; Kokoro example) | yes: `MatMulInteger`, `ConvInteger`, dynamic quantization fusions; 4-bit `MatMulNBits` | weights up-converted to f32 at load; no f16 compute | thread pool sized to physical or performance cores | `Model::load` copies; `load_mmap` is `unsafe`; `external_data_static` embeds data without copying | AVX2, AVX-512, NEON, WASM SIMD |
| ort 2.0.0-rc.13 (ONNX Runtime 1.28) [27][28] | no, C++ | yes, widest coverage | yes, with VNNI and AMX kernels | limited on CPU | intra-op pool | memory map available | default feature downloads Microsoft binaries that may carry telemetry; static linking "not recommended" by Microsoft; CPU package about 21 MB installed (Arch 1.22.2) [29] |
| tract 0.23.8, September 21, 2026 [30][31] | yes | no: dynamic `Range` for shape construction fails on VITS graphs (#2928), `RandomNormalLike` unimplemented | yes | no | rayon | copies | good for fixed-shape CNNs |
| candle 0.11.0 | mostly; `tokenizers` pulls Oniguruma C (ADR-0023) | needs a hand-written model, not an ONNX graph | yes, GGUF | yes | rayon | safetensors mmap | a tensor library, not a graph runtime |
| burn 0.21.0 with burn-onnx 0.22.0-pre [32] | yes | converts ONNX to Rust code at build time; 134 of 160 operators (84%) | partial (unverified) | yes on wgpu | ndarray or wgpu backends | compiled in | long compile times; each voice export becomes code |
| wonnx 0.5.1 | yes | GPU only | no | | | | archived September 1, 2026 [32] |
| Lele, OxiONNX, tpt-infer, yscv-onnx [33] | yes | unproven | varies | | | | all announced in 2026; none has a VITS or Kokoro example |

What changed in RTen since textweaver pinned 0.26, from the `Unreleased` section of its changelog [24]: the `.rten` format is deprecated and off by default (textweaver builds with `onnx_format` and without `rten_format` in `Cargo.toml`, so nothing changes); `external_data_static` embeds a model in the binary without a copy (useful only if a voice were bundled); compile time and binary size fall by sharing operator instantiations across element types (worth taking, since RTen builds at `opt-level = 3` even in dev and build time rose "by a few minutes cold" per ADR-0023); and fixes in `ScatterND`, buffer pooling, and quantization saturation in the generic SIMD path. From 0.25 and 0.26: shape inference on by default, input validation, f16 models loaded by up-conversion, `GRU`/`LSTM` sped up, `Conv` and `BatchNormalization` fused, and "unused operator outputs are now removed during graph optimization" [24], so asking for `w_ceil` as a second output costs nothing extra, as `model.rs` assumes.

Still open: no AVX-VNNI path, so the 7-bit int8 weight patch in `crates/textweaver-dictation/src/onnx_patch.rs` stays; no f16 compute, so an f16 export saves disk but not time; `load_mmap` stays `unsafe`, so `model.rs` reads the file into memory, one copy of the model (63 MB for a medium voice) and about 0.1 s of load (ADR-0023).

**Keep RTen.** ort would buy VNNI int8 and the widest operator coverage at the price of a 20 MB C++ library, a download at build time, and a second build system on every platform. tract cannot run the graphs. burn-onnx would turn each voice into generated code. **Re-measure** after each RTen release: Piper real-time factor and first audio, Whisper base.en int8 real-time factor, and OCR detection time (the `Cargo.toml` comment records 0.11 s per page in release), with the measurement described in section 3.

## 3. Performance engineering for the speech path

### Sentence streaming versus chunking

textweaver already streams: the worker synthesizes one sentence, sends it, then the next, and the first clause of a long first sentence goes alone (`crates/textweaver-piper/src/text.rs`, `chunks`). This is the right shape: a VITS run is one shot per input, so latency scales with the chunk, and the clause tail punctuation is put back so the model keeps comma intonation across the split. Two refinements. The clause split applies only to the first sentence of an utterance (`LONG_FIRST_SENTENCE` in `text.rs`); splitting the first clause of every chunk longer than about 80 characters would cap first-audio synthesis near the 77 to 150 ms measured for short inputs, at one more model run per long sentence. And the sentence can be phonemized once and the model run on clause slices, since phoneme ids are already tracked by owner word (`phonemes.rs`, `prepare`).

### Lookahead depth versus memory

A 22,050 Hz 16-bit sentence of eight seconds is about 350 KB. Three chunks in flight is about 1 MB, nothing beside a 63 MB model. Memory is not the constraint; CPU under load is. ADR-0023 measured the real-time factor rising from 0.13 to 1.0 when five builds shared the machine, and near 1 two sentences of lookahead is the whole margin before a gap. The lookahead is a `ServiceConfig` field (`service.rs`), so the fix is adaptive, not larger: the worker reports each chunk's `elapsed` and audio length (`Chunk` in `synth.rs`), and the backend asks for one more chunk when `elapsed / audio` over the last two chunks exceeds 0.5, capped at four so a cursor move does not discard a minute of synthesis.

### First audio under 150 ms on a stop-and-restart

The restart cases for Piper are a rate or pitch change (`respeak_with_new_params`), a skip, a cursor move while reading, and an announcement inside a reading. The budget today is phonemization (a few ms with libespeak-ng; the pure-Rust port reports 606 ns to the first phoneme [34]), a model run on the first chunk (77 to 274 ms measured, ADR-0023), and rodio's 100 ms default buffer [26]. The device buffer alone eats two thirds of the target. Four changes, in order of return:

1. **Set the output buffer.** `DeviceSinkBuilder::with_buffer_size(cpal::BufferSize::Fixed(n))` in `crates/textweaver-enginehost/src/audio.rs`. At 22,050 Hz, 1,024 frames is 46 ms; rodio's upgrade notes recommend 1,024 to 2,048 for media playback [35]. Fall back to the default when the device refuses, and keep the stall watchdog (`STALL_TIMEOUT` in `service.rs`) as the safety net. Expected gain: 50 to 75 ms on every first word, for every engine that plays through the shared client (Eloquence, SAPI, DECtalk, Piper).
2. **Splice instead of restart on a rate change.** The playback client knows each queued word's first sample (`Chunk.words`). On a rate change, keep playing the current chunk, synthesize from a word at least 400 ms ahead at the new length scale, and swap the feed at that word's sample. The highlight never stops and no word repeats. This touches `service.rs` (`respeak_with_new_params` gains a backend hook), `backend.rs` (a `Resynthesize { from_word }` command to the worker), and `playback.rs` (replace queued samples from an offset). The generation rules of ADR-0003 stay: the replacement carries the same utterance id.
3. **Cache the last chunks' audio by word offset.** A resume from a cursor move inside the last sentence, or a "say again", slices the cached audio from that word's first sample instead of synthesizing. Per voice, rate, and pitch; the last four chunks, about 1.5 MB.
4. **Cache phonemes.** An LRU in `phonemes.rs` keyed by clause text, voice phoneme map, and phonemizer, holding ids and owners. Phonemization is about 0.05 of the real-time factor on the pure-Rust path (0.18 against 0.13, ADR-0023), so a cache saves about a quarter of the synthesis time on re-reads and restarts, and all of it for repeated announcements ("Next heading", "heading level 2").

### Prosody across sentence boundaries

Piper models sentence-final intonation from the trailing `.`, `!`, or `?` that `text.rs` appends, so a sentence per chunk keeps it right. The only discontinuity is the first-clause split, where the clause ends in a comma and the model does not see the rest. If listening tests on `en_US-joe-medium` find the comma pause too long, trim 30 to 60 ms of trailing silence from the first clause, measured from the last `w_ceil` frame. Piper's own program inserts `--sentence-silence` between sentences [22]; textweaver does not, and the model's own end-of-sentence silence is enough, and it halves at 2x rate, which fast readers want.

### Resampling and pitch

Pitch is the only resampling in the Piper path, and only when the pitch is not zero. The cost is 128 taps per output sample, about 2.8 million multiply-adds per second of audio, well under a millisecond with rubato's AVX or NEON kernels [36]. rubato notes that "each halving of the sinc function length nearly doubles the speed" [36], but the gain is not worth measuring until the model run shrinks. Whisper's 16 kHz resample uses rubato's FFT resampler (ADR-0023), the fastest for a fixed ratio [36]. rubato 5.0.1 (October 1, 2026) is a patch over the 5.0.0 textweaver uses. To avoid resampling, the playback client already opens the device at the voice's rate (`set_sample_rate` in `backend.rs`), so the OS mixer does the only other conversion; a Kokoro voice at 24 kHz would do the same.

### cpal directly versus rodio

rodio adds a mixer, a `Source` trait, and a sink over cpal, and textweaver uses little of it: `FeedSource` in `audio.rs` is one endless source. Going to cpal directly would remove one crate and the 100 ms default, but textweaver would then own stream callbacks, device enumeration, and the tone generator on every platform. The buffer-size call gets the latency gain without that; keep rodio.

### How to measure

Add `cargo xtask bench --speech` (or the `piper-bench` subcommand that `synth.rs` already names) in `xtask/src/bench.rs`: run `textweaver_piper::synth::measure` on a fixed 120-word paragraph with the voice named by `TEXTWEAVER_PIPER_BENCH_VOICE`, and skip with a note when none is installed. Report model load, first audio, real-time factor, and a new restart number: start a reading, change the rate at 1.5 s, and time from the command to the first new sample leaving the feed, through the silent timed output in `audio.rs` so CI never plays audio. Print, gate nothing, compare across RTen releases. The existing `open_to_first_speech_ms` stays as the application-path number.

### Formant and concatenative engines

Fast screen-reader users want a formant engine at 500 words per minute and above, with word events. The free ones:

- **eSpeak NG as a library.** textweaver loads `libespeak-ng` at run time on Linux (`docs/speech.md`). It gives word events with audio positions, and it parses SSML: `break`, `mark`, `emphasis`, `prosody` (rate, pitch, volume), `say-as`, `voice`, `s`, and `p` [37].
- **eSpeak NG as a pure-Rust port.** The `espeak-ng` crate 0.2.0 (September 5, 2026, GPL-3.0-or-later) reimplements eSpeak NG with data from 1.53.0-dev, 123 dictionaries for 151 languages, phonemization and audio synthesis, and a reported 380x real-time throughput [34]. textweaver uses it only as Piper's English phonemizer (`bundled-data-en` in `Cargo.toml`). It could become an `espeak-rs` backend on every platform, with word timing from the synthesizer itself. Known gaps per its README: prefix stripping, mid-word language switching, some number formatting [34]. Its bit-identical claim is a claim to test.
- **DECtalk.** The GitHub source carries FONIX's proprietary notice: use "authorized only pursuant to a valid written license" [38]. It stays bring-your-own, as `docs/dectalk.md` says.
- **Eloquence.** Proprietary. There is no free Eloquence.
- **RHVoice.** LGPL-2.1 core, GPL-3 with MAGE; statistical parametric, not formant; SSML, 30 or more languages; speaks through SAPI5, Speech Dispatcher, and NVDA [39]. textweaver reaches it today through the `sapi` and `speechd` backends, with those APIs' word or index events. Voice licences differ per voice (unverified).
- **Flite** (BSD-style, Carnegie Mellon [40]) and **SVOX Pico** (Apache-2.0, six languages [41]) have no SSML and no documented word events (unverified). Neither is worth a backend while eSpeak NG covers the fast-reader case.

### SSML-style pauses and emphasis

eSpeak NG parses SSML [37]; Piper has none, only `--sentence-silence` [22]; Kokoro reads punctuation and nothing else [42]; Supertonic has expression tags such as laugh and breath, not SSML [9]. Eloquence, SAPI, and DECtalk take their own control sequences through their hosts (not researched here).

textweaver can do pauses without any engine's help, because for the engines that matter it owns the clock. A pause becomes a span in the narration plan (`SpanKind` in `textweaver-core`, where "heading level 2" already lives as inserted speech), and the service turns it into a silent utterance of N milliseconds: the shared playback client (`playback.rs`) pushes N ms of zeros into the feed, which keeps the audio clock honest and pause and stop working; a timer-paced or host-owned engine waits N ms before the next `speak`. Effect on the offset map: none for the words. A silent utterance carries no source range, so `Position` reports `None` as inserted speech does, the previous word keeps its highlight, and the next utterance's `audio_ms` values start from its own first sample, as every utterance's do. Only a pause inside one utterance would shift later `audio_ms` values by N; the plan should split the utterance instead. Emphasis is a span with its own rate or pitch, a separate utterance with scaled `VoiceParams`, which is what `char_rate_scale` already does for single characters. For Piper, pitch is a resample, so emphasis is best a 15 percent longer length scale plus a 100 ms pause before the span.

## 4. Whisper dictation and OCR on the same runtime

| Model | Licence | Size | CPU real-time factor (reported) | Word timestamps |
|---|---|---|---|---|
| Whisper tiny / base / small [43] | MIT | 39M / 74M / 244M | 0.04 / 0.07 / 0.17 on an M1 with whisper.cpp [44] | via cross-attention alignment heads and DTW; sherpa-onnx's turbo export adds a `cross_attention_weights` decoder output for it [45] |
| Whisper large-v3-turbo [43] | MIT | 809M | below real time on CPU unless int4 or int8 [46] | same |
| distil-large-v3 [46] | MIT | about 756M | 6x faster than large-v3 (reported) | same |
| Moonshine [47] | MIT (streaming models); community licence for legacy non-English | tiny and base; counts unverified here | about 5x real time on CPU for the 245M streaming model (reported [48]) | not documented |
| Parakeet TDT 0.6B v3 [48] | CC-BY-4.0 (unverified here) | 600M | 10.8 to 12.9x real time on CPU via ONNX (reported) | yes, documented word and segment stamps |
| Kyutai STT [49] | CC-BY-4.0 | 1B, 2.6B | GPU examples only | yes, word level |

textweaver runs `base.en` int8 on RTen (`crates/textweaver-dictation/src/rten_whisper.rs`). Keep it: the smallest model that transcribes dictation reliably, with the 7-bit patch for AVX2 saturation. `small.en` int8 is the next step if accuracy complaints come in, at about 2.5x the time. Word timestamps are not needed for dictation; they would matter only for aligning an audiobook to its text, which is not on the roadmap. Parakeet is the one to watch if that changes: faster than Whisper on CPU with stamps built in, but 600M and exported for ONNX Runtime.

OCR: ocrs 0.13.1 (September 13, 2026; Apache-2.0 or MIT; Latin alphabet only; on RTen) [50] is what `crates/textweaver-ocr/src/ocrs_engine.rs` runs, with `paddle.rs` and `tesseract.rs` beside it. The `Cargo.toml` comment records detection at 0.11 s per page in release. PaddleOCR is Apache-2.0; PP-OCRv5 mobile ONNX exports are small (detection 5 MB, recognition 7.5 to 16 MB, reported [51]), and PP-OCRv6 claims a 5.2x CPU speedup over v5 [52]. TrOCR (62M small, 334M base) is line-level and needs a detector [53]; `rten-examples/src/trocr.rs` runs it on RTen [7]. The next OCR step is a PP-OCRv5 or v6 mobile recognizer on RTen for non-Latin scripts, measured per page against ocrs.

## 5. The twelve most valuable speech-side improvements

Ranked by expected gain per unit of work. Sizes: small is hours, medium a day or two, large a week or more.

| # | Improvement | Evidence | Touches | Size | Target | Budget | Measure |
|---|---|---|---|---|---|---|---|
| 1 | Output buffer of about 46 ms (1,024 frames at 22,050 Hz), default as fallback | rodio defaults to 100 ms [26]; `audio.rs` sets none | `crates/textweaver-enginehost/src/audio.rs` | small | alpha.8 | first audio -50 to -75 ms on every host engine and Piper; no memory or size change | restart timing in `cargo xtask bench --speech` |
| 2 | `cargo xtask bench --speech`: load, first audio, real-time factor, restart latency | `synth.rs` names a `piper-bench` that `main.rs` lacks | `xtask/src/bench.rs`, `synth.rs` (`measure`) | small | alpha.8 | none at run time | its own numbers, per RTen release |
| 3 | Phoneme cache (LRU by clause, voice map, phonemizer) | about a quarter of the pure-Rust path's time (ADR-0023); no cache today | `crates/textweaver-piper/src/phonemes.rs` | small | alpha.8 | about 100 KB per 1,000 clauses; synthesis -25% on repeats | `measure` run twice on one text |
| 4 | Keep the last four chunks' audio; resume by word sample offset | `Chunk.words` holds each word's first sample | `crates/textweaver-piper/src/backend.rs` worker | small | alpha.8 | 1.5 MB; 0 ms of synthesis on resume inside the last sentence | restart timing with a cursor move inside the sentence |
| 5 | Splice at a word boundary on a rate or pitch change | `respeak_with_new_params` restarts; the client knows word samples | `service.rs`, `backend.rs`, `playback.rs` | medium | alpha.9 | no gap; one extra model run per change, on the worker | silence between last old and first new sample, target 0 |
| 6 | Pure-Rust eSpeak NG backend with word events, all platforms | the crate synthesizes at a reported 380x real time, 151 languages [34]; used only for English phonemes today | new backend in `crates/textweaver-speech/src/backends/`, `Cargo.toml` features | medium | alpha.9 | English data already embedded, other languages as a download; real-time factor expected below 0.01 | `tw backends`; word timing against libespeak-ng on 154 words as in ADR-0023 |
| 7 | Adaptive lookahead, two to four chunks, from measured real-time factor | factor reached 1.0 under load (ADR-0023); `lookahead` is a config field | `queue.rs`, `service.rs`, Piper backend reporting `elapsed` | small | alpha.9 | up to 1.5 MB more audio; no latency change | soak test beside a build; the stall watchdog never fires |
| 8 | Pauses as silent utterances, emphasis as scaled spans | engines own the clock through the shared client; roadmap lists SSML-style pauses | `crates/textweaver-core` span kinds, `service.rs`, `playback.rs` | medium | alpha.9 | zero samples are free; words' offset maps unchanged | recording-backend tests around a pause; `tw speak --json` |
| 9 | Kokoro-82M on RTen with word timing from a duration output | Apache-2.0; RTen example [7]; kokoro-onnx export adds durations [6] | new `crates/textweaver-kokoro` sharing `phonemes.rs` and the playback path | large | later | 326 MB fp32 or 92 MB int8 in memory (reported); real-time factor expected 0.2 to 0.4 | `measure`; timing error after pauses as in ADR-0023 |
| 10 | Kitten TTS nano int8 on RTen, a 25 MB English voice | Apache-2.0; 0.12 real-time factor via ort (reported [12]); espeak-ng input | the module from 9; `crates/textweaver-piper/src/catalog.rs` | medium | later | 25 MB download; under 100 MB in memory | `measure`; a duration tensor decides word timing |
| 11 | Full eSpeak NG language data as a download for non-English Piper voices | only `bundled-data-en` is built in (`Cargo.toml`) | `phonemes.rs`, `download.rs`, data folder | medium | alpha.9 | download size to measure; binary unchanged | diff Spanish and German phonemes against libespeak-ng |
| 12 | Next RTen release, plus an int8 dynamic-quantized Piper export | compile time and binary size fall [24]; int8 kernels exist, VNNI does not | `Cargo.toml`, `model.rs`; `onnx_patch.rs` if weights saturate | medium | alpha.9 | model file -60 to -75%; AVX2 saturation risk | `measure` before and after; the 7-bit patch test |

Everything above keeps the input thread free: synthesis on the Piper worker, playback on the audio thread, the service on the speech thread (ADR-0003). Nothing adds GUI or TUI state; the only visible change is a shorter wait after a key press.

## Sources

1. OHF-Voice/piper1-gpl releases: https://github.com/OHF-Voice/piper1-gpl/releases
2. OHF-Voice/piper1-gpl repository: https://github.com/OHF-Voice/piper1-gpl
3. hexgrad/kokoro repository: https://github.com/hexgrad/kokoro
4. Kokoro pipeline timestamps (`pred_dur`, `MAGIC_DIVISOR`): https://raw.githubusercontent.com/hexgrad/kokoro/main/kokoro/pipeline.py
5. Kokoro TTS review with CPU timing: https://www.visionstory.ai/open-source/kokoro-tts
6. kokoro-onnx pull request 197, timestamps from a duration output: https://github.com/thewh1teagle/kokoro-onnx/pull/197
7. rten-examples source list (kokoro.rs, piper.rs, whisper.rs, trocr.rs): https://github.com/robertknight/rten/tree/main/rten-examples/src
8. supertone-inc/supertonic repository: https://github.com/supertone-inc/supertonic
9. Supertonic 3 local TTS notes (real-time factor 0.200 at 16 threads): https://localclaw.io/tts/supertonic-3
10. sherpa-onnx Supertonic int8 export: https://huggingface.co/csukuangfj2/sherpa-onnx-supertonic-tts-int8-2026-03-06
11. KittenML/KittenTTS repository: https://github.com/KittenML/KittenTTS
12. kitten_tts_rs (ort, espeak-ng, real-time factors): https://github.com/second-state/kitten_tts_rs
13. Matcha-TTS repository: https://github.com/shivammehta25/Matcha-TTS
14. MeloTTS repository: https://github.com/myshell-ai/MeloTTS
15. kyutai-labs/pocket-tts repository: https://github.com/kyutai-labs/pocket-tts
16. k2-fsa/ZipVoice repository: https://github.com/k2-fsa/ZipVoice
17. resemble-ai/chatterbox repository: https://github.com/resemble-ai/chatterbox
18. Open-source TTS comparison (Orpheus, Dia, Zonos sizes and licences): https://pinggy.io/blog/best_open_source_self_hosted_text_to_speech_models/
19. edwko/OuteTTS repository: https://github.com/edwko/OuteTTS
20. Open-weight TTS comparison (Kokoro ONNX sizes, XTTS, F5, Fish, Parler licences): https://builderai.tools/blog/open-weight-text-to-speech-models-2026
21. NeuTTS Air project page: https://github.com/goodtab/neutts-air
22. Piper "Insert Pause" discussion (no SSML, `--sentence-silence`): https://github.com/rhasspy/piper/discussions/199
23. k2-fsa/sherpa-onnx repository: https://github.com/k2-fsa/sherpa-onnx
24. RTen changelog: https://raw.githubusercontent.com/robertknight/rten/main/CHANGELOG.md
25. RTen README (SIMD, threads, quantization): https://github.com/robertknight/rten
26. rodio default buffer size of 100 ms (`src/stream.rs`): https://raw.githubusercontent.com/RustAudio/rodio/master/src/stream.rs
27. ort on crates.io: https://crates.io/crates/ort
28. ort releases (ONNX Runtime 1.28): https://github.com/pykeio/ort/releases
29. Arch Linux onnxruntime-cpu package size: https://www.archlinux.de/packages/x86_64/onnxruntime-cpu
30. tract issue 2928, dynamic Range in a VITS graph: https://github.com/sonos/tract/issues/2928
31. sonos/tract repository: https://github.com/sonos/tract
32. Rust inference runtimes index (wonnx archived, burn-onnx coverage): https://arewelearningyet.com/inference/
33. Lele announcement, compile ONNX to Rust: https://users.rust-lang.org/t/lele-bare-metal-ml-inference-engine-in-pure-rust-compile-onnx-into-rust/138195
34. espeak-ng-rs, pure-Rust eSpeak NG: https://github.com/eugenehp/espeak-ng-rs
35. rodio UPGRADE notes (DeviceSinkBuilder, buffer sizes): https://github.com/RustAudio/rodio/blob/master/UPGRADE.md
36. rubato README (resampler types, SIMD, sinc length): https://github.com/HEnquist/rubato
37. eSpeak NG SSML support: https://raw.githubusercontent.com/espeak-ng/espeak-ng/master/docs/markup.md
38. DECtalk LICENCE (FONIX): https://raw.githubusercontent.com/dectalk/dectalk/develop/LICENCE
39. RHVoice repository: https://github.com/RHVoice/RHVoice
40. Flite COPYING: https://raw.githubusercontent.com/festvox/flite/master/COPYING
41. SVOX Pico TTS: https://github.com/naggety/picotts
42. Kokoro prosody from punctuation only: https://deapi.ai/blog/kokoro-tts-guide-how-to-control-41-voices-with-nothing-but-punctuation
43. openai/whisper model table: https://github.com/openai/whisper
44. Whisper CPU benchmarks on Apple Silicon: https://justvoice.ai/blog/whisper-benchmark-apple-silicon-m3-m4
45. sherpa-onnx Whisper turbo int8 with cross-attention output: https://huggingface.co/cohearo/sherpa-onnx-whisper-large-v3-turbo-attention-int8
46. Whisper and distil-whisper on CPU: https://www.simplismart.ai/blog/deploy-whisper-v3-turbo-using-vox-box
47. moonshine-ai/moonshine repository: https://github.com/moonshine-ai/moonshine
48. Local STT models 2026 (Moonshine and Parakeet CPU figures): https://openvoxai.com/blog/best-local-stt-transcription-models-2026
49. Kyutai delayed-streams-modeling (STT): https://github.com/kyutai-labs/delayed-streams-modeling
50. robertknight/ocrs repository: https://github.com/robertknight/ocrs
51. kreuzberg-paddle-ocr (PP-OCRv5 ONNX sizes): https://lib.rs/crates/kreuzberg-paddle-ocr
52. PaddlePaddle/PaddleOCR repository: https://github.com/PaddlePaddle/PaddleOCR
53. TrOCR model sizes: https://github.com/microsoft/unilm/tree/master/trocr

## See also

- [Research index](README.md)
- [Speech engines and voices](../../speech.md)
- [ADR-0023: Piper voices and Whisper dictation in-process on RTen](../../adr/0023-in-process-neural-speech.md)
- [ADR-0003: Speech threading and event timing](../../adr/0003-speech-threading-and-event-timing.md)

# Research: pure-Rust choices for Wave 3

Researched on Saturday, September 26, 2026, for Agents W3d (OCR), W3e (define word), and W3f (voices and dictation). It records the versions, dates, and licences found on that day. The full source list is at the end.

## Headline

- **One runtime covers all three models.** RTen (Robert Knight's pure-Rust ONNX runtime, 0.26.0) runs ocrs (OCR), Piper voices, Whisper, and Silero voice detection. Examples for each exist in `rten-examples`.
- **tract cannot run Piper's voice models (VITS) today.** Its issues #2928 and #2931 are open, and the next blocker appears after their fixes.
- **candle can't either.** Its VITS support is in an unmerged pull request (#3792). candle-core also builds the Oniguruma C library through `tokenizers`, so it is not pure Rust.
- **ocrs reads only ASCII plus the euro sign.** That means no accented letters, curly quotes, or Greek. Its models are CC BY-SA 4.0.
- **A pure-Rust espeak-ng port exists.** The `espeak-ng` crate (0.2.0) is GPL-3.0-or-later, and it can phonemize text for Piper when libespeak-ng is missing.
- **Piper can give word timing.** The voice graph's `w_ceil` tensor holds frames per phoneme. Multiply by 256 to get samples. Piper's own "alignments" feature uses the same tensor.
- **earshot is a pure-Rust voice-activity detector** with its model built in, about 95 KiB, so nothing needs downloading.
- **Open English WordNet 2025** (CC BY 4.0) is the maintained successor to Princeton WordNet 3.1.

## OCR (W3d)

### The plan

- **Primary engine: ocrs** 0.13.1 on rten 0.26.0.
  - Render only the pages that have no text layer, with **hayro** 0.7.1. It is pure Rust, by krilla's author, and includes JBIG2, CCITT, and JPEG 2000 decoders.
  - Try the cheap path first: a scanned page is usually one image. Extract it with lopdf and decode it, instead of rendering the page.
- **Worth trying: PaddleOCR PP-OCRv5 Latin models through rten.** They cover about 45 Latin-script languages with accents (Apache-2.0). We haven't checked whether rten supports every operator they use.
- **Fallback: a Tesseract subprocess.** Route to it automatically for non-English text, or when accents are expected.

### Things to know

- **Build speed.** Build `rten*` and `ocrs` at `opt-level = 3` in the dev profile. In debug builds, detection takes 4.8 s against 0.11 s in release.
- **Layout.** Reading order comes from heuristics. There are no paragraphs, headings, or tables, and rotated pages are not handled.
- **Safety.** Avoid rten's `load_mmap`, because it is unsafe.
- **Models to download** (the user confirms first, and each file is checked by SHA-256):
  - `text-detection.onnx`: 2.5 MB.
  - `text-recognition.onnx`: 9.7 MB.
  - Licence: CC BY-SA 4.0. They must be credited, and they are one-way compatible with GPLv3.

## Piper voices (W3f)

### The plan

- **Primary runtime: rten** 0.26.0, running Piper's `.onnx` file directly. It is based on `rten-examples/src/piper.rs`, and in 2024 its author measured it at about 1.3 times slower than ONNX Runtime.
- **Fallbacks:**
  - `ort` 2.0.0-rc.13 behind a feature. It is not pure Rust.
  - The piper subprocess, or libpiper (GPL-3.0) loaded at run time.
  - Revisit tract and candle once their pull requests merge.
- **Phonemization.**
  - Use our libespeak-ng loader (`espeak_TextToPhonemes`, IPA mode). We add the clause punctuation ourselves.
  - When the library is missing, fall back to the pure-Rust `espeak-ng` crate (`default-features = false, features = ["bundled-data-en"]`).
  - Test that both give the same phonemes for each language.
  - Voices that use Japanese, Thai, Chinese, Hebrew, or Lithuanian phonemizers need the subprocess.
- **Word timing.** Read the `w_ceil` node with rten's `run` and a `node_id`. If graph optimization fuses the node away, turn optimization off, or patch the graph once when the voice is downloaded. `tw backends` reports the timing as "from model" or "estimated".

### Speed

- Published real-time factors on ONNX Runtime are 0.12 to 0.22 on small CPUs.
- The estimate for rten on the owner's desktop is 0.1 to 0.3.
- First audio should come 100 to 300 ms after starting, if the first clause is spoken as its own chunk and the model stays loaded.
- W3f must measure these.

### Voice catalogue

- `voices.json` (246 KB) lists 177 voices in 53 languages.
- File sizes: x_low 21 to 28 MB, low 28 to 70 MB, medium 63 to 79 MB, high 63 to 137 MB.
- Get SHA-256 sums from the Hugging Face tree API (`lfs.oid`).
- **Licences differ by voice**, so read each voice's `MODEL_CARD` and show the licence before downloading:
  - `lessac`: non-commercial.
  - `ryan` and `hfc_female`: CC BY-NC-SA.
  - `libritts_r`: CC BY.
  - `joe`: CC0.
  - `kristin`, `norman`, and `cori`: public domain.
  - Never bundle a non-commercial (NC) voice.

## Whisper dictation (W3f)

### The plan

- **Primary runtime: rten**, using the ONNX int8 models from onnx-community. It is the same runtime as OCR and Piper, and `rten-examples/src/whisper.rs` is complete.
- **Fallbacks:** candle-transformers 0.11.0 (it builds the Oniguruma C library), then today's whisper.cpp subprocess.
- **Changed from the brief:** the brief named candle. rten is the purer choice, so W3f records the switch in an ADR.
- **Audio input:**
  - Capture: rodio 0.22's `recording` feature (a microphone source, with cpal underneath).
  - Resampling: rubato 5.0.0.
  - Voice activity detection: **earshot** 1.2.2 (16 ms frames, model built in). Silero VAD through rten is the fallback.
- **Streaming.** Whisper does not stream. Split speech into utterances with the voice detector, and transcribe each one after about 600 ms of silence.

### Models to download

- tiny.en int8: 40.8 MB.
- base.en int8: 76.9 MB.
- small.en int8: 249 MB.
- Each also needs `tokenizer.json` (2.4 MB).
- Licence: MIT (OpenAI).

## Define word (W3e)

### The plan

- **Definitions:** Open English WordNet 2025.
  - Licence: CC BY 4.0.
  - Download: the WNDB zip, 9.6 MB.
  - Size: 107,519 synsets.
  - Princeton WordNet 3.1 (16.4 MB, WordNet licence) is the alternative.
- **Pronunciations:** CMUdict.
  - File: `cmudict.dict`, 3.6 MB.
  - Licence: BSD-style.
- **Data format.** Build one compact file with a script in `tools/`: headwords to offsets in `fst` 0.4.7, with zstd blocks read by `ruzstd` 0.9.0. Parse the data ourselves; the WordNet crates are weak.
- **Word forms.** Add a "morphy" step, which turns "running" into "run" (a port of NLTK's).
- **Not Wiktionary.** The kaikki.org extract is 3.1 GB and CC BY-SA, so leave it out.
- **Licences.** All of these can ship inside GPL-3.0-or-later software, with notices.

## Other pure-Rust swaps

- **Adopt:**
  - `icu_segmenter` 2.3.0 for word and sentence boundaries: 1.3 to 3.9 times faster on graphemes, with dictionary-based word breaks for Thai, Lao, Khmer, Burmese, Chinese, and Japanese.
  - `calamine` 0.36.1 for XLSX, XLS, and ODS.
  - `sevenz-rust2` 0.23.0 and `tar` 0.4.46 for archives.
  - The zip crate's pure-Rust `deflate-flate2-zlib-rs`, `lzma`, `xz`, `bzip2-rs`, and `ppmd` features.
  - `memchr::memmem::Finder` for literal find, and aho-corasick for highlighting many terms at once.
- **Keep:**
  - pulldown-cmark and comrak (already pure Rust).
  - rodio and cpal.
  - ropey 1.6: its O(1) clone matters to the writer thread. Watch ropey 2.0.
- **Optional:** `rustls-graviola` 0.4.0 replaces ring's C code (Rust plus assembly, on x86_64 and aarch64), with ring as the fallback.

## Crates to add

- `rten`, `rten-tensor`, and `rten-imageproc`: 0.26.0.
- `ocrs`: 0.13.1, with `default-features = false, features = ["onnx"]`.
- `hayro`: 0.7.1.
- `espeak-ng`: 0.2.0, with `bundled-data-en`.
- `earshot`: 1.2.2.
- `rubato`: 5.0.0.
- `fst`: 0.4.7.
- `ruzstd`: 0.9.0.
- `calamine`: 0.36.1.
- `sevenz-rust2`: 0.23.0.
- `tar`: 0.4.46.
- `icu_segmenter`: 2.3.0.

Every licence is permissive or GPL-compatible. `icu_segmenter` is Unicode-3.0, and `espeak-ng` is GPL-3.0-or-later, the same as textweaver.

## Downloads that need the owner's approval

- **Build time:** crates from crates.io. The largest is `icu_segmenter`, at 4.2 MB compressed.
- **Run time:** users confirm each download in the app, and every file is checked by SHA-256.
  - ocrs models: 12.2 MB.
  - Piper voices: 21 to 137 MB each.
  - Whisper models: 41 to 249 MB.
  - Tesseract data: only for the fallback.
- **One-time data build** for define word, run by the orchestrator:
  - Open English WordNet 2025: 9.6 MB.
  - CMUdict: 3.6 MB.

## Sources

- [ocrs](https://github.com/robertknight/ocrs), [ocrs models](https://huggingface.co/robertknight/ocrs), [rten](https://github.com/robertknight/rten)
- [Piper on rten](https://github.com/rhasspy/piper/discussions/504), [piper1-gpl](https://github.com/OHF-Voice/piper1-gpl), [Piper voices](https://huggingface.co/rhasspy/piper-voices)
- [hayro](https://github.com/LaurenzV/hayro), [espeak-ng-rs](https://github.com/eugenehp/espeak-ng-rs), [earshot](https://github.com/pykeio/earshot)
- [tract #2928](https://github.com/sonos/tract/issues/2928), [candle](https://github.com/huggingface/candle) (PR #3792)
- [onnx-community Whisper](https://huggingface.co/onnx-community), [whisper-rs](https://codeberg.org/tazz4843/whisper-rs)
- [Open English WordNet](https://github.com/globalwordnet/english-wordnet), [CMUdict](https://github.com/cmusphinx/cmudict), [kaikki.org](https://kaikki.org/dictionary/rawdata.html)
- [FSF licence list](https://www.gnu.org/licenses/license-list.html), [zlib-rs performance](https://trifectatech.org/blog/zlib-rs-is-faster-than-c/), [grapheme segmenter benchmark](https://github.com/cometkim/fast-grapheme-segmenter)

## See also

- [Xilem GUI research](xilem-gui.md)
- [Roadmap](../roadmap.md)
- [Documentation index](../README.md)

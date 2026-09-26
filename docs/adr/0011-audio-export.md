# ADR-0011: Audio export

- Status: accepted
- Date: 2026-09-25
- Status update (Saturday, September 26, 2026): Implemented. DECtalk also gives exact word timings (ADR-0021). `tw export-audio` now reads the voice, rate, pitch, volume, preferred engine, table and footnote modes, and the `[export]` settings from `settings.toml` (Agent D4).

## Context

Star exported audio three ways (`docs/star-parity.md` Part 2 §5.D, `star/tts/audio.py`, `star/audiobook.py`): the whole normalized text synthesized in one call to a WAV, then converted with ffmpeg (or pydub) to MP3, OGG, or MP4; an M4B audiobook synthesized chapter by chapter, joined by ffmpeg's concat demuxer with chapter metadata; and subtitles (`star/tts/subtitles.py`) built by spreading the whole file's duration over its whitespace tokens by length (`len + 1`), grouped into caption lines of at most 12 words and 90 characters, broken at sentences.

Three problems follow. Apportioning one duration over a whole document drifts: a long word, a pause, a number read as six words, or a heading announcement pushes every later cue away from its audio, and the error grows to minutes in a book. Engines that know exactly where each word sounds (espeak-ng's `audio_position`, ECI index marks, SAPI word-boundary offsets) had that knowledge thrown away. And export needed a backend-specific path per engine.

## Decision

- **One pipeline over the backend trait.** `textweaver-export` reads any document through any backend with `Caps::SYNTH_TO_FILE`. A new default trait method, `SpeechBackend::synthesize_utterance(&Utterance, &Path) -> FileSynthesis`, writes one utterance to a WAV file and returns word timings (`WordTiming { byte_range, audio_ms }`) when the engine knows them; its default calls `synthesize_to_file` and returns none. espeak-ng implements it from retrieval-mode word events; the recording test double writes deterministic silence (250 ms per word) with exact timings. ECI, SAPI, and DECtalk override it from their hosts' index marks and word-boundary events: the utterance is captured through the engine host, and each word's sample offset in that capture becomes its time in the file (`textweaver_enginehost::word_timings`), so the cue is exact on the file's own clock. SAPI voices without word timing (Code Factory's Eloquence) report none.
- **Utterance by utterance.** The document is planned with `text::narrate::plan` exactly as for reading aloud (structure announcements included), each utterance is normalized with the same pipeline the speech service uses (skipping what an engine normalizes natively), synthesized to its own file in a private temporary folder, and appended to one output WAV. The running sample count gives each sentence its exact start and end, and each reported word its time, on the output's clock: the `Timeline`. An optional gap (default 0) can separate utterances. The WAV writer keeps the first piece's `fmt ` chunk byte for byte and refuses a piece in another format; the RIFF sizes are patched at the end; 4 GB is the limit.
- **Subtitles from the timeline.** Star's cue rules are kept (12 words, 90 characters, breaks at sentences, `len + 1` weighting, SRT and WebVTT formats, zero-length cues stretched to 50 ms, Star's test vectors ported), but the weighting only spreads a sentence's own measured duration over its tokens, and tokens take engine word times when there are any. Captions show the document's text, not the normalized spoken text: word timings are mapped through each utterance's offset map, so "$5" read as "five dollars" is one word cue showing "$5", and spoken-only text ("heading level 1") has no caption. Word-level cues (Star's `subtitle_word_level`) follow the engine's words when it reports them.
- **Chapters from structure.** Every `SectionBreak` marker and every `Heading` up to a level (default 6, Star's "every heading") starts a chapter at the time the reading first reaches it; a section that opens with a heading is one chapter titled by the heading; text before the first chapter is a chapter titled after the document (Star's rule); chapters without audio of their own are dropped.
- **ffmpeg for MP3 and M4B, when present.** ffmpeg is found through `TEXTWEAVER_FFMPEG` or `PATH`, never downloaded or bundled. The WAV and an `;FFMETADATA1` file (title, album, artist, `genre=Audiobook`, one `[CHAPTER]` per chapter in milliseconds) go to `ffmpeg -map_metadata 1 -map_chapters 1`: MP3 with LAME at VBR quality 2 (ID3 chapters included), M4B as 64 kbit/s AAC in MP4 with `+faststart` (Star's settings). Without ffmpeg, MP3 and M4B fail with a sentence saying so before any synthesis starts; WAV always works. Intermediate files live in a temporary folder beside the output and are removed.
- **Command line.** `tw export-audio FILE --out out.wav|mp3|m4b [--subtitles out.srt|vtt] [--word-level] [--backend --voice --rate --pitch] [--json] [--quiet]`. Without `--backend`, the highest-priority available backend that can write files is chosen (engines that only play, such as Omnivox and speech-dispatcher, are passed over). Progress is reported in whole tens of percent, one short sentence each, so a screen reader is not flooded; the result is one sentence (length, sentences, chapters, voice). Cancellation between sentences is part of the library API for the GUI.

## Consequences

- Cue times no longer drift: every sentence cue starts and ends where its audio does, measured, and word cues are exact for engines that report word times.
- Export costs one engine call per utterance. For engines with per-call start-up cost (host processes) this is slower than one large call, but it is what makes timing exact, bounds memory, and lets the user cancel.
- Engines that cannot write files (Omnivox's write-only protocol, speech-dispatcher) cannot export; `tw backends` lists which can ("audio files").
- Measured on 2026-09-25 on Windows with Microsoft David through SAPI: a two-heading sample exported to M4B, and `ffprobe -show_chapters` listed both chapters at the times of their headings.
- Not done here: cover art (Star read it from document metadata), OGG and MP4 targets, and video export with burned-in subtitles (Star's `video.subtitles = "burn"`, never implemented there either).

## See also

- [Audio export](../audio-export.md): the user guide to `tw export-audio`.
- [Settings](../settings.md#export): the `[export]` settings.
- [Documentation index](../README.md)

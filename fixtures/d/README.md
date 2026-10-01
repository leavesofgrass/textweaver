# Fixtures for the app and TUI

- `stream-note`, `stream-long`, `stream-short` (`.txt`, `.wav`, `.srt`):
  speech for the streaming dictation spike
  (`crates/textweaver-dictation/examples/stream_probe.rs`). Written with
  textweaver's own audio export and Windows SAPI5, never recorded and
  never played: `tw export-audio stream-note.txt --out stream-note.wav
  --subtitles stream-note.srt --word-level --backend sapi --rate 160`.
  `stream-note` is dictation with a pause after each sentence;
  `stream-long` is one 21-second sentence with no pause long enough to end
  an utterance; `stream-short` is three short commands. The word cues are
  the export's estimates, spread over each sentence's audio. Only the
  `.txt` files are committed (the repository ignores `*.wav`); make the
  `.wav` and `.srt` pairs with the command above before running the probe,
  with `TEXTWEAVER_HOME` pointed at a scratch folder so no settings are
  read or written.
- `reading.txt`: plain prose for the scripted TUI test
  (`crates/textweaver-tui/tests/it/scripted.rs`). Plain words and sentences
  ending in ". " so every word and sentence segmentation agrees on its
  boundaries; one paragraph is longer than 80 columns so the viewport wraps.

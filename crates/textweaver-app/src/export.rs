//! Audio export defaults from `[export]`, for `tw export-audio` and later
//! the GUI's export dialog.

use std::path::{Path, PathBuf};

use textweaver_store::Settings;

/// Which subtitles an audio export writes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubtitlePlan {
    /// The subtitle file, if any.
    pub path: Option<PathBuf>,
    /// One cue per word instead of caption lines.
    pub word_level: bool,
}

/// The subtitles for exporting audio to `out`: the file asked for, else,
/// with `[export] subtitles_with_audio`, one beside `out` named like it
/// with `subtitle_format`'s extension (`book.mp3` gives `book.srt`).
/// Word-level cues when asked for or when `subtitle_word_level` is on.
pub fn subtitle_plan(
    settings: &Settings,
    out: &Path,
    asked: Option<&Path>,
    word_level: bool,
) -> SubtitlePlan {
    let ex = &settings.export;
    let path = asked.map(Path::to_path_buf).or_else(|| {
        ex.subtitles_with_audio
            .then(|| out.with_extension(ex.subtitle_format.extension()))
    });
    SubtitlePlan {
        path,
        word_level: word_level || ex.subtitle_word_level,
    }
}

#[cfg(test)]
mod tests {
    use textweaver_store::SubtitleFormat;

    use super::*;

    #[test]
    fn subtitles_follow_the_flags_then_the_settings() {
        let out = Path::new("book.mp3");
        let mut s = Settings::default();
        assert_eq!(
            subtitle_plan(&s, out, None, false),
            SubtitlePlan {
                path: None,
                word_level: false
            }
        );
        s.export.subtitles_with_audio = true;
        assert_eq!(
            subtitle_plan(&s, out, None, false).path,
            Some(PathBuf::from("book.srt"))
        );
        s.export.subtitle_format = SubtitleFormat::Vtt;
        s.export.subtitle_word_level = true;
        let p = subtitle_plan(&s, out, None, false);
        assert_eq!(p.path, Some(PathBuf::from("book.vtt")));
        assert!(p.word_level);
        // A file asked for wins.
        let p = subtitle_plan(&s, out, Some(Path::new("x.srt")), false);
        assert_eq!(p.path, Some(PathBuf::from("x.srt")));
        let s = Settings::default();
        assert!(subtitle_plan(&s, out, None, true).word_level);
    }
}

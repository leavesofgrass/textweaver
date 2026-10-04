//! The read-along page: one offline HTML file holding the document as real
//! text, its MP3 inside as base64, and the timeline, with a small script
//! that marks the sentence and the word being heard.
//!
//! The text stays text, so a screen reader, a Braille display, zoom, reflow
//! and the reader's own fonts all work on it. The script never moves focus
//! and has no live region: a screen reader reads where its own cursor is
//! while the audio plays. Without script the page still shows the text and
//! the browser's own audio control.
//!
//! The spoken sentence is underlined (and tinted with the theme's spoken
//! sentence colors); the spoken word takes the theme's spoken word style
//! (reversed and bold in every built-in theme), plus an underline and a
//! 2 px outline, so it never relies on color alone. Under forced colors the
//! word uses `Highlight` and `HighlightText`, and the sentence a
//! `CanvasText` underline. With reduced motion, following the audio jumps
//! instead of scrolling smoothly.

use std::fmt::Write as _;
use std::ops::ControlFlow;
use std::path::Path;

use textweaver_speech::SpeechBackend;
use textweaver_text::Document;
use textweaver_writers::sync::{self, SyncSentence};

use crate::{ExportError, ExportOptions, ExportReport, Progress, SubtitleRequest, Timeline};

/// The words on the page's controls, in the interface language.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageLabels {
    /// The skip link: "Skip to the text".
    pub skip: String,
    /// The controls' region: "Audio".
    pub controls: String,
    /// The play button.
    pub play: String,
    /// The play button while playing.
    pub pause: String,
    /// "Back a sentence".
    pub back: String,
    /// "Forward a sentence".
    pub forward: String,
    /// The toggle: "Follow along" (scroll the text with the audio).
    pub follow: String,
    /// The speed list's label: "Speed".
    pub speed: String,
    /// The contents list's label: "Contents".
    pub contents: String,
    /// A chapter's button, with `{title}`: "Play section: {title}".
    pub play_section: String,
}

impl Default for PageLabels {
    fn default() -> Self {
        PageLabels {
            skip: "Skip to the text".into(),
            controls: "Audio".into(),
            play: "Play".into(),
            pause: "Pause".into(),
            back: "Back a sentence".into(),
            forward: "Forward a sentence".into(),
            follow: "Follow along".into(),
            speed: "Speed".into(),
            contents: "Contents".into(),
            play_section: "Play section: {title}".into(),
        }
    }
}

/// How the page looks and speaks.
#[derive(Clone, Debug, Default)]
pub struct PageOptions {
    /// The theme's properties and highlight classes, such as
    /// `textweaver_theme::css::single_stylesheet` of the reader's theme;
    /// `None` uses the default (Galaxy, following the system's light or
    /// dark setting).
    pub theme_css: Option<String>,
    /// The page's language (`lang`), such as `en`; empty means `en`.
    pub lang: String,
    /// The title when the document has none.
    pub fallback_title: Option<String>,
    /// The words on the controls.
    pub labels: PageLabels,
}

/// The rules the read-along page adds to the theme and page stylesheet.
const PAGE_CSS: &str = include_str!("readalong.css");
/// The page's script.
const PAGE_JS: &str = include_str!("readalong.js");

/// The sentences of `timeline` as the writers' sync input.
pub fn sync_sentences(timeline: &Timeline) -> Vec<SyncSentence> {
    timeline
        .sentences
        .iter()
        .map(|s| SyncSentence {
            range: s.source,
            words: s.words.iter().map(|w| w.source).collect(),
        })
        .collect()
}

/// The table the page's script searches: sentence `n` is `s[n]`, word `k`
/// (numbered over all sentences in order) is `w[k]`, each `[start, end]`
/// in ms; chapters are `[title, start]`.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct SyncTable {
    /// Total length, in ms.
    pub d: u64,
    /// Sentences: start and end, in ms.
    pub s: Vec<[u64; 2]>,
    /// Words: start and end, in ms.
    pub w: Vec<[u64; 2]>,
    /// Chapters: title and start in ms.
    pub c: Vec<(String, u64)>,
}

impl SyncTable {
    /// The table for `timeline`.
    pub fn new(timeline: &Timeline) -> Self {
        SyncTable {
            d: timeline.duration_ms,
            s: timeline
                .sentences
                .iter()
                .map(|s| [s.start_ms, s.end_ms])
                .collect(),
            w: timeline
                .sentences
                .iter()
                .flat_map(|s| s.words.iter().map(|w| [w.start_ms, w.end_ms]))
                .collect(),
            c: timeline
                .chapters
                .iter()
                .map(|c| (c.title.clone(), c.start_ms))
                .collect(),
        }
    }

    /// The entry of `list` playing at `ms`, as the page's script finds it:
    /// the last one starting at or before `ms`, if `ms` is before its end.
    pub fn find(list: &[[u64; 2]], ms: u64) -> Option<usize> {
        let i = list.partition_point(|e| e[0] <= ms).checked_sub(1)?;
        (ms < list[i][1]).then_some(i)
    }
}

/// Standard base64 with padding.
pub fn base64(data: &[u8]) -> String {
    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(char::from(ABC[(n >> (18 - 6 * i) & 63) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            c => o.push(c),
        }
    }
    o
}

/// The page for `doc` read as `timeline`, with `mp3` inside.
pub fn page(doc: &Document, timeline: &Timeline, mp3: &[u8], opts: &PageOptions) -> String {
    let l = &opts.labels;
    let title = doc
        .meta
        .title
        .clone()
        .or_else(|| timeline.title.clone())
        .or_else(|| opts.fallback_title.clone())
        .unwrap_or_else(|| "Untitled".to_owned());
    let lang = if opts.lang.trim().is_empty() {
        "en"
    } else {
        opts.lang.trim()
    };
    let theme = opts
        .theme_css
        .clone()
        .unwrap_or_else(textweaver_theme::css::default_stylesheet);
    let table = SyncTable::new(timeline);
    // `</` cannot appear inside the script element's JSON.
    let json = serde_json::to_string(&table)
        .unwrap_or_else(|_| "{}".to_owned())
        .replace("</", "<\\/");
    let body = sync::body(doc, &sync_sentences(timeline));
    let mut h = String::with_capacity(body.len() + mp3.len() * 4 / 3 + 16_384);
    let _ = write!(
        h,
        "<!DOCTYPE html>\n<html lang=\"{lang}\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <meta name=\"generator\" content=\"textweaver {ver}\">\n\
         <title>{title}</title>\n<style>\n{theme}{base}{page}</style>\n</head>\n<body>\n\
         <a class=\"skip-link\" href=\"#text\">{skip}</a>\n\
         <header class=\"tw-bar\">\n<h1 class=\"tw-title\">{title}</h1>\n\
         <audio id=\"tw-audio\" controls preload=\"metadata\" src=\"data:audio/mpeg;base64,{audio}\"></audio>\n\
         <div id=\"tw-controls\" role=\"group\" aria-label=\"{controls}\" hidden>\n\
         <button type=\"button\" id=\"tw-play\" data-play=\"{play}\" data-pause=\"{pause}\">{play}</button>\n\
         <button type=\"button\" id=\"tw-back\">{back}</button>\n\
         <button type=\"button\" id=\"tw-forward\">{forward}</button>\n\
         <button type=\"button\" id=\"tw-follow\" aria-pressed=\"true\">{follow}</button>\n\
         <label for=\"tw-speed\">{speed}</label>\n<select id=\"tw-speed\">",
        ver = env!("CARGO_PKG_VERSION"),
        title = esc(&title),
        base = textweaver_render::template::STYLESHEET,
        page = PAGE_CSS,
        skip = esc(&l.skip),
        audio = base64(mp3),
        controls = esc(&l.controls),
        play = esc(&l.play),
        pause = esc(&l.pause),
        back = esc(&l.back),
        forward = esc(&l.forward),
        follow = esc(&l.follow),
        speed = esc(&l.speed),
    );
    for (v, name) in [
        ("0.75", "0.75"),
        ("1", "1"),
        ("1.25", "1.25"),
        ("1.5", "1.5"),
        ("1.75", "1.75"),
        ("2", "2"),
    ] {
        let sel = if v == "1" { " selected" } else { "" };
        let _ = write!(h, "<option value=\"{v}\"{sel}>{name}</option>");
    }
    h.push_str("</select>\n");
    if table.c.len() > 1 {
        let _ = write!(
            h,
            "<nav aria-labelledby=\"tw-contents\"><h2 id=\"tw-contents\">{}</h2>\n<ul>\n",
            esc(&l.contents)
        );
        for (title, start) in &table.c {
            let name = l.play_section.replace("{title}", title);
            let _ = writeln!(
                h,
                "<li><button type=\"button\" data-at=\"{start}\">{}</button></li>",
                esc(&name)
            );
        }
        h.push_str("</ul>\n</nav>\n");
    }
    let _ = write!(
        h,
        "</div>\n</header>\n<main id=\"text\" tabindex=\"-1\">\n{body}</main>\n\
         <script type=\"application/json\" id=\"tw-timeline\">{json}</script>\n\
         <script>\n{PAGE_JS}</script>\n</body>\n</html>\n"
    );
    h
}

/// Exports `doc` as a read-along page at `out` (an `.html` file): the
/// audio is written as MP3 to a temporary file (as [`crate::export`] writes
/// it, with `subtitles_to` beside the page when asked), then the page is
/// written in one go. The report's `out` is the page; its format is MP3,
/// the audio inside.
#[allow(clippy::too_many_arguments)]
pub fn export_page(
    doc: &Document,
    backend: &mut dyn SpeechBackend,
    out: &Path,
    subtitles_to: Option<&SubtitleRequest>,
    ffmpeg: Option<&Path>,
    opts: &ExportOptions,
    page_opts: &PageOptions,
    progress: &mut dyn FnMut(Progress) -> ControlFlow<()>,
) -> Result<ExportReport, ExportError> {
    let io = |path: &Path, e: std::io::Error| ExportError::Io {
        path: path.to_owned(),
        message: e.to_string(),
    };
    let dir = out.parent().filter(|p| !p.as_os_str().is_empty());
    let tmp = match dir {
        Some(d) => tempfile::Builder::new()
            .prefix(".tw-read-along")
            .tempdir_in(d),
        None => tempfile::Builder::new().prefix(".tw-read-along").tempdir(),
    }
    .map_err(|e| io(out, e))?;
    let mp3_path = tmp.path().join("audio.mp3");
    let mut report = crate::export(
        doc,
        backend,
        &mp3_path,
        subtitles_to,
        ffmpeg,
        opts,
        progress,
    )?;
    let mp3 = std::fs::read(&mp3_path).map_err(|e| io(&mp3_path, e))?;
    let html = page(doc, &report.timeline, &mp3, page_opts);
    std::fs::write(out, html).map_err(|e| io(out, e))?;
    report.out = out.to_owned();
    Ok(report)
}

/// True when `path` names a read-along page (`.html` or `.htm`).
pub fn is_page(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("html") || e.eq_ignore_ascii_case("htm"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TimedSentence, TimedWord};
    use textweaver_core::{CharPos, CharRange};

    fn r(a: usize, b: usize) -> Option<CharRange> {
        Some(CharRange::new(CharPos(a), CharPos(b)))
    }

    fn word(a: u64, b: u64, src: Option<CharRange>, text: &str) -> TimedWord {
        TimedWord {
            start_ms: a,
            end_ms: b,
            source: src,
            caption: 0..0,
            text: text.into(),
        }
    }

    /// "Plants use light. Roots drink." on a fixed clock.
    fn fixture() -> (Document, Timeline) {
        let doc = Document::from_plain_text("Plants use light. Roots drink.");
        let t = Timeline {
            sentences: vec![
                TimedSentence {
                    start_ms: 0,
                    end_ms: 1500,
                    source: r(0, 17),
                    text: "Plants use light.".into(),
                    spoken: "Plants use light.".into(),
                    words: vec![
                        word(0, 500, r(0, 6), "Plants"),
                        word(500, 900, r(7, 10), "use"),
                        word(900, 1500, r(11, 16), "light"),
                    ],
                },
                TimedSentence {
                    start_ms: 1500,
                    end_ms: 2500,
                    source: r(18, 30),
                    text: "Roots drink.".into(),
                    spoken: "Roots drink.".into(),
                    words: vec![
                        word(1500, 2000, r(18, 23), "Roots"),
                        word(2000, 2500, r(24, 29), "drink"),
                    ],
                },
            ],
            duration_ms: 2500,
            chapters: vec![
                crate::Chapter {
                    title: "Light".into(),
                    source_start: CharPos(0),
                    start_ms: 0,
                    end_ms: 1500,
                },
                crate::Chapter {
                    title: "Water & roots".into(),
                    source_start: CharPos(18),
                    start_ms: 1500,
                    end_ms: 2500,
                },
            ],
            title: None,
            author: None,
        };
        (doc, t)
    }

    #[test]
    fn base64_matches_the_standard_alphabet_and_padding() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64(&[0xff, 0xfe, 0x00]), "//4A");
    }

    #[test]
    fn the_page_holds_the_text_spans_timings_controls_and_theme() {
        let (doc, t) = fixture();
        let opts = PageOptions {
            fallback_title: Some("essay".into()),
            ..PageOptions::default()
        };
        let html = page(&doc, &t, b"ID3fake", &opts);
        // The text, as text.
        assert!(html.contains("<title>essay</title>"));
        assert!(html.contains(">light</span>."), "{html}");
        // A span per sentence and per word.
        for n in 0..2 {
            assert!(html.contains(&format!("id=\"s{n}\"")), "s{n}");
        }
        for k in 0..5 {
            assert!(html.contains(&format!("id=\"w{k}\"")), "w{k}");
        }
        // The JSON times equal the timeline.
        let json = html
            .split("id=\"tw-timeline\">")
            .nth(1)
            .and_then(|s| s.split("</script>").next())
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(v["d"], 2500);
        assert_eq!(v["s"][1], serde_json::json!([1500, 2500]));
        assert_eq!(v["w"][2], serde_json::json!([900, 1500]));
        assert_eq!(v["c"][1][0], "Water & roots");
        // Named buttons, a labeled speed list, and the follow toggle.
        for name in [
            ">Play</button>",
            ">Back a sentence</button>",
            ">Forward a sentence</button>",
            "aria-pressed=\"true\">Follow along</button>",
            "<label for=\"tw-speed\">Speed</label>",
            ">Play section: Water &amp; roots</button>",
        ] {
            assert!(html.contains(name), "{name}");
        }
        // The controls only appear with script; the audio and text always.
        assert!(html.contains("role=\"group\" aria-label=\"Audio\" hidden"));
        assert!(html.contains("<audio id=\"tw-audio\" controls"));
        assert!(html.contains(&format!("base64,{}\"", base64(b"ID3fake"))));
        // The theme's highlight classes and the page rules.
        assert!(html.contains(".tw-spoken-word"));
        assert!(html.contains("forced-colors: active"));
        assert!(html.contains("prefers-reduced-motion"));
        // No live region, and nothing fetched from elsewhere.
        assert!(!html.contains("aria-live"));
        for bad in ["http://", "https://", "<link", " src=\"//", "@import"] {
            assert!(!html.contains(bad), "{bad}");
        }
    }

    #[test]
    fn each_word_is_found_at_its_start_plus_one_ms() {
        let (_, t) = fixture();
        let table = SyncTable::new(&t);
        for (k, w) in table.w.iter().enumerate() {
            assert_eq!(SyncTable::find(&table.w, w[0] + 1), Some(k));
        }
        for (n, s) in table.s.iter().enumerate() {
            assert_eq!(SyncTable::find(&table.s, s[0] + 1), Some(n));
        }
        assert_eq!(SyncTable::find(&table.w, 2500), None);
    }

    #[test]
    fn the_chosen_theme_replaces_the_default() {
        let (doc, t) = fixture();
        let opts = PageOptions {
            theme_css: Some(":root { --tw-text: #123456; }\n".into()),
            lang: "de".into(),
            ..PageOptions::default()
        };
        let html = page(&doc, &t, b"", &opts);
        assert!(html.contains("--tw-text: #123456;"));
        assert!(html.contains("<html lang=\"de\">"));
    }

    #[test]
    fn exports_a_page_with_the_recording_double() {
        let dir = tempfile::tempdir().unwrap();
        let doc = Document::from_plain_text("Plants use light. Roots drink.");
        let (mut backend, _rec) = textweaver_speech::RecordingBackend::new();
        let out = dir.path().join("essay.html");
        let report = export_page(
            &doc,
            &mut backend,
            &out,
            None,
            None,
            &ExportOptions::default(),
            &PageOptions::default(),
            &mut |_| ControlFlow::Continue(()),
        );
        match report {
            Ok(r) => {
                assert_eq!(r.out, out);
                let html = std::fs::read_to_string(&out).unwrap();
                assert!(html.contains("data:audio/mpeg;base64,"));
                assert!(html.contains("id=\"s0\""));
                // Only the page is left beside it.
                let left: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
                assert_eq!(left.len(), 1);
            }
            // Without the in-process encoder, MP3 needs ffmpeg.
            Err(ExportError::NoFfmpeg(_)) => assert!(!cfg!(feature = "mp3")),
            Err(e) => panic!("{e}"),
        }
    }
}

//! Whisper programs: finding them, their command lines, and reading their
//! output.
//!
//! Three command-line programs are supported (ADR-0013):
//! - whisper.cpp's `whisper-cli` (older builds: `whisper-cpp`), which needs
//!   a `ggml-<model>.bin` file and 16 kHz WAV input;
//! - `whisper-ctranslate2`, the command line of faster-whisper;
//! - `whisper`, the command line of openai-whisper (what star used through
//!   Python), which shares its options with `whisper-ctranslate2`.
//!
//! Each prints one line per segment as it goes (`[00:01.000 --> 00:03.500]
//! text`), which becomes a partial event, and writes a JSON transcript,
//! which becomes the final one.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

use crate::DictationError;
use crate::transcript::{Segment, Transcript, WhisperModel};

/// Which Whisper program.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum WhisperEngine {
    /// whisper.cpp (`whisper-cli`).
    Cpp,
    /// faster-whisper (`whisper-ctranslate2`).
    Faster,
    /// openai-whisper (`whisper`).
    OpenAi,
}

impl WhisperEngine {
    /// Every engine, in the order they are preferred: whisper.cpp needs no
    /// Python and starts fastest, faster-whisper is next.
    pub const ALL: [WhisperEngine; 3] = [
        WhisperEngine::Cpp,
        WhisperEngine::Faster,
        WhisperEngine::OpenAi,
    ];

    /// `cpp`, `faster`, or `openai`.
    pub fn as_str(self) -> &'static str {
        match self {
            WhisperEngine::Cpp => "cpp",
            WhisperEngine::Faster => "faster",
            WhisperEngine::OpenAi => "openai",
        }
    }

    /// The engine's name for people.
    pub fn display_name(self) -> &'static str {
        match self {
            WhisperEngine::Cpp => "whisper.cpp",
            WhisperEngine::Faster => "faster-whisper",
            WhisperEngine::OpenAi => "OpenAI Whisper",
        }
    }

    /// Parses `cpp`, `whisper.cpp`, `faster`, `faster-whisper`, `openai`,
    /// or `openai-whisper`.
    pub fn parse(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "cpp" | "whisper.cpp" | "whisper-cpp" | "whisper-cli" => Some(WhisperEngine::Cpp),
            "faster" | "faster-whisper" | "whisper-ctranslate2" | "ctranslate2" => {
                Some(WhisperEngine::Faster)
            }
            "openai" | "openai-whisper" | "whisper" => Some(WhisperEngine::OpenAi),
            _ => None,
        }
    }

    /// The program names to look for on `PATH`.
    pub fn program_names(self) -> &'static [&'static str] {
        match self {
            WhisperEngine::Cpp => &["whisper-cli", "whisper-cpp"],
            WhisperEngine::Faster => &["whisper-ctranslate2"],
            WhisperEngine::OpenAi => &["whisper"],
        }
    }

    /// Guesses the engine from a program's file name.
    pub fn from_program(program: &Path) -> Option<Self> {
        let name = program.file_stem()?.to_string_lossy().to_ascii_lowercase();
        if name.contains("ctranslate2") || name.contains("faster") {
            Some(WhisperEngine::Faster)
        } else if name.contains("whisper-cli") || name.contains("whisper-cpp") || name == "main" {
            Some(WhisperEngine::Cpp)
        } else if name.contains("whisper") {
            Some(WhisperEngine::OpenAi)
        } else {
            None
        }
    }
}

/// A Whisper program found on this computer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DetectedEngine {
    /// Which program it is.
    pub engine: WhisperEngine,
    /// Where it is.
    pub program: PathBuf,
}

/// The environment variable naming a Whisper program to use.
pub const WHISPER_ENV: &str = "TEXTWEAVER_WHISPER";
/// The environment variable naming the engine of [`WHISPER_ENV`]'s program
/// (`cpp`, `faster`, `openai`) when its file name does not tell.
pub const WHISPER_ENGINE_ENV: &str = "TEXTWEAVER_WHISPER_ENGINE";
/// The environment variable naming a folder of whisper.cpp model files.
pub const WHISPER_MODELS_ENV: &str = "TEXTWEAVER_WHISPER_MODELS";

/// Finds `name` in the folders of `path_var` (a `PATH` value), by the
/// workspace's one rule ([`textweaver_core::process::find_program_in`]:
/// the `PATHEXT` extensions first on Windows, then the bare name).
pub fn find_on_path(name: &str, path_var: &OsStr) -> Option<PathBuf> {
    textweaver_core::process::find_program_in(name, path_var)
}

/// Every Whisper program found: the one named by `TEXTWEAVER_WHISPER`
/// first, then each engine's programs on `path_var`, in preference order.
pub fn detect_engines(path_var: &OsStr) -> Vec<DetectedEngine> {
    let mut found: Vec<DetectedEngine> = Vec::new();
    if let Some(program) = std::env::var_os(WHISPER_ENV).map(PathBuf::from)
        && program.is_file()
    {
        let engine = std::env::var(WHISPER_ENGINE_ENV)
            .ok()
            .and_then(|e| WhisperEngine::parse(&e))
            .or_else(|| WhisperEngine::from_program(&program));
        if let Some(engine) = engine {
            found.push(DetectedEngine { engine, program });
        }
    }
    for engine in WhisperEngine::ALL {
        for name in engine.program_names() {
            if let Some(program) = find_on_path(name, path_var)
                && !found.iter().any(|d| d.program == program)
            {
                found.push(DetectedEngine { engine, program });
            }
        }
    }
    found
}

/// Every Whisper program on this computer's `PATH`.
pub fn detect() -> Vec<DetectedEngine> {
    detect_engines(&std::env::var_os("PATH").unwrap_or_default())
}

/// The folders searched for whisper.cpp model files, in order:
/// `TEXTWEAVER_WHISPER_MODELS`, the `extra` folders (the app passes its
/// data folder's `whisper` folder), and `models` beside the program and one
/// level up (where whisper.cpp's build and download script put them).
pub fn model_dirs(program: &Path, extra: &[PathBuf]) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(d) = std::env::var_os(WHISPER_MODELS_ENV) {
        dirs.push(PathBuf::from(d));
    }
    dirs.extend(extra.iter().cloned());
    if let Some(parent) = program.parent() {
        dirs.push(parent.join("models"));
        if let Some(up) = parent.parent() {
            dirs.push(up.join("models"));
        }
    }
    dirs
}

/// The whisper.cpp model file for `model` in `dirs`.
pub fn find_model(model: WhisperModel, dirs: &[PathBuf]) -> Result<PathBuf, DictationError> {
    let file = model.ggml_file();
    dirs.iter()
        .map(|d| d.join(&file))
        .find(|p| p.is_file())
        .ok_or_else(|| DictationError::ModelNotFound {
            model: model.as_str().to_owned(),
            file,
            searched: dirs.to_vec(),
        })
}

/// What to run for one transcription.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invocation {
    /// Arguments after the program.
    pub args: Vec<OsString>,
    /// Where the JSON transcript will be.
    pub json: PathBuf,
}

/// The command line for transcribing `input` (a WAV file for whisper.cpp)
/// into `out_dir`.
pub fn invocation(
    engine: WhisperEngine,
    model: WhisperModel,
    model_file: Option<&Path>,
    language: Option<&str>,
    threads: Option<u32>,
    input: &Path,
    out_dir: &Path,
) -> Invocation {
    let mut args: Vec<OsString> = Vec::new();
    let lang = language.filter(|l| !l.trim().is_empty());
    match engine {
        WhisperEngine::Cpp => {
            let base = out_dir.join("transcript");
            args.push("-m".into());
            args.push(
                model_file.map_or_else(|| model.ggml_file().into(), |p| p.as_os_str().to_owned()),
            );
            args.push("-f".into());
            args.push(input.as_os_str().to_owned());
            args.push("-l".into());
            args.push(lang.unwrap_or("auto").into());
            if let Some(t) = threads {
                args.push("-t".into());
                args.push(t.to_string().into());
            }
            // No progress or system info, only the segments; plus JSON.
            args.push("-np".into());
            args.push("-oj".into());
            args.push("-of".into());
            args.push(base.as_os_str().to_owned());
            Invocation {
                args,
                json: base.with_extension("json"),
            }
        }
        WhisperEngine::Faster | WhisperEngine::OpenAi => {
            args.push(input.as_os_str().to_owned());
            args.push("--model".into());
            args.push(model.as_str().into());
            args.push("--output_format".into());
            args.push("json".into());
            args.push("--output_dir".into());
            args.push(out_dir.as_os_str().to_owned());
            args.push("--verbose".into());
            args.push("True".into());
            if engine == WhisperEngine::OpenAi {
                // Half precision is GPU-only; on a CPU it only prints a warning.
                args.push("--fp16".into());
                args.push("False".into());
            }
            if let Some(l) = lang {
                args.push("--language".into());
                args.push(l.into());
            }
            if let Some(t) = threads {
                args.push("--threads".into());
                args.push(t.to_string().into());
            }
            let stem = input.file_stem().unwrap_or_default();
            let mut json = out_dir.join(stem);
            json.set_extension("json");
            Invocation { args, json }
        }
    }
}

/// `[00:01.000 --> 00:03.500]  text` (openai, faster) or
/// `[00:00:01.000 --> 00:00:03.500]   text` (whisper.cpp).
static SEGMENT_LINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*\[\s*((?:\d+:)?\d+:\d+(?:[.,]\d+)?)\s*-->\s*((?:\d+:)?\d+:\d+(?:[.,]\d+)?)\s*\]\s?(.*)$")
        .expect("valid regex")
});

/// `hh:mm:ss.mmm` or `mm:ss.mmm` to milliseconds.
fn clock_ms(s: &str) -> Option<u64> {
    let (clock, frac) = match s.split_once(['.', ',']) {
        Some((c, f)) => (c, f),
        None => (s, "0"),
    };
    let mut secs: u64 = 0;
    for part in clock.split(':') {
        secs = secs.checked_mul(60)?.checked_add(part.parse().ok()?)?;
    }
    let frac_ms: u64 = {
        let digits: String = frac.chars().take(3).collect();
        let n: u64 = digits.parse().ok()?;
        n * 10u64.pow(3 - u32::try_from(digits.len()).ok()?)
    };
    secs.checked_mul(1000)?.checked_add(frac_ms)
}

/// Parses one printed segment line.
pub fn parse_segment_line(line: &str) -> Option<Segment> {
    let c = SEGMENT_LINE.captures(line.trim_end_matches(['\r', '\n']))?;
    Some(Segment {
        start_ms: clock_ms(c.get(1)?.as_str())?,
        end_ms: clock_ms(c.get(2)?.as_str())?,
        text: c.get(3)?.as_str().trim().to_owned(),
    })
}

fn seconds_to_ms(v: &serde_json::Value) -> u64 {
    let s = v.as_f64().unwrap_or(0.0);
    if s.is_finite() && s > 0.0 {
        // Rounded to whole milliseconds; transcripts are far below u64::MAX.
        (s * 1000.0).round() as u64
    } else {
        0
    }
}

/// Parses a JSON transcript: whisper.cpp's `transcription` list (offsets
/// in milliseconds) or openai/faster's `segments` list (seconds).
pub fn parse_json(text: &str) -> Result<Transcript, String> {
    let v: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let mut segments = Vec::new();
    if let Some(list) = v.get("transcription").and_then(|t| t.as_array()) {
        for item in list {
            let offsets = item.get("offsets");
            let ms = |k: &str| {
                offsets
                    .and_then(|o| o.get(k))
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0)
            };
            segments.push(Segment {
                start_ms: ms("from"),
                end_ms: ms("to"),
                text: item
                    .get("text")
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_owned(),
            });
        }
    } else if let Some(list) = v.get("segments").and_then(|t| t.as_array()) {
        for item in list {
            segments.push(Segment {
                start_ms: item.get("start").map_or(0, seconds_to_ms),
                end_ms: item.get("end").map_or(0, seconds_to_ms),
                text: item
                    .get("text")
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_owned(),
            });
        }
    } else if let Some(t) = v.get("text").and_then(|t| t.as_str()) {
        segments.push(Segment {
            start_ms: 0,
            end_ms: 0,
            text: t.trim().to_owned(),
        });
    } else {
        return Err("no transcription in the JSON output".to_owned());
    }
    Ok(Transcript { segments })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_names() {
        for e in WhisperEngine::ALL {
            assert_eq!(WhisperEngine::parse(e.as_str()), Some(e));
            // JSON uses the same names.
            assert_eq!(
                serde_json::to_string(&e).unwrap(),
                format!("\"{}\"", e.as_str())
            );
        }
        assert_eq!(
            WhisperEngine::parse("faster-whisper"),
            Some(WhisperEngine::Faster)
        );
        assert_eq!(
            WhisperEngine::from_program(Path::new("/usr/bin/whisper-cli")),
            Some(WhisperEngine::Cpp)
        );
        assert_eq!(
            WhisperEngine::from_program(Path::new(r"C:\py\Scripts\whisper-ctranslate2.exe")),
            Some(WhisperEngine::Faster)
        );
        assert_eq!(
            WhisperEngine::from_program(Path::new(r"C:\py\Scripts\whisper.exe")),
            Some(WhisperEngine::OpenAi)
        );
        assert_eq!(WhisperEngine::from_program(Path::new("/bin/ls")), None);
    }

    #[test]
    fn segment_lines_of_every_program() {
        assert_eq!(
            parse_segment_line("[00:00:01.000 --> 00:00:03.500]   Hello there."),
            Some(Segment {
                start_ms: 1000,
                end_ms: 3500,
                text: "Hello there.".into()
            })
        );
        assert_eq!(
            parse_segment_line("[01:02.5 --> 01:04.250]  the quick fox"),
            Some(Segment {
                start_ms: 62_500,
                end_ms: 64_250,
                text: "the quick fox".into()
            })
        );
        assert_eq!(
            parse_segment_line("[1:00:00,000 --> 1:00:02,000] late"),
            Some(Segment {
                start_ms: 3_600_000,
                end_ms: 3_602_000,
                text: "late".into()
            })
        );
        // Lines as read from a pipe keep their line ends.
        assert_eq!(
            parse_segment_line("[00:00.000 --> 00:01.000]  Hi.\r\n").map(|s| s.text),
            Some("Hi.".to_owned())
        );
        assert_eq!(parse_segment_line("Detecting language: English"), None);
        assert_eq!(
            parse_segment_line("whisper_init_from_file: loading model"),
            None
        );
    }

    #[test]
    fn json_of_every_program() {
        let cpp = r#"{"systeminfo":"x","transcription":[{"timestamps":{"from":"00:00:00,000","to":"00:00:02,000"},"offsets":{"from":0,"to":2000},"text":" Hello."},{"offsets":{"from":2000,"to":4100},"text":" World."}]}"#;
        let t = parse_json(cpp).unwrap();
        assert_eq!(t.text(), "Hello. World.");
        assert_eq!(t.segments[1].end_ms, 4100);
        let openai = r#"{"text":" a b","segments":[{"id":0,"start":0.0,"end":1.25,"text":" a"},{"start":1.25,"end":2.0,"text":" b"}],"language":"en"}"#;
        let t = parse_json(openai).unwrap();
        assert_eq!(t.segments[0].end_ms, 1250);
        assert_eq!(t.text(), "a b");
        assert_eq!(
            parse_json(r#"{"text":" only text"}"#).unwrap().text(),
            "only text"
        );
        assert!(parse_json("{}").is_err());
        assert!(parse_json("not json").is_err());
    }

    #[test]
    fn command_lines() {
        let out = Path::new("/tmp/out");
        let cpp = invocation(
            WhisperEngine::Cpp,
            WhisperModel::Base,
            Some(Path::new("/m/ggml-base.bin")),
            Some("en"),
            Some(4),
            Path::new("/a/in.wav"),
            out,
        );
        let args: Vec<String> = cpp
            .args
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            args,
            vec![
                "-m",
                "/m/ggml-base.bin",
                "-f",
                "/a/in.wav",
                "-l",
                "en",
                "-t",
                "4",
                "-np",
                "-oj",
                "-of"
            ]
            .into_iter()
            .map(str::to_owned)
            .chain([out.join("transcript").to_string_lossy().into_owned()])
            .collect::<Vec<_>>()
        );
        assert_eq!(cpp.json, out.join("transcript.json"));

        let py = invocation(
            WhisperEngine::OpenAi,
            WhisperModel::Small,
            None,
            None,
            None,
            Path::new("/a/talk.mp3"),
            out,
        );
        let args: Vec<String> = py
            .args
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(args.windows(2).any(|w| w == ["--model", "small"]));
        assert!(args.windows(2).any(|w| w == ["--fp16", "False"]));
        assert!(!args.contains(&"--language".to_owned()));
        assert_eq!(py.json, out.join("talk.json"));
        let fw = invocation(
            WhisperEngine::Faster,
            WhisperModel::Small,
            None,
            Some("de"),
            None,
            Path::new("/a/talk.mp3"),
            out,
        );
        let args: Vec<String> = fw
            .args
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(!args.contains(&"--fp16".to_owned()));
        assert!(args.windows(2).any(|w| w == ["--language", "de"]));
    }

    #[test]
    fn finds_programs_and_models_in_given_folders() {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let exe = if cfg!(windows) {
            "whisper-cli.exe"
        } else {
            "whisper-cli"
        };
        std::fs::write(bin.join(exe), "").unwrap();
        std::fs::write(
            bin.join(if cfg!(windows) {
                "whisper.exe"
            } else {
                "whisper"
            }),
            "",
        )
        .unwrap();
        let path = std::env::join_paths([bin.clone()]).unwrap();
        assert_eq!(find_on_path("whisper-cli", &path), Some(bin.join(exe)));
        assert_eq!(find_on_path("nope", &path), None);
        let found = detect_engines(&path);
        let engines: Vec<WhisperEngine> = found.iter().map(|d| d.engine).collect();
        // TEXTWEAVER_WHISPER is not set in tests.
        assert_eq!(engines, vec![WhisperEngine::Cpp, WhisperEngine::OpenAi]);

        let models = dir.path().join("models");
        std::fs::create_dir_all(&models).unwrap();
        std::fs::write(models.join("ggml-tiny.bin"), "").unwrap();
        let dirs = model_dirs(&bin.join(exe), &[]);
        assert!(dirs.contains(&models));
        assert_eq!(
            find_model(WhisperModel::Tiny, &dirs).unwrap(),
            models.join("ggml-tiny.bin")
        );
        let err = find_model(WhisperModel::Base, &dirs).unwrap_err();
        assert!(err.to_string().contains("ggml-base.bin"), "{err}");
    }
}

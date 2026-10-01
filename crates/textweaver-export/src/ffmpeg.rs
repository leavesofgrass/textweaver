//! Converting the exported WAV with ffmpeg, when it is installed.
//!
//! ffmpeg is found through `TEXTWEAVER_FFMPEG` (a path to the program) or on
//! `PATH`. textweaver never downloads or bundles it. The commands follow
//! Star's (`star/tts/audio.py`, `star/audiobook.py`): MP3 with LAME at VBR
//! quality 2; M4B as AAC in an MP4 container at 64 kbit/s (spoken word needs
//! little) with `+faststart`, the chapters and title mapped from an ffmpeg
//! metadata file. MP3 gets the same metadata, so players that read ID3
//! chapters see them too.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::Serialize;

use crate::ExportError;

/// The audio formats export writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioFormat {
    /// WAV, written directly (no ffmpeg needed).
    Wav,
    /// MP3, written in process (the `mp3` feature; through ffmpeg
    /// without it).
    Mp3,
    /// An M4B audiobook with chapters through ffmpeg.
    M4b,
    /// FLAC, written in process (the `flac` feature; through ffmpeg
    /// without it).
    Flac,
    /// Ogg Opus, written in process (the `opus` feature; through ffmpeg
    /// without it).
    Opus,
}

impl AudioFormat {
    /// The format for a file name's extension (`wav`, `flac`, `mp3`,
    /// `opus`, `m4b`; any case).
    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "wav" => Some(AudioFormat::Wav),
            "flac" => Some(AudioFormat::Flac),
            "mp3" => Some(AudioFormat::Mp3),
            "m4b" => Some(AudioFormat::M4b),
            "opus" => Some(AudioFormat::Opus),
            _ => None,
        }
    }

    /// The name users see ("WAV", "FLAC", "MP3", "Opus", "M4B").
    pub fn name(self) -> &'static str {
        match self {
            AudioFormat::Wav => "WAV",
            AudioFormat::Flac => "FLAC",
            AudioFormat::Mp3 => "MP3",
            AudioFormat::M4b => "M4B",
            AudioFormat::Opus => "Opus",
        }
    }

    /// Whether writing this format needs ffmpeg in this build: M4B always;
    /// FLAC, MP3 and Opus only when built without the `flac`, `mp3` or
    /// `opus` feature.
    pub fn needs_ffmpeg(self) -> bool {
        match self {
            AudioFormat::Wav => false,
            AudioFormat::Flac => !cfg!(feature = "flac"),
            AudioFormat::Mp3 => !cfg!(feature = "mp3"),
            AudioFormat::M4b => true,
            AudioFormat::Opus => !cfg!(feature = "opus"),
        }
    }
}

/// Default AAC bitrate for M4B (Star's `DEFAULT_M4B_BITRATE`).
pub const M4B_BITRATE: &str = "64k";

/// The ffmpeg program: `TEXTWEAVER_FFMPEG` if set (and it exists), else
/// `ffmpeg` on `PATH`.
pub fn find() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("TEXTWEAVER_FFMPEG") {
        let p = PathBuf::from(p);
        return p.is_file().then_some(p);
    }
    find_on_path("ffmpeg", std::env::var_os("PATH"))
}

/// `name` in the directories of `path` (a `PATH`-style list), trying the
/// Windows executable extension too.
pub fn find_on_path(name: &str, path: Option<OsString>) -> Option<PathBuf> {
    let path = path?;
    let names: Vec<String> = if cfg!(windows) {
        vec![format!("{name}.exe"), name.to_owned()]
    } else {
        vec![name.to_owned()]
    };
    std::env::split_paths(&path)
        .flat_map(|dir| names.iter().map(move |n| dir.join(n)))
        .find(|p| p.is_file())
}

/// The ffmpeg arguments that turn `wav` into `out` in `format`, with
/// metadata (title and chapters) from `metadata`.
pub fn args(wav: &Path, metadata: &Path, out: &Path, format: AudioFormat) -> Vec<OsString> {
    let mut a: Vec<OsString> = ["-y", "-hide_banner", "-loglevel", "error", "-nostdin"]
        .into_iter()
        .map(OsString::from)
        .collect();
    a.push("-i".into());
    a.push(wav.into());
    a.push("-i".into());
    a.push(metadata.into());
    for s in ["-map", "0:a", "-map_metadata", "1", "-map_chapters", "1"] {
        a.push(s.into());
    }
    let codec: &[&str] = match format {
        AudioFormat::Mp3 => &[
            "-codec:a",
            "libmp3lame",
            "-qscale:a",
            "2",
            "-id3v2_version",
            "3",
        ],
        AudioFormat::M4b => &[
            "-codec:a",
            "aac",
            "-b:a",
            M4B_BITRATE,
            "-f",
            "mp4",
            "-movflags",
            "+faststart",
        ],
        AudioFormat::Flac => &["-codec:a", "flac"],
        // The in-process encoder's speech settings: mono, VoIP tuning,
        // 32 kbit/s, 20 ms packets.
        AudioFormat::Opus => &[
            "-codec:a",
            "libopus",
            "-ac",
            "1",
            "-b:a",
            "32k",
            "-application",
            "voip",
            "-frame_duration",
            "20",
        ],
        AudioFormat::Wav => &[],
    };
    a.extend(codec.iter().map(OsString::from));
    a.push(out.into());
    a
}

/// Runs ffmpeg; its error output becomes the error message.
pub fn run(ffmpeg: &Path, args: &[OsString]) -> Result<(), ExportError> {
    let output = Command::new(ffmpeg)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| ExportError::io(ffmpeg, e))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let last = stderr
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("no error output")
        .trim()
        .to_owned();
    Err(ExportError::Ffmpeg(last))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_from_extensions() {
        assert_eq!(
            AudioFormat::from_path(Path::new("a.MP3")),
            Some(AudioFormat::Mp3)
        );
        assert_eq!(
            AudioFormat::from_path(Path::new("b.m4b")),
            Some(AudioFormat::M4b)
        );
        assert_eq!(
            AudioFormat::from_path(Path::new("c.wav")),
            Some(AudioFormat::Wav)
        );
        assert_eq!(
            AudioFormat::from_path(Path::new("e.Flac")),
            Some(AudioFormat::Flac)
        );
        assert!(!AudioFormat::Wav.needs_ffmpeg());
        assert!(AudioFormat::M4b.needs_ffmpeg());
        assert_eq!(AudioFormat::Flac.needs_ffmpeg(), !cfg!(feature = "flac"));
        assert_eq!(AudioFormat::Mp3.needs_ffmpeg(), !cfg!(feature = "mp3"));
        assert_eq!(
            AudioFormat::from_path(Path::new("f.OPUS")),
            Some(AudioFormat::Opus)
        );
        assert_eq!(AudioFormat::Opus.needs_ffmpeg(), !cfg!(feature = "opus"));
        assert_eq!(AudioFormat::from_path(Path::new("d.ogg")), None);
        assert_eq!(AudioFormat::from_path(Path::new("noext")), None);
    }

    #[test]
    fn m4b_arguments_follow_star() {
        let a = args(
            Path::new("in.wav"),
            Path::new("meta.txt"),
            Path::new("out.m4b"),
            AudioFormat::M4b,
        );
        let a: Vec<String> = a.iter().map(|s| s.to_string_lossy().into_owned()).collect();
        let joined = a.join(" ");
        assert!(
            joined.ends_with(
                "-i in.wav -i meta.txt -map 0:a -map_metadata 1 -map_chapters 1 \
                 -codec:a aac -b:a 64k -f mp4 -movflags +faststart out.m4b"
            ),
            "{joined}"
        );
        assert_eq!(a[0], "-y");
    }

    #[test]
    fn path_search() {
        let dir = tempfile::tempdir().unwrap();
        let name = if cfg!(windows) { "tool.exe" } else { "tool" };
        std::fs::write(dir.path().join(name), b"").unwrap();
        let path = std::env::join_paths([Path::new("/no/such/dir"), dir.path()]).unwrap();
        assert_eq!(
            find_on_path("tool", Some(path.clone())),
            Some(dir.path().join(name))
        );
        assert_eq!(find_on_path("other", Some(path)), None);
        assert_eq!(find_on_path("tool", None), None);
    }
}

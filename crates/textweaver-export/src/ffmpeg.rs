//! Converting the exported WAV with ffmpeg, when it is installed.
//!
//! ffmpeg is found in textweaver's components folder first (a copy placed
//! or unpacked there from a components source), then through
//! `TEXTWEAVER_FFMPEG` (a path to the program), then on `PATH`. textweaver
//! never bundles it. The commands follow
//! star's (`star/tts/audio.py`, `star/audiobook.py`): MP3 with LAME at VBR
//! quality 2; M4B as AAC in an MP4 container at 64 kbit/s (spoken word needs
//! little) with `+faststart`, the chapters and title mapped from an ffmpeg
//! metadata file. MP3 gets the same metadata, so players that read ID3
//! chapters see them too.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Stdio;

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
    /// Ogg Vorbis, written in process (the `vorbis` feature; through
    /// ffmpeg without it).
    Ogg,
    /// A karaoke video with captions and chapters (MP4 through ffmpeg; the
    /// frames are drawn with the `video` feature).
    Mp4,
}

impl AudioFormat {
    /// The format for a file name's extension (`wav`, `flac`, `mp3`,
    /// `opus`, `ogg`, `m4b`, `mp4`; any case).
    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "wav" => Some(AudioFormat::Wav),
            "flac" => Some(AudioFormat::Flac),
            "mp3" => Some(AudioFormat::Mp3),
            "m4b" => Some(AudioFormat::M4b),
            "opus" => Some(AudioFormat::Opus),
            "ogg" => Some(AudioFormat::Ogg),
            "mp4" => Some(AudioFormat::Mp4),
            _ => None,
        }
    }

    /// The name users see ("WAV", "FLAC", "MP3", "Opus", "Ogg Vorbis",
    /// "M4B", "MP4").
    pub fn name(self) -> &'static str {
        match self {
            AudioFormat::Wav => "WAV",
            AudioFormat::Flac => "FLAC",
            AudioFormat::Mp3 => "MP3",
            AudioFormat::M4b => "M4B",
            AudioFormat::Opus => "Opus",
            AudioFormat::Ogg => "Ogg Vorbis",
            AudioFormat::Mp4 => "MP4",
        }
    }

    /// Whether writing this format needs ffmpeg in this build: M4B and MP4
    /// video always;
    /// FLAC, MP3, Opus and Ogg Vorbis only when built without the `flac`,
    /// `mp3`, `opus` or `vorbis` feature.
    pub fn needs_ffmpeg(self) -> bool {
        match self {
            AudioFormat::Wav => false,
            AudioFormat::Flac => !cfg!(feature = "flac"),
            AudioFormat::Mp3 => !cfg!(feature = "mp3"),
            AudioFormat::M4b | AudioFormat::Mp4 => true,
            AudioFormat::Opus => !cfg!(feature = "opus"),
            AudioFormat::Ogg => !cfg!(feature = "vorbis"),
        }
    }
}

/// Default AAC bitrate for M4B (star's `DEFAULT_M4B_BITRATE`).
pub const M4B_BITRATE: &str = "64k";

/// The ffmpeg program: the copy in textweaver's components folder
/// first, then `TEXTWEAVER_FFMPEG` if set (and it exists), else `ffmpeg`
/// on `PATH`.
pub fn find() -> Option<PathBuf> {
    find_in(
        textweaver_store::components_dir().as_deref(),
        std::env::var_os("TEXTWEAVER_FFMPEG"),
        std::env::var_os("PATH"),
    )
}

/// [`find`] with its places given: the components folder, the
/// `TEXTWEAVER_FFMPEG` value, and the `PATH` value.
pub fn find_in(
    components: Option<&Path>,
    env: Option<OsString>,
    path: Option<OsString>,
) -> Option<PathBuf> {
    textweaver_store::find_helper_in(components, "ffmpeg", env, path)
}

/// `name` in the directories of `path` (a `PATH`-style list), by the
/// workspace's one rule ([`textweaver_core::process::find_program_in`]).
pub fn find_on_path(name: &str, path: Option<OsString>) -> Option<PathBuf> {
    textweaver_core::process::find_program_in(name, &path?)
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
        // The in-process encoder's quality (`oggenc -q 3`).
        AudioFormat::Ogg => &["-codec:a", "libvorbis", "-qscale:a", "3"],
        // Video has its own arguments (`video::args`).
        AudioFormat::Wav | AudioFormat::Mp4 => &[],
    };
    a.extend(codec.iter().map(OsString::from));
    a.push(out.into());
    a
}

/// Runs ffmpeg; its error output becomes the error message.
pub fn run(ffmpeg: &Path, args: &[OsString]) -> Result<(), ExportError> {
    run_with_stop(ffmpeg, args, &|| false)
}

/// How often a running ffmpeg is checked for a stop.
const POLL: std::time::Duration = std::time::Duration::from_millis(100);

/// [`run`], asking `stop` every 100 ms while ffmpeg works: when it says
/// stop, ffmpeg is killed and the result is [`ExportError::Cancelled`] (the
/// caller removes the partial file).
pub fn run_with_stop(
    ffmpeg: &Path,
    args: &[OsString],
    stop: &dyn Fn() -> bool,
) -> Result<(), ExportError> {
    let mut child = textweaver_core::process::command(ffmpeg)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| ExportError::io(ffmpeg, e))?;
    let reader = drain(child.stderr.take());
    wait_with_stop(child, reader, ffmpeg, stop)
}

/// The thread that reads a running ffmpeg's error output, so a chatty
/// ffmpeg never blocks on a full pipe while it is polled.
pub(crate) type Drain = Option<std::thread::JoinHandle<Vec<u8>>>;

/// Starts reading `err` on its own thread.
pub(crate) fn drain(err: Option<std::process::ChildStderr>) -> Drain {
    err.map(|mut err| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = std::io::Read::read_to_end(&mut err, &mut buf);
            buf
        })
    })
}

/// Kills a stopped ffmpeg and waits for it: the result of a stop.
pub(crate) fn kill(mut child: std::process::Child, reader: Drain) -> ExportError {
    let _ = child.kill();
    let _ = child.wait();
    if let Some(r) = reader {
        let _ = r.join();
    }
    ExportError::Cancelled
}

/// Waits for ffmpeg to finish, asking `stop` every 100 ms; its last line
/// of error output becomes the error when it fails.
pub(crate) fn wait_with_stop(
    mut child: std::process::Child,
    reader: Drain,
    ffmpeg: &Path,
    stop: &dyn Fn() -> bool,
) -> Result<(), ExportError> {
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if stop() => return Err(kill(child, reader)),
            Ok(None) => std::thread::sleep(POLL),
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(ExportError::io(ffmpeg, e));
            }
        }
    };
    let stderr_bytes = reader.and_then(|r| r.join().ok()).unwrap_or_default();
    if status.success() {
        return Ok(());
    }
    let stderr = textweaver_core::process::decode_output(&stderr_bytes);
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
        assert_eq!(
            AudioFormat::from_path(Path::new("d.Ogg")),
            Some(AudioFormat::Ogg)
        );
        assert_eq!(AudioFormat::Ogg.needs_ffmpeg(), !cfg!(feature = "vorbis"));
        assert_eq!(
            AudioFormat::from_path(Path::new("v.MP4")),
            Some(AudioFormat::Mp4)
        );
        assert!(AudioFormat::Mp4.needs_ffmpeg());
        assert_eq!(AudioFormat::from_path(Path::new("g.oga")), None);
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

    #[test]
    fn the_components_folder_comes_first() {
        let tmp = tempfile::tempdir().unwrap();
        let components = tmp.path().join("components");
        let name = if cfg!(windows) {
            "ffmpeg.exe"
        } else {
            "ffmpeg"
        };
        let on_path = tmp.path().join("bin");
        std::fs::create_dir_all(&on_path).unwrap();
        std::fs::write(on_path.join(name), b"").unwrap();
        let path = Some(std::env::join_paths([&on_path]).unwrap());
        // Nothing in the components folder: the PATH's.
        assert_eq!(
            find_in(Some(&components), None, path.clone()),
            Some(on_path.join(name))
        );
        // A placed copy wins over the variable and the PATH.
        let placed = components.join("ffmpeg").join("ffmpeg-9.0.2").join("bin");
        std::fs::create_dir_all(&placed).unwrap();
        std::fs::write(placed.join(name), b"").unwrap();
        let env = Some(on_path.join(name).into_os_string());
        assert_eq!(
            find_in(Some(&components), env, path),
            Some(placed.join(name))
        );
        // Found with no variable and nothing on the PATH.
        assert_eq!(
            find_in(Some(&components), None, None),
            Some(placed.join(name))
        );
    }
}

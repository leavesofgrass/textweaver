//! The diagnostic log (`--log`): announcements, commands, keys, caret sync,
//! load timing, and frame times, one line each, to standard error or to
//! `--log-file`. The UI Automation report reads it.

use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

static FILE: OnceLock<Mutex<File>> = OnceLock::new();

/// Sends log lines to `path` instead of standard error.
pub fn to_file(path: &Path) -> std::io::Result<()> {
    let file = File::create(path)?;
    let _ = FILE.set(Mutex::new(file));
    Ok(())
}

/// True when log lines go to a `--log-file`.
pub fn to_file_active() -> bool {
    FILE.get().is_some()
}

/// Writes one line.
pub fn line(text: &str) {
    match FILE.get() {
        Some(file) => {
            if let Ok(mut f) = file.lock() {
                let _ = writeln!(f, "{text}");
                let _ = f.flush();
            }
        }
        None => eprintln!("{text}"),
    }
}

/// How many frames each frame-time line covers.
pub const FRAMES_PER_LINE: usize = 200;

/// The name of Masonry's span around drawing one frame (layout, paint,
/// the accessibility update, rendering, and the wait for the GPU).
const FRAME_SPAN: &str = "redraw";

/// Logs a frame-time line every [`FRAMES_PER_LINE`] frames, timed from
/// Masonry's own trace span, so a visual change can be measured in a real
/// window (the frame-time probe leaves out the GPU). Installs Masonry's
/// usual trace output with the timer added; call it before the window
/// starts, which otherwise installs the output alone.
pub fn frame_times() {
    let times = Mutex::new(FrameTimes::default());
    let report = Box::new(move |d: Duration| {
        let line = times.lock().ok().and_then(|mut t| t.push(d));
        if let Some(l) = line {
            self::line(&l);
        }
    });
    if masonry::app::try_init_tracing_with_span_times(FRAME_SPAN, report).is_err() {
        line("frame times: not measured, tracing was already set up");
    }
}

/// Frame times waiting for their line.
#[derive(Debug, Default)]
pub struct FrameTimes {
    /// This line's frames, in milliseconds.
    ms: Vec<f64>,
    /// Frames in earlier lines.
    before: usize,
}

impl FrameTimes {
    /// Adds a frame; every [`FRAMES_PER_LINE`] frames gives the line, in
    /// words, with the median, the 95th percentile, and the worst.
    pub fn push(&mut self, frame: Duration) -> Option<String> {
        self.ms.push(frame.as_secs_f64() * 1000.0);
        if self.ms.len() < FRAMES_PER_LINE {
            return None;
        }
        self.ms.sort_by(f64::total_cmp);
        let first = self.before + 1;
        self.before += self.ms.len();
        let line = format!(
            "frame times, frames {first} to {}: median {:.1} ms, 95th percentile {:.1} ms, worst {:.1} ms",
            self.before,
            quantile(&self.ms, 0.5),
            quantile(&self.ms, 0.95),
            quantile(&self.ms, 1.0),
        );
        self.ms.clear();
        Some(line)
    }
}

/// The value at fraction `q` of sorted `v` (0 when empty).
pub fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let i = ((sorted.len() - 1) as f64 * q.clamp(0.0, 1.0)).round() as usize;
    sorted[i.min(sorted.len() - 1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_time_line_every_two_hundred_frames() {
        let mut t = FrameTimes::default();
        let mut lines = Vec::new();
        for k in 1..=400u64 {
            // 1 ms to 200 ms, twice.
            let ms = (k - 1) % 200 + 1;
            if let Some(l) = t.push(Duration::from_millis(ms)) {
                lines.push((k, l));
            }
        }
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].0, 200);
        assert_eq!(
            lines[0].1,
            "frame times, frames 1 to 200: median 101.0 ms, 95th percentile 190.0 ms, worst 200.0 ms"
        );
        assert!(lines[1].1.starts_with("frame times, frames 201 to 400: "));
    }
}

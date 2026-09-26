//! Synthesizes text to a WAV file with the `eci` backend (never plays it).
//!
//! ```text
//! cargo run -p textweaver-eci --example say -- OUT.wav [--voice ID] [--rate WPM] [TEXT...]
//! ```
//!
//! Prints each word with the audio time of its index mark, so the file can
//! be checked by ear against the reported word timing.

use std::path::PathBuf;

use textweaver_core::Rate;
use textweaver_eci::{AudioOutput, EciBackend, EciConfig};
use textweaver_speech::{SpeechBackend, VoiceParams};

const DEFAULT_TEXT: &str = "Dr. Smith opened the library at 9:30 a.m. Café crème, naïve résumé. \
This is textweaver reading with ETI-Eloquence, and every word you hear \
is highlighted at the moment it is spoken.";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let out = PathBuf::from(
        args.next()
            .ok_or("usage: say OUT.wav [--voice ID] [--rate WPM] [TEXT...]")?,
    );
    let mut params = VoiceParams::default();
    let mut text = Vec::new();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--voice" => params.voice = args.next(),
            "--rate" => params.rate = Rate::Wpm(args.next().ok_or("--rate WPM")?.parse()?),
            _ => text.push(a),
        }
    }
    let text = if text.is_empty() {
        DEFAULT_TEXT.to_string()
    } else {
        text.join(" ")
    };
    let mut b = EciBackend::new(EciConfig {
        output: AudioOutput::Null { speed: 1.0 },
        ..EciConfig::default()
    })?;
    b.set_params(&params)?;
    println!(
        "engine ECI {}, {} Hz, host {}",
        b.engine_version().unwrap_or("?"),
        b.sample_rate().unwrap_or(0),
        b.host_path()
            .map(|p| p.display().to_string())
            .unwrap_or_default()
    );
    println!(
        "rate {} wpm -> ECI speed {} (achieves {} wpm)",
        params.rate.wpm(),
        b.eci_speed(),
        b.effective_wpm()
    );
    let s = b.synthesize(&text)?;
    let rate = u64::from(s.sample_rate.max(1));
    for (range, sample) in &s.words {
        println!(
            "{:>6} ms  {}",
            sample * 1000 / rate,
            &text[range.start as usize..range.end as usize]
        );
    }
    b.synthesize_to_file(&text, &out)?;
    println!(
        "wrote {} ({} ms of audio)",
        out.display(),
        s.samples.len() as u64 * 1000 / rate
    );
    Ok(())
}

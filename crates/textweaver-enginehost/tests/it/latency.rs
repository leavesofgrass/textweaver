//! The stop-and-restart probe on the real audio device (Wave 8b). It
//! plays only silence (zero samples at zero gain), so nothing is heard,
//! but it opens the machine's default output, so it runs only when asked:
//!
//! ```text
//! TEXTWEAVER_AUDIO_PROBE=1 cargo test -p textweaver-enginehost --features playback --test it -- latency --ignored --nocapture
//! ```
//!
//! For each output buffer size it opens the device three times and
//! reports:
//! - "first callback": the median time from a stop and a restart (the
//!   feed cleared and new audio pushed at once) to the audio system's
//!   first callback taking the new audio. This is when the playback
//!   client reports `Started`.
//! - "lead": how far the samples taken run ahead of the sound (the
//!   median over two seconds of playing), which is the audio still to
//!   play in the device's buffer. A restart's first sample is heard about
//!   this long after its first callback, and a stop is heard about this
//!   long after it. It is measured from when the first callback was seen,
//!   so a stream that starts late (a busy machine) makes it low; the
//!   highest of the three opens is reported.
//! - "drift": the worst fall of the lead over two seconds of playing. A
//!   device that ran dry falls behind for good, so a drift near zero means
//!   no underrun.

#![cfg(feature = "playback")]

use std::sync::Arc;
use std::time::{Duration, Instant};

use textweaver_enginehost::audio::{
    DEFAULT_OUTPUT_BUFFER_MS, output_buffer_ms, set_output_buffer_ms,
};
use textweaver_enginehost::{AudioOutput, Feed, Player};

const RATE: u32 = 22_050;
/// Restarts per open.
const RUNS: usize = 11;
/// Opens per buffer size.
const OPENS: usize = 3;

fn median(v: &[f64]) -> f64 {
    let mut v = v.to_vec();
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

/// Milliseconds of audio in `samples`.
fn ms(samples: u64) -> f64 {
    samples as f64 * 1000.0 / f64::from(RATE)
}

/// Waits until the feed's clock passes `at`; the time it did, or `None`
/// after a second.
fn wait_past(feed: &Feed, at: u64) -> Option<Instant> {
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_secs(1) {
        if feed.consumed() > at {
            return Some(Instant::now());
        }
        std::thread::yield_now();
    }
    None
}

/// One open of the device.
struct Open {
    /// Stop-and-restart to first callback, per run, in ms.
    first: Vec<f64>,
    /// Median lead, in ms.
    lead: f64,
    /// Median lead of the last half second less that of the first.
    drift: f64,
}

/// Opens the device with the buffer set now and measures; `None` when no
/// device opens.
fn open_once() -> Option<Open> {
    let feed = Arc::new(Feed::default());
    feed.set_gain(0.0);
    // The feed never runs dry while the lead is measured: every sample
    // the device takes from its first callback on is counted.
    feed.push(&vec![0i16; RATE as usize * 30]);
    let player = Player::start(AudioOutput::Device, Arc::clone(&feed), RATE).ok()?;
    let t0 = Instant::now();
    let opened = loop {
        if player.is_open() && feed.consumed() > 0 {
            break Instant::now();
        }
        if let Some(e) = player.failure() {
            eprintln!("skipped: {e}");
            return None;
        }
        if t0.elapsed() > Duration::from_secs(5) {
            eprintln!("skipped: the device did not open");
            return None;
        }
        std::thread::yield_now();
    };
    // The lead: samples taken against time since the first callback.
    // The first 300 ms are left out: a device waking up starts late.
    let mut leads = Vec::new();
    while opened.elapsed() < Duration::from_millis(2300) {
        let lead = ms(feed.consumed()) - opened.elapsed().as_secs_f64() * 1000.0;
        if opened.elapsed() >= Duration::from_millis(300) {
            leads.push(lead);
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    let edge = leads.len() / 4;
    let drift = median(&leads[leads.len() - edge..]) - median(&leads[..edge]);
    // Stop and restart: the feed cleared and new audio pushed at once
    // (an engine that answers instantly, like the silent test engine),
    // at a different moment of the device's period each time.
    let audio = vec![0i16; RATE as usize];
    let mut first = Vec::new();
    for run in 0..RUNS {
        std::thread::sleep(Duration::from_millis(5 + (run as u64 * 7) % 20));
        let stop = Instant::now();
        feed.clear();
        let at = feed.push(&audio);
        let taken = wait_past(&feed, at)?;
        first.push((taken - stop).as_secs_f64() * 1000.0);
    }
    Some(Open {
        first,
        lead: median(&leads),
        drift,
    })
}

#[test]
#[ignore = "opens the real audio device (silently): TEXTWEAVER_AUDIO_PROBE=1"]
fn stop_and_restart_on_the_real_device() {
    if std::env::var("TEXTWEAVER_AUDIO_PROBE").as_deref() != Ok("1") {
        eprintln!("skipped: set TEXTWEAVER_AUDIO_PROBE=1 to open the real audio device");
        return;
    }
    // The first open only wakes the device up.
    if open_once().is_none() {
        return;
    }
    // 0 is the audio library's own size: the number before Wave 8b.
    for buffer_ms in [0, 40, DEFAULT_OUTPUT_BUFFER_MS, 20] {
        set_output_buffer_ms(buffer_ms);
        let mut first = Vec::new();
        let mut leads = Vec::new();
        let mut worst_drift = 0.0f64;
        for _ in 0..OPENS {
            let Some(o) = open_once() else { return };
            first.extend(o.first);
            leads.push(o.lead);
            worst_drift = worst_drift.min(o.drift);
        }
        let name = if buffer_ms == 0 {
            "library size".to_owned()
        } else {
            format!("{buffer_ms} ms buffer")
        };
        let first = median(&first);
        let lead = leads.iter().copied().fold(f64::MIN, f64::max);
        println!(
            "{name}: to the first sound about {:.1} ms (first callback {first:.1} ms, \
             lead {lead:.1} ms), drift {worst_drift:.1} ms",
            first + lead,
        );
    }
    set_output_buffer_ms(DEFAULT_OUTPUT_BUFFER_MS);
    assert_eq!(output_buffer_ms(), DEFAULT_OUTPUT_BUFFER_MS);
}

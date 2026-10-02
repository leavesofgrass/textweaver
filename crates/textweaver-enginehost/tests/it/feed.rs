//! [`FeedReader`], the way an audio output reads the feed: a stop drops
//! what it had taken, a push during silence plays at once, and a long
//! utterance through a fake device with the small output buffer plays
//! with no gaps and no lost or repeated samples.

use std::sync::Arc;

use textweaver_enginehost::audio::{DEFAULT_OUTPUT_BUFFER_MS, output_buffer_frames};
use textweaver_enginehost::{Feed, FeedReader};

/// The feed's rate (SAPI and most Piper voices).
const RATE: usize = 22_050;
/// A typical device rate (Windows' shared-mode mixer).
const DEVICE_RATE: u32 = 48_000;

/// A sample as the reader hands it out, back to the pushed value.
fn value(v: f32) -> i16 {
    (v * 32768.0).round() as i16
}

#[test]
fn a_push_during_silence_plays_at_the_next_sample() {
    // Before: the output played the rest of a silent 64-sample batch
    // (3 ms at 22,050 Hz) before a restart's first sample, and a reading
    // that ran dry for a moment had a 3 ms hole in it.
    let feed = Arc::new(Feed::default());
    let mut r = FeedReader::new(Arc::clone(&feed));
    assert_eq!(r.read(), None, "empty: silence");
    feed.push(&[100, 200]);
    assert_eq!(r.read().map(value), Some(100));
    assert_eq!(r.read().map(value), Some(200));
    assert_eq!(r.read(), None);
    assert_eq!(r.read(), None);
    feed.push(&[300]);
    assert_eq!(r.read().map(value), Some(300));
    assert_eq!(feed.consumed(), 3, "silence consumes nothing");
}

#[test]
fn a_stop_drops_what_the_reader_had_taken() {
    let feed = Arc::new(Feed::default());
    let mut r = FeedReader::new(Arc::clone(&feed));
    feed.push(&[1; 1000]);
    assert_eq!(r.read().map(value), Some(1));
    // The reader holds the rest of its batch; a stop must not play it.
    feed.clear();
    feed.push(&[7; 10]);
    assert_eq!(r.read().map(value), Some(7), "the new reading, at once");
    feed.clear();
    assert_eq!(r.read(), None, "and nothing after a stop");
}

#[test]
fn a_pause_keeps_what_the_reader_had_taken() {
    // A pause must never lose speech: the samples already taken (and
    // counted on the clock) still play, then silence until resume.
    let feed = Arc::new(Feed::default());
    let mut r = FeedReader::new(Arc::clone(&feed));
    let samples: Vec<i16> = (1..=200).collect();
    feed.push(&samples);
    assert_eq!(r.read().map(value), Some(1));
    feed.set_paused(true);
    let mut heard: Vec<i16> = std::iter::from_fn(|| r.read().map(value)).collect();
    assert_eq!(heard.len(), FeedReader::BATCH - 1, "the batch plays out");
    assert_eq!(r.read(), None, "paused");
    feed.set_paused(false);
    heard.extend(std::iter::from_fn(|| r.read().map(value)));
    assert_eq!(heard, (2..=200).collect::<Vec<i16>>(), "nothing lost");
}

/// A small, fixed pseudo-random sequence (no dependency, the same on
/// every run).
struct Lcg(u64);

impl Lcg {
    /// A number in `lo..hi`.
    fn range(&mut self, lo: u64, hi: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        lo + (self.0 >> 33) % (hi - lo)
    }
}

/// What a fake device heard of one long utterance.
#[derive(Debug, Default, PartialEq)]
struct Heard {
    /// Real samples, in order.
    samples: usize,
    /// Runs of silence between the first and the last real sample.
    gaps: usize,
    /// Samples out of order, lost, or repeated.
    wrong: usize,
}

/// Plays `seconds` of speech through a fake device: every 10 ms (the
/// period of Windows' shared mode) it plays 10 ms from its buffer and
/// tops the buffer up to `buffer_ms` through a [`FeedReader`], as the
/// audio system's callback does. A fake engine synthesizes the speech in
/// chunks of 2,048 to 4,096 samples at a real-time factor of 0.1 to 0.5
/// (Piper measured 0.13 on an idle laptop), its first chunk after 77 ms
/// (ADR-0023), pushing each when it is done. Time is simulated, so the
/// test never waits and never depends on the machine.
fn play_through_fake_device(seconds: usize, buffer_ms: u32) -> Heard {
    let frames = output_buffer_frames(buffer_ms, DEVICE_RATE, None).expect("a buffer");
    // The device's buffer in feed samples (rodio resamples on the way).
    let buffer = frames as usize * RATE / DEVICE_RATE as usize;
    let period = RATE / 100;
    assert!(buffer >= period, "{buffer_ms} ms holds a period");

    let feed = Arc::new(Feed::default());
    let mut reader = FeedReader::new(Arc::clone(&feed));
    let total = seconds * RATE;
    let mut rng = Lcg(8);
    // The engine: the next chunk's first sample, and when it is ready
    // (in samples of simulated time).
    let mut made = 0usize;
    let mut ready = RATE * 77 / 1000;
    let mut chunk = rng.range(2048, 4097) as usize;

    let mut heard = Heard::default();
    let mut silence_since_real = 0usize;
    let mut level = 0usize;
    let mut now = 0usize;
    while heard.samples + heard.wrong < total && now < 2 * total + RATE {
        // The engine pushes every chunk finished by now.
        while made < total && ready <= now {
            let n = chunk.min(total - made);
            let samples: Vec<i16> = (made..made + n).map(|i| (i % 30_000) as i16 + 1).collect();
            feed.push(&samples);
            made += n;
            chunk = rng.range(2048, 4097) as usize;
            let rtf = rng.range(10, 51) as usize;
            ready += chunk * rtf / 100;
        }
        // The device plays a period, then its callback tops it up.
        level = level.saturating_sub(period);
        for _ in level..buffer {
            match reader.read() {
                Some(v) => {
                    if silence_since_real > 0 && heard.samples > 0 {
                        heard.gaps += 1;
                    }
                    silence_since_real = 0;
                    if value(v) == (heard.samples % 30_000) as i16 + 1 {
                        heard.samples += 1;
                    } else {
                        heard.wrong += 1;
                    }
                }
                None => silence_since_real += 1,
            }
        }
        level = buffer;
        now += period;
    }
    heard
}

#[test]
fn a_long_utterance_plays_with_no_gaps_through_the_small_buffer() {
    // A minute of speech, with the default buffer and with the smallest
    // size tried: every sample once, in order, and not one gap.
    for buffer_ms in [DEFAULT_OUTPUT_BUFFER_MS, 20] {
        let heard = play_through_fake_device(60, buffer_ms);
        assert_eq!(
            heard,
            Heard {
                samples: 60 * RATE,
                gaps: 0,
                wrong: 0,
            },
            "{buffer_ms} ms buffer"
        );
    }
}

#[test]
fn output_buffer_frames_follow_the_device() {
    assert_eq!(output_buffer_frames(30, 48_000, None), Some(1440));
    assert_eq!(output_buffer_frames(30, 44_100, None), Some(1323));
    // Inside the device's own limits.
    assert_eq!(
        output_buffer_frames(30, 48_000, Some((2048, 4096))),
        Some(2048)
    );
    assert_eq!(
        output_buffer_frames(30, 48_000, Some((64, 1024))),
        Some(1024)
    );
    // A range that says nothing (WASAPI without offload: 0 to the max).
    assert_eq!(
        output_buffer_frames(30, 48_000, Some((0, u32::MAX))),
        Some(1440)
    );
    assert_eq!(output_buffer_frames(30, 48_000, Some((0, 0))), Some(1440));
    // 0 leaves the size to the audio library.
    assert_eq!(output_buffer_frames(0, 48_000, None), None);
    assert_eq!(output_buffer_frames(30, 0, None), None);
}

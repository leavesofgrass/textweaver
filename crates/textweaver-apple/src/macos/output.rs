//! Audio output for the `avspeech` backend.
//!
//! Each chunk of synthesized samples is appended to the output with a
//! counter that the output advances as the samples are consumed, so the
//! backend knows exactly how far playback has got in each utterance: that
//! is the audio clock its word events are timed against.
//!
//! - [`RodioOut`] plays through the default output device (rodio with the
//!   `playback` feature); the mixer thread pulls samples and advances the
//!   counters.
//! - [`SilentOut`] plays nothing and advances the counters by wall-clock
//!   time (`realtime`) or at once. Tests use it: nothing is ever played
//!   aloud in tests.

use std::collections::VecDeque;
use std::num::NonZero;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// A sink for synthesized audio with per-chunk progress counters.
pub(crate) trait AudioOut {
    /// Queues mono `samples` at `rate` Hz after everything already queued;
    /// `played` is advanced by one for every sample consumed.
    fn append(&mut self, samples: Vec<f32>, rate: u32, played: Arc<AtomicU64>);
    /// Pauses playback where it is.
    fn pause(&mut self);
    /// Resumes playback.
    fn resume(&mut self);
    /// Drops everything queued and playing.
    fn clear(&mut self);
    /// Sets the volume, `0.0..=1.0`, immediately.
    fn set_volume(&mut self, volume: f32);
    /// Advances clocks that are not driven by a device (called from `poll`).
    fn tick(&mut self) {}
}

/// A chunk of samples that counts what has been consumed.
struct CountingSource {
    samples: Vec<f32>,
    pos: usize,
    rate: NonZero<u32>,
    played: Arc<AtomicU64>,
}

impl Iterator for CountingSource {
    type Item = rodio::Sample;

    fn next(&mut self) -> Option<Self::Item> {
        let s = *self.samples.get(self.pos)?;
        self.pos += 1;
        self.played.fetch_add(1, Ordering::Relaxed);
        Some(s)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.samples.len() - self.pos;
        (n, Some(n))
    }
}

impl rodio::Source for CountingSource {
    fn current_span_len(&self) -> Option<usize> {
        Some(self.samples.len() - self.pos)
    }

    fn channels(&self) -> rodio::ChannelCount {
        NonZero::<u16>::MIN
    }

    fn sample_rate(&self) -> rodio::SampleRate {
        self.rate
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs_f64(
            self.samples.len() as f64 / f64::from(self.rate.get()),
        ))
    }
}

/// Plays through the default output device.
pub(crate) struct RodioOut {
    // Field order matters: the player must drop before the device sink.
    player: rodio::Player,
    device: rodio::MixerDeviceSink,
    volume: f32,
}

impl RodioOut {
    /// Opens the default output device.
    pub(crate) fn open() -> Result<Self, String> {
        let mut device =
            rodio::DeviceSinkBuilder::open_default_sink().map_err(|e| e.to_string())?;
        device.log_on_drop(false);
        let player = rodio::Player::connect_new(device.mixer());
        Ok(RodioOut {
            player,
            device,
            volume: 1.0,
        })
    }
}

impl AudioOut for RodioOut {
    fn append(&mut self, samples: Vec<f32>, rate: u32, played: Arc<AtomicU64>) {
        let Some(rate) = NonZero::new(rate) else {
            return;
        };
        self.player.append(CountingSource {
            samples,
            pos: 0,
            rate,
            played,
        });
    }

    fn pause(&mut self) {
        self.player.pause();
    }

    fn resume(&mut self) {
        self.player.play();
    }

    fn clear(&mut self) {
        // A fresh player is the quickest way to drop everything queued.
        self.player.stop();
        self.player = rodio::Player::connect_new(self.device.mixer());
        self.player.set_volume(self.volume);
    }

    fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        self.player.set_volume(self.volume);
    }
}

/// Plays nothing; consumes samples by the clock.
pub(crate) struct SilentOut {
    realtime: bool,
    queue: VecDeque<(u64, u32, Arc<AtomicU64>)>,
    paused: bool,
    last: Instant,
    /// Fractional samples carried between ticks.
    carry: f64,
}

impl SilentOut {
    /// A silent output; `realtime` consumes samples at their sample rate,
    /// otherwise everything is consumed on the next tick.
    pub(crate) fn new(realtime: bool) -> Self {
        SilentOut {
            realtime,
            queue: VecDeque::new(),
            paused: false,
            last: Instant::now(),
            carry: 0.0,
        }
    }
}

impl AudioOut for SilentOut {
    fn append(&mut self, samples: Vec<f32>, rate: u32, played: Arc<AtomicU64>) {
        if self.queue.is_empty() {
            self.last = Instant::now();
            self.carry = 0.0;
        }
        self.queue
            .push_back((samples.len() as u64, rate.max(1), played));
    }

    fn pause(&mut self) {
        self.tick();
        self.paused = true;
    }

    fn resume(&mut self) {
        self.paused = false;
        self.last = Instant::now();
    }

    fn clear(&mut self) {
        self.queue.clear();
        self.carry = 0.0;
    }

    fn set_volume(&mut self, _volume: f32) {}

    fn tick(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last).as_secs_f64();
        self.last = now;
        if self.paused {
            return;
        }
        if !self.realtime {
            while let Some((len, _, played)) = self.queue.pop_front() {
                played.fetch_add(len, Ordering::Relaxed);
            }
            return;
        }
        let mut secs = elapsed + self.carry;
        self.carry = 0.0;
        while let Some((len, rate, played)) = self.queue.front_mut() {
            let can = (secs * f64::from(*rate)).floor() as u64;
            let take = can.min(*len);
            played.fetch_add(take, Ordering::Relaxed);
            *len -= take;
            secs -= take as f64 / f64::from(*rate);
            if *len > 0 {
                self.carry = secs.max(0.0);
                return;
            }
            self.queue.pop_front();
        }
    }
}

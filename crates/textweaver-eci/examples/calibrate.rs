//! Measures an ECI engine's rate and pitch scales for
//! `textweaver_eci::calibration`.
//!
//! ```text
//! cargo run -p textweaver-eci --example calibrate [-- --host PATH]
//! ```
//!
//! Needs a licensed engine (in the dev container: the Voxin overlay). Talks
//! to the host directly over the protocol so it can set raw ECI speed and
//! pitch-baseline values, synthesizes into memory, and never plays audio.
//! Prints, for each speed, the words per minute of a fixed passage, and for
//! each pitch baseline, the median fundamental frequency of a sentence.

use std::io::BufReader;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use textweaver_eci::protocol::{self, Reply, Request};
use textweaver_eci::{EciConfig, VoiceParam, words};

const PASSAGE: &str = "The library opened early on Saturday, and the students \
arrived with their notes, their laptops, and a great deal of coffee. Most of \
them had come to study for the final examination in history, which covered \
three centuries of trade, war, and invention. A few had come simply because \
the building was warm and quiet. The librarian walked between the long tables, \
answering questions about the catalogue and reminding everyone that the \
reading room would close at six. By noon the windows were bright, the tables \
were full, and the only sound was the steady turning of pages.";

const PITCH_SENTENCE: &str = "Many rivers run through the valley, and every one of them is wide.";

struct Host {
    stdin: std::process::ChildStdin,
    out: BufReader<std::process::ChildStdout>,
    rate: u32,
    child: std::process::Child,
}

impl Host {
    fn start(path: &PathBuf) -> Host {
        let mut child = Command::new(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap_or_else(|e| panic!("cannot start {}: {e}", path.display()));
        let stdin = child.stdin.take().expect("stdin");
        let mut out = BufReader::new(child.stdout.take().expect("stdout"));
        let body = protocol::read_body(&mut out).expect("read").expect("ready");
        let rate = match Reply::decode(&body).expect("decode") {
            Reply::Ready {
                sample_rate,
                version,
                presets,
                ..
            } => {
                println!("engine: ECI {version}, {sample_rate} Hz");
                for (i, p) in presets.iter().enumerate() {
                    println!("  preset {}: {:?} {:?}", i + 1, p.name, p.params);
                }
                sample_rate
            }
            other => panic!("host failed: {other:?}"),
        };
        Host {
            stdin,
            out,
            rate,
            child,
        }
    }

    fn send(&mut self, r: &Request) {
        protocol::write_frame(&mut self.stdin, &r.encode()).expect("write");
    }

    fn set(&mut self, p: VoiceParam, value: i32) {
        self.send(&Request::SetVoiceParam {
            param: p as u8,
            value,
        });
    }

    /// Samples and (index, sample) marks for `text`.
    fn synth(&mut self, text: &str) -> (Vec<i16>, Vec<(u32, u64)>) {
        let ws = words::words(text);
        self.send(&Request::Speak {
            token: 1,
            pieces: words::pieces(text, &ws),
        });
        let mut samples = Vec::new();
        let mut marks = Vec::new();
        loop {
            let body = protocol::read_body(&mut self.out)
                .expect("read")
                .expect("frame");
            match Reply::decode(&body).expect("decode") {
                Reply::Audio { samples: s, .. } => samples.extend(s),
                Reply::Mark { index, sample, .. } => marks.push((index, sample)),
                Reply::End { .. } => return (samples, marks),
                Reply::Error { message, .. } => panic!("engine error: {message}"),
                Reply::Ready { .. } | Reply::Dictionary { .. } => {}
            }
        }
    }
}

/// Median F0 in Hz over voiced 40 ms frames (normalized autocorrelation).
fn median_f0(samples: &[i16], rate: u32) -> f64 {
    let x: Vec<f64> = samples.iter().map(|&s| f64::from(s)).collect();
    let frame = (rate as usize) / 25;
    let hop = (rate as usize) / 100;
    let (min_lag, max_lag) = ((rate / 500) as usize, (rate / 40) as usize);
    let peak = x.iter().fold(0.0f64, |m, v| m.max(v.abs()));
    let mut f0s = Vec::new();
    let mut i = 0;
    while i + frame + max_lag < x.len() {
        let w = &x[i..i + frame + max_lag];
        let energy: f64 = w[..frame].iter().map(|v| v * v).sum::<f64>() / frame as f64;
        if energy.sqrt() > peak * 0.1 {
            let rs: Vec<f64> = (min_lag..=max_lag)
                .map(|lag| {
                    let (mut num, mut d0, mut d1) = (0.0, 0.0, 0.0);
                    for k in 0..frame {
                        num += w[k] * w[k + lag];
                        d0 += w[k] * w[k];
                        d1 += w[k + lag] * w[k + lag];
                    }
                    num / (d0 * d1).sqrt().max(1e-9)
                })
                .collect();
            let best = rs.iter().copied().fold(0.0f64, f64::max);
            // The shortest lag that is a local peak close to the best one
            // (longer lags at multiples of the period are octave errors).
            let pick = (1..rs.len().saturating_sub(1))
                .find(|&j| rs[j] >= 0.85 * best && rs[j] >= rs[j - 1] && rs[j] >= rs[j + 1]);
            if let (true, Some(j)) = (best > 0.6, pick) {
                f0s.push(f64::from(rate) / (min_lag + j) as f64);
            }
        }
        i += hop;
    }
    f0s.sort_by(f64::total_cmp);
    f0s.get(f0s.len() / 2).copied().unwrap_or(0.0)
}

fn main() {
    let mut args = std::env::args().skip(1);
    let host = match (args.next().as_deref(), args.next()) {
        (Some("--host"), Some(p)) => PathBuf::from(p),
        _ => textweaver_eci::host_candidates(&EciConfig::default())
            .into_iter()
            .next()
            .expect("no textweaver-eci-host found; build it or pass --host"),
    };
    let mut h = Host::start(&host);
    let n_words = words::words(PASSAGE).len();

    println!("\nrate: {n_words}-word passage, preset 1 (Reed)");
    println!("| ECI speed | wpm |\n|---:|---:|");
    let mut speeds: Vec<i32> = (0..50).step_by(10).collect();
    speeds.extend((50..=140).step_by(5));
    speeds.extend([160, 180, 200, 225, 250]);
    for speed in speeds {
        h.set(VoiceParam::Speed, speed);
        let (samples, marks) = h.synth(PASSAGE);
        // From the first word's mark to the end of the audio, so leading
        // silence does not count.
        let first = marks.first().map_or(0, |m| m.1);
        let secs = (samples.len() as u64 - first) as f64 / f64::from(h.rate);
        let wpm = n_words as f64 / (secs / 60.0);
        println!("| {speed} | {wpm:.0} |");
    }
    h.set(VoiceParam::Speed, 50);

    println!("\npitch: median F0 of one sentence, preset 1 (Reed)");
    println!("| baseline | F0 Hz | semitones vs 65 |\n|---:|---:|---:|");
    h.set(VoiceParam::PitchBaseline, 65);
    let (s, _) = h.synth(PITCH_SENTENCE);
    let reference = median_f0(&s, h.rate);
    for baseline in (0..=100).step_by(5) {
        h.set(VoiceParam::PitchBaseline, baseline);
        let (s, _) = h.synth(PITCH_SENTENCE);
        let f0 = median_f0(&s, h.rate);
        let st = 12.0 * (f0 / reference).log2();
        println!("| {baseline} | {f0:.1} | {st:+.2} |");
    }
    h.send(&Request::Quit);
    let _ = h.child.wait();
}

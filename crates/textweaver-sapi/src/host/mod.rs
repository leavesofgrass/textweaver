//! The host process, `textweaver-sapi-host` (ADR-0009).
//!
//! One binary, built twice: x64 for 64-bit voices and x86 for 32-bit-only
//! voices. It speaks the [`protocol`](crate::protocol) on stdin and stdout
//! and logs to stderr.
//!
//! ```text
//! textweaver-sapi-host [--engine sapi|fake] [--list-voices] [--category sapi|onecore|<registry path>]
//!                      [--report-arch x64|x86]
//! ```
//!
//! - `--engine fake`: a deterministic test engine (no SAPI): each
//!   whitespace-separated word becomes a word event and a short tone, with
//!   a little synthesis time per word so stop can land mid-utterance. The
//!   text `__crash__` makes it exit abruptly and `__fail__` fails the
//!   utterance (tests of host death and engine errors). `--report-arch`
//!   makes it claim an architecture, so one test binary can stand in for
//!   both hosts.
//! - `--list-voices`: write `Ready`, one `Voice` per token of the category,
//!   and exit. Tokens are read from the registry; no engine is loaded.
//! - `--category`: the token category to list (`sapi`, the default, is
//!   `HKLM\SOFTWARE\Microsoft\Speech\Voices`; `onecore` is
//!   `HKLM\SOFTWARE\Microsoft\Speech_OneCore\Voices`).
//!
//! Threads: the main thread owns the engine (COM is initialized on it). A
//! reader thread decodes requests from stdin; a `Stop` bumps a shared stop
//! generation at once (so it reaches an utterance mid-synthesis), and every
//! other request is queued, stamped with the stop generation current when it
//! arrived. A `Speak` whose stamp is older than the current generation was
//! received before a `Stop` and ends as `Aborted` without synthesis.

#[cfg(windows)]
mod com;

use std::io::{self, BufReader, BufWriter};
use std::process::ExitCode;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::protocol::{self, EndStatus, PROTOCOL_VERSION, Reply, Request, VoiceToken};
use crate::voices::Arch;

/// Sample rate of every host's output (16-bit mono PCM). SAPI converts each
/// engine's native format to it.
pub const SAMPLE_RATE: u32 = 22050;

/// The registry category of SAPI5 voices.
pub const CATEGORY_SAPI: &str = r"HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Speech\Voices";
/// The registry category of OneCore voices.
pub const CATEGORY_ONECORE: &str = r"HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Speech_OneCore\Voices";

/// The host's stdout, shared by the main thread and the engine's audio
/// thread. Each frame is written whole and flushed.
#[derive(Debug)]
pub struct Out {
    w: Mutex<BufWriter<io::Stdout>>,
}

impl Out {
    fn new() -> Self {
        Out {
            w: Mutex::new(BufWriter::with_capacity(64 * 1024, io::stdout())),
        }
    }

    /// Sends one reply. Write failures (the backend went away) are logged;
    /// the reader thread notices the closed pipe and ends the host.
    pub fn send(&self, reply: &Reply) {
        let frame = reply.encode();
        let mut w = self.w.lock().unwrap_or_else(|e| e.into_inner());
        if let Err(e) = protocol::write_frame(&mut *w, &frame) {
            eprintln!("sapi host: cannot write reply: {e}");
        }
    }
}

/// What an engine does for the host loop.
trait Engine {
    fn set_voice(&mut self, token_id: &str) -> Result<(), String>;
    fn set_rate(&mut self, rate: i8) -> Result<(), String>;
    /// Synthesizes `text`, sending `Audio` and `Word` frames for `token`
    /// through `out` as they are produced. Returns how the utterance ended
    /// and how many samples were sent. `stopped` turns true when a `Stop`
    /// arrives.
    fn speak(
        &mut self,
        token: u64,
        text: &str,
        pitch: i8,
        out: &Out,
        stopped: &dyn Fn() -> bool,
    ) -> Result<(EndStatus, u64), String>;
    fn voices(&self, category: &str) -> Result<Vec<VoiceToken>, String>;
}

struct Args {
    fake: bool,
    list: bool,
    category: String,
    arch: Arch,
}

fn parse_args() -> Result<Args, String> {
    let mut a = Args {
        fake: false,
        list: false,
        category: CATEGORY_SAPI.to_owned(),
        arch: Arch::native(),
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--engine" => match it.next().as_deref() {
                Some("fake") => a.fake = true,
                Some("sapi") => a.fake = false,
                other => return Err(format!("unknown engine {other:?}")),
            },
            "--list-voices" => a.list = true,
            "--report-arch" => {
                a.arch = it
                    .next()
                    .as_deref()
                    .and_then(Arch::parse)
                    .ok_or("--report-arch needs x64 or x86")?;
            }
            "--category" => {
                a.category = match it.next() {
                    Some(c) if c == "sapi" => CATEGORY_SAPI.to_owned(),
                    Some(c) if c == "onecore" => CATEGORY_ONECORE.to_owned(),
                    Some(c) => c,
                    None => return Err("--category needs a value".into()),
                }
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(a)
}

/// Entry point of `textweaver-sapi-host`.
pub fn main() -> ExitCode {
    let out = Arc::new(Out::new());
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            out.send(&Reply::Error {
                token: 0,
                message: e,
            });
            return ExitCode::from(2);
        }
    };
    let engine: Result<Box<dyn Engine>, String> = if args.fake {
        Ok(Box::new(FakeEngine {
            arch: args.arch,
            ..FakeEngine::default()
        }))
    } else {
        sapi_engine(Arc::clone(&out))
    };
    let mut engine = match engine {
        Ok(e) => e,
        Err(e) => {
            out.send(&Reply::Error {
                token: 0,
                message: e,
            });
            return ExitCode::FAILURE;
        }
    };
    out.send(&Reply::Ready {
        protocol: PROTOCOL_VERSION,
        sample_rate: SAMPLE_RATE,
        arch: if args.fake { args.arch } else { Arch::native() }
            .as_str()
            .to_owned(),
        engine: if args.fake { "fake" } else { "sapi" }.to_owned(),
    });
    if args.list {
        return match engine.voices(&args.category) {
            Ok(list) => {
                for v in list {
                    out.send(&Reply::Voice(v));
                }
                ExitCode::SUCCESS
            }
            Err(e) => {
                out.send(&Reply::Error {
                    token: 0,
                    message: e,
                });
                ExitCode::FAILURE
            }
        };
    }
    serve(engine.as_mut(), &out);
    ExitCode::SUCCESS
}

#[cfg(windows)]
fn sapi_engine(out: Arc<Out>) -> Result<Box<dyn Engine>, String> {
    Ok(Box::new(com::SapiEngine::new(out, SAMPLE_RATE)?))
}

#[cfg(not(windows))]
fn sapi_engine(_out: Arc<Out>) -> Result<Box<dyn Engine>, String> {
    Err("SAPI5 is only available on Windows".into())
}

/// Reads requests until `Quit` or end of input.
fn serve(engine: &mut dyn Engine, out: &Out) {
    let stop_gen = Arc::new(AtomicU64::new(0));
    let (tx, rx) = mpsc::channel::<(u64, Request)>();
    {
        let stop_gen = Arc::clone(&stop_gen);
        let spawned = std::thread::Builder::new()
            .name("sapi-host-stdin".into())
            .spawn(move || {
                let mut r = BufReader::new(io::stdin());
                loop {
                    let req = match protocol::read_body(&mut r) {
                        Ok(Some(body)) => match Request::decode(&body) {
                            Ok(req) => req,
                            Err(e) => {
                                eprintln!("sapi host: bad request: {e}");
                                continue;
                            }
                        },
                        Ok(None) => Request::Quit,
                        Err(e) => {
                            eprintln!("sapi host: input failed: {e}");
                            Request::Quit
                        }
                    };
                    if req == Request::Stop {
                        stop_gen.fetch_add(1, Ordering::SeqCst);
                        continue;
                    }
                    let quit = req == Request::Quit;
                    if tx.send((stop_gen.load(Ordering::SeqCst), req)).is_err() || quit {
                        return;
                    }
                }
            });
        if let Err(e) = spawned {
            out.send(&Reply::Error {
                token: 0,
                message: format!("cannot start the input thread: {e}"),
            });
            return;
        }
    }
    while let Ok((stamp, req)) = rx.recv() {
        match req {
            Request::Speak { token, text, pitch } => {
                if stop_gen.load(Ordering::SeqCst) != stamp {
                    out.send(&Reply::End {
                        token,
                        status: EndStatus::Aborted,
                        samples: 0,
                    });
                    continue;
                }
                let stopped = || stop_gen.load(Ordering::SeqCst) != stamp;
                let (status, samples) = match engine.speak(token, &text, pitch, out, &stopped) {
                    Ok(r) => r,
                    Err(message) => {
                        out.send(&Reply::Error { token, message });
                        (EndStatus::Failed, 0)
                    }
                };
                out.send(&Reply::End {
                    token,
                    status,
                    samples,
                });
            }
            Request::SetVoice { token_id } => {
                if let Err(message) = engine.set_voice(&token_id) {
                    out.send(&Reply::Error { token: 0, message });
                }
            }
            Request::SetRate { rate } => {
                if let Err(message) = engine.set_rate(rate) {
                    out.send(&Reply::Error { token: 0, message });
                }
            }
            Request::Stop => {}
            Request::Quit => break,
        }
    }
}

/// Text prepared for `ISpVoice::Speak`, and the way back from SAPI's event
/// positions to UTF-16 positions in the plain text.
///
/// Pitch 0 speaks the plain text with `SPF_IS_NOT_XML`, so positions are
/// already plain-text positions. Any other pitch wraps the XML-escaped text
/// in `<pitch absmiddle="n">…</pitch>`. Measured on 2026-09-25 with
/// Microsoft Zira: in XML input SAPI counts markup at its raw length but
/// each entity (`&amp;`, `&lt;`, `&gt;`) as the one character it stands
/// for, so a plain-text position is the event position minus the length of
/// the opening tag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpeakText {
    /// Nul-terminated UTF-16 for `Speak`.
    pub wide: Vec<u16>,
    /// UTF-16 units before the plain text in SAPI's position space (the
    /// opening tag), or `None` for plain text.
    head: Option<u32>,
    /// Length of the plain text in UTF-16 units.
    plain_len: u32,
}

impl SpeakText {
    /// Prepares `text` at SAPI pitch `pitch` (`-10..=10`).
    pub fn new(text: &str, pitch: i8) -> SpeakText {
        let plain_len = u32::try_from(text.encode_utf16().count()).unwrap_or(u32::MAX);
        if pitch == 0 {
            return SpeakText {
                wide: text.encode_utf16().chain(Some(0)).collect(),
                head: None,
                plain_len,
            };
        }
        let head = format!("<pitch absmiddle=\"{}\">", pitch.clamp(-10, 10));
        let mut xml = head.clone();
        for c in text.chars() {
            match c {
                '&' => xml.push_str("&amp;"),
                '<' => xml.push_str("&lt;"),
                '>' => xml.push_str("&gt;"),
                c => xml.push(c),
            }
        }
        xml.push_str("</pitch>");
        SpeakText {
            wide: xml.encode_utf16().chain(Some(0)).collect(),
            head: Some(u32::try_from(head.len()).unwrap_or(0)),
            plain_len,
        }
    }

    /// True when the text is SAPI XML.
    pub fn xml(&self) -> bool {
        self.head.is_some()
    }

    /// Maps an event's position and length to the plain text, clamped to it.
    pub fn to_plain(&self, pos: u32, len: u32) -> (u32, u32) {
        let pos = match self.head {
            Some(h) => pos.saturating_sub(h),
            None => pos,
        }
        .min(self.plain_len);
        (pos, len.min(self.plain_len - pos))
    }
}

/// UTF-16 position and length of each whitespace-separated word.
fn utf16_words(text: &str) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    let mut unit = 0u32;
    let mut start: Option<u32> = None;
    for c in text.chars() {
        if c.is_whitespace() {
            if let Some(s) = start.take() {
                out.push((s, unit - s));
            }
        } else if start.is_none() {
            start = Some(unit);
        }
        unit += u32::try_from(c.len_utf16()).unwrap_or(1);
    }
    if let Some(s) = start {
        out.push((s, unit - s));
    }
    out
}

/// The test engine: no SAPI, deterministic output.
///
/// Each word is `SAMPLE_RATE * 60 / wpm` samples of a quiet 220 Hz tone
/// followed by nothing (so word starts are exact), where `wpm` is
/// [`FAKE_WPM_AT_RATE_0`] scaled by SAPI's rate curve (`3^(rate/10)`).
/// Synthesis takes about 2 ms per word, so a `Stop` can land mid-utterance.
#[derive(Debug, Default)]
struct FakeEngine {
    rate: i8,
    voice: String,
    arch: Arch,
}

/// The fake engine's rate at SAPI rate 0.
pub const FAKE_WPM_AT_RATE_0: f64 = 200.0;

impl Engine for FakeEngine {
    fn set_voice(&mut self, token_id: &str) -> Result<(), String> {
        if token_id.contains("missing") {
            return Err(format!("no such voice: {token_id}"));
        }
        self.voice = token_id.to_owned();
        Ok(())
    }

    fn set_rate(&mut self, rate: i8) -> Result<(), String> {
        self.rate = rate.clamp(-10, 10);
        Ok(())
    }

    fn speak(
        &mut self,
        token: u64,
        text: &str,
        _pitch: i8,
        out: &Out,
        stopped: &dyn Fn() -> bool,
    ) -> Result<(EndStatus, u64), String> {
        if text.trim() == "__crash__" {
            eprintln!("sapi host (fake): crashing on request");
            std::process::exit(3);
        }
        if text.trim() == "__fail__" {
            return Err("the fake engine failed on request".into());
        }
        let wpm = FAKE_WPM_AT_RATE_0 * 3f64.powf(f64::from(self.rate) / 10.0);
        let per_word = (f64::from(SAMPLE_RATE) * 60.0 / wpm) as usize;
        let mut sent = 0u64;
        for (start, len) in utf16_words(text) {
            if stopped() {
                return Ok((EndStatus::Aborted, sent));
            }
            out.send(&Reply::Word {
                token,
                start,
                len,
                sample: sent,
            });
            let samples: Vec<i16> = (0..per_word)
                .map(|i| {
                    let t = i as f64 / f64::from(SAMPLE_RATE);
                    ((t * 220.0 * std::f64::consts::TAU).sin() * 1000.0) as i16
                })
                .collect();
            // Two frames per word, as a real engine streams.
            for chunk in samples.chunks(per_word.div_ceil(2).max(1)) {
                out.send(&Reply::Audio {
                    token,
                    samples: chunk.to_vec(),
                });
            }
            sent += per_word as u64;
            std::thread::sleep(Duration::from_millis(2));
        }
        Ok((EndStatus::Done, sent))
    }

    fn voices(&self, category: &str) -> Result<Vec<VoiceToken>, String> {
        let arch = self.arch;
        let voice = |key: &str, name: &str, gender: &str| VoiceToken {
            token_id: format!("{category}\\Tokens\\{key}"),
            name: name.to_owned(),
            language: "409".to_owned(),
            gender: gender.to_owned(),
            vendor: "textweaver".to_owned(),
        };
        let mut v = vec![voice("FAKE_ALPHA", "Fake Alpha", "Female")];
        // Each architecture also has a voice of its own, and x86 repeats
        // Alpha, as real registries do.
        match arch {
            Arch::X64 => v.push(voice("FAKE_WIDE", "Fake Wide", "Male")),
            Arch::X86 => v.push(voice("FAKE_NARROW", "Fake Narrow", "Male")),
        }
        Ok(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_word_positions() {
        assert_eq!(
            utf16_words("  Café 😀 x\ty "),
            [(2, 4), (7, 2), (10, 1), (12, 1)]
        );
        assert!(utf16_words("   ").is_empty());
    }

    fn wide_str(t: &SpeakText) -> String {
        String::from_utf16(&t.wide[..t.wide.len() - 1]).unwrap()
    }

    #[test]
    fn plain_text_is_spoken_as_is() {
        let t = SpeakText::new("a <b> & c", 0);
        assert!(!t.xml());
        assert_eq!(wide_str(&t), "a <b> & c");
        assert_eq!(*t.wide.last().unwrap(), 0);
        assert_eq!(t.to_plain(6, 1), (6, 1));
        assert_eq!(t.to_plain(8, 5), (8, 1), "clamped to the text");
    }

    #[test]
    fn pitch_wraps_escaped_text_and_positions_map_back() {
        // Positions SAPI reported for this text at pitch 4 (Zira,
        // 2026-09-25): Tom 21+3, & 25+1, Jerry 27+5, said 34+4, x 40+1,
        // y 42+1, fine 44+4.
        let text = "Tom & Jerry <said> x<y fine.";
        let t = SpeakText::new(text, 4);
        assert!(t.xml());
        assert_eq!(
            wide_str(&t),
            "<pitch absmiddle=\"4\">Tom &amp; Jerry &lt;said&gt; x&lt;y fine.</pitch>"
        );
        let words: Vec<&str> = [
            (21, 3),
            (25, 1),
            (27, 5),
            (34, 4),
            (40, 1),
            (42, 1),
            (44, 4),
        ]
        .iter()
        .map(|&(p, l)| {
            let (s, l) = t.to_plain(p, l);
            &text[s as usize..(s + l) as usize]
        })
        .collect();
        assert_eq!(words, ["Tom", "&", "Jerry", "said", "x", "y", "fine"]);
        assert_eq!(SpeakText::new("x", 99).wide, SpeakText::new("x", 10).wide);
    }
}

//! speech-dispatcher backend (feature `speechd`), speaking SSIP, the
//! Speech Synthesis Interface Protocol, directly over its socket (no
//! libspeechd, so no C library and no `unsafe`).
//!
//! **Word events.** Every utterance is sent as SSML with an index mark
//! before each spoken word (`<mark name="3"/>`), with SSML mode and all
//! notifications on. speech-dispatcher reports each mark as its output
//! module reaches it in the audio (event 700 `INDEX MARK`), and the
//! message's start and end (701 `BEGIN`, 702 `END`, 703 `CANCELED`). Mark
//! `i` maps back to the byte range of spoken word `i` in the utterance, so
//! the backend emits `Word { byte_range, audio_ms: None }` at the moment the
//! word is heard: word events on arrival, without an audio clock.
//!
//! **Messages and replies.** A reader thread splits the socket's input into
//! SSIP messages: event messages (7xx) go to `poll`, every other message
//! is the reply to the one command in flight (SSIP answers commands in
//! order). `speak` waits only for the server's "message queued" reply (a few
//! milliseconds), never for audio (ADR-0003).
//!
//! **Parameters** (ADR-0004). speech-dispatcher takes rate, pitch, and
//! volume on a -100..=100 scale that each output module maps to its engine.
//! Rate is converted with the espeak-ng module's default mapping (-100 is 80
//! wpm, 0 is 170, +100 is 449; `EspeakMinRate`, `EspeakNormalRate`,
//! `EspeakMaxRate` in `espeak-ng.conf`), which is also what
//! `effective_wpm()` reports; other modules map the scale their own way.
//! Pitch is `semitones × 100 / 12` (the espeak-ng module's pitch scale spans
//! an octave each way, like the espeak backend's), volume `percent × 2 -
//! 100`. Voices are the output module's synthesis voices (`LIST
//! SYNTHESIS_VOICES`), selected by name; a voice's variant becomes its tag.
//!
//! **Pause** is native (`PAUSE SELF`, `RESUME SELF`): speech-dispatcher
//! pauses at the next index mark, which is the next word.
//!
//! **Where the server is** ([`SpeechdAddress::from_env`]): `SPEECHD_ADDRESS`
//! (`unix_socket:/path` or `inet_socket:host:port`, as libspeechd reads it),
//! else the default socket `$XDG_RUNTIME_DIR/speech-dispatcher/speechd.sock`.
//! When the default socket does not answer, the backend starts the server
//! with `speech-dispatcher --spawn` (which honors the user's autospawn
//! setting), as libspeechd does, and retries for two seconds.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::ops::Range;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use textweaver_core::{Utterance, UtteranceId, UtteranceKind};

use crate::backend::{
    BackendId, Caps, EventSink, RawEvent, SpeechBackend, SpeechError, Voice, VoiceParams,
};
use crate::pacing::spoken_words;

/// What the speech-dispatcher backend can do.
pub const CAPS: Caps = Caps::WORD_EVENTS
    .union(Caps::SSML_MARKS)
    .union(Caps::PAUSE)
    .union(Caps::PITCH)
    .union(Caps::VOLUME);

/// espeak-ng module rate mapping: wpm at -100, 0, and +100.
pub const RATE_MIN: u16 = 80;
/// wpm at rate 0.
pub const RATE_NORMAL: u16 = 170;
/// wpm at rate +100.
pub const RATE_MAX: u16 = 449;

/// How long to wait for the server's reply to a command.
const REPLY_TIMEOUT: Duration = Duration::from_secs(5);

/// Where speech-dispatcher listens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpeechdAddress {
    /// A Unix domain socket.
    Unix(PathBuf),
    /// A TCP socket.
    Inet {
        /// Host name or address.
        host: String,
        /// Port.
        port: u16,
    },
}

impl SpeechdAddress {
    /// Parses `SPEECHD_ADDRESS` syntax: `unix_socket:/path`,
    /// `unix_socket` (the default path), `inet_socket:host:port`, or
    /// `inet_socket:host` (port 6560).
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        let (method, rest) = s.split_once(':').unwrap_or((s, ""));
        match method {
            "unix_socket" if rest.is_empty() => default_socket().map(SpeechdAddress::Unix),
            "unix_socket" => Some(SpeechdAddress::Unix(PathBuf::from(rest))),
            "inet_socket" => {
                let (host, port) = match rest.rsplit_once(':') {
                    Some((h, p)) => (h, p.parse().ok()?),
                    None => (rest, 6560),
                };
                let host = if host.is_empty() { "127.0.0.1" } else { host };
                Some(SpeechdAddress::Inet {
                    host: host.to_owned(),
                    port,
                })
            }
            _ => None,
        }
    }

    /// `SPEECHD_ADDRESS` if set and valid, else the default Unix socket.
    pub fn from_env() -> Option<Self> {
        if let Ok(v) = std::env::var("SPEECHD_ADDRESS")
            && !v.trim().is_empty()
        {
            return Self::parse(&v);
        }
        default_socket().map(SpeechdAddress::Unix)
    }

    fn is_default(&self) -> bool {
        matches!(self, SpeechdAddress::Unix(p) if Some(p) == default_socket().as_ref())
    }
}

/// `$XDG_RUNTIME_DIR/speech-dispatcher/speechd.sock` (Unix only).
fn default_socket() -> Option<PathBuf> {
    if !cfg!(unix) {
        return None;
    }
    let dir = std::env::var_os("XDG_RUNTIME_DIR")?;
    Some(PathBuf::from(dir).join("speech-dispatcher/speechd.sock"))
}

/// True when speech-dispatcher can probably be reached: its socket exists,
/// an inet address is configured, or the server is installed to be
/// spawned.
pub fn available() -> bool {
    match SpeechdAddress::from_env() {
        Some(SpeechdAddress::Inet { .. }) => true,
        Some(SpeechdAddress::Unix(p)) => {
            p.exists() || crate::backends::on_path("speech-dispatcher").is_some()
        }
        None => false,
    }
}

/// A connected socket of either kind.
#[derive(Debug)]
enum Stream {
    Tcp(TcpStream),
    #[cfg(unix)]
    Unix(std::os::unix::net::UnixStream),
}

impl Stream {
    fn connect(addr: &SpeechdAddress) -> std::io::Result<Self> {
        match addr {
            SpeechdAddress::Inet { host, port } => {
                let s = TcpStream::connect((host.as_str(), *port))?;
                s.set_nodelay(true)?;
                Ok(Stream::Tcp(s))
            }
            #[cfg(unix)]
            SpeechdAddress::Unix(p) => {
                Ok(Stream::Unix(std::os::unix::net::UnixStream::connect(p)?))
            }
            #[cfg(not(unix))]
            SpeechdAddress::Unix(_) => Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "Unix sockets are not available on this system",
            )),
        }
    }

    fn try_clone(&self) -> std::io::Result<Self> {
        match self {
            Stream::Tcp(s) => s.try_clone().map(Stream::Tcp),
            #[cfg(unix)]
            Stream::Unix(s) => s.try_clone().map(Stream::Unix),
        }
    }

    fn shutdown(&self) {
        let _ = match self {
            Stream::Tcp(s) => s.shutdown(Shutdown::Both),
            #[cfg(unix)]
            Stream::Unix(s) => s.shutdown(Shutdown::Both),
        };
    }
}

impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Stream::Tcp(s) => s.read(buf),
            #[cfg(unix)]
            Stream::Unix(s) => s.read(buf),
        }
    }
}

impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Stream::Tcp(s) => s.write(buf),
            #[cfg(unix)]
            Stream::Unix(s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Stream::Tcp(s) => s.flush(),
            #[cfg(unix)]
            Stream::Unix(s) => s.flush(),
        }
    }
}

/// One SSIP message: its code, its data lines (`NNN-data`), and the text
/// of its final line (`NNN text`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    /// The three-digit code.
    pub code: u16,
    /// Data lines, in order.
    pub data: Vec<String>,
    /// The final line's text.
    pub text: String,
}

impl Message {
    /// 2xx: success.
    pub fn is_ok(&self) -> bool {
        (200..300).contains(&self.code)
    }

    /// 7xx: an event notification.
    pub fn is_event(&self) -> bool {
        (700..800).contains(&self.code)
    }
}

/// Splits SSIP input into messages. Lines end with CRLF (a bare LF is
/// accepted); a message ends at the first line whose fourth char is a space.
#[derive(Debug, Default)]
pub struct MessageParser {
    data: Vec<String>,
}

impl MessageParser {
    /// Feeds one line (without its line end); returns a message when the
    /// line completes one. Malformed lines are skipped.
    pub fn line(&mut self, line: &str) -> Option<Message> {
        let line = line.trim_end_matches(['\r', '\n']);
        let code: u16 = line.get(..3)?.parse().ok()?;
        match line.as_bytes().get(3) {
            Some(b'-') => {
                self.data.push(line[4..].to_owned());
                None
            }
            Some(b' ') | None => Some(Message {
                code,
                data: std::mem::take(&mut self.data),
                text: line.get(4..).unwrap_or("").to_owned(),
            }),
            _ => None,
        }
    }
}

/// An event from the server about one message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SsipEvent {
    /// 701: the message started.
    Begin(u64),
    /// 700: the audio reached an index mark.
    Mark(u64, String),
    /// 702: the message ended.
    End(u64),
    /// 703: the message was cancelled.
    Cancelled(u64),
    /// 704 or 705: paused or resumed (no action needed).
    Other(u64),
}

impl SsipEvent {
    /// The event in `m`, if `m` is an event notification.
    pub fn from_message(m: &Message) -> Option<Self> {
        let id: u64 = m.data.first()?.trim().parse().ok()?;
        Some(match m.code {
            700 => SsipEvent::Mark(id, m.data.get(2)?.trim().to_owned()),
            701 => SsipEvent::Begin(id),
            702 => SsipEvent::End(id),
            703 => SsipEvent::Cancelled(id),
            704 | 705 => SsipEvent::Other(id),
            _ => return None,
        })
    }
}

fn escape_xml(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            // Line ends would split the SSIP data block; they read as
            // spaces anyway.
            '\r' | '\n' => out.push(' '),
            c if c.is_control() => out.push(' '),
            c => out.push(c),
        }
    }
}

/// SSML for `text` with a mark before each word in `words` (byte ranges of
/// `text`), named by the word's index.
pub fn ssml_with_marks(text: &str, words: &[Range<u32>]) -> String {
    let mut out = String::with_capacity(text.len() + words.len() * 16 + 16);
    out.push_str("<speak>");
    let mut at = 0usize;
    for (i, w) in words.iter().enumerate() {
        let start = w.start as usize;
        if start < at || start > text.len() {
            continue;
        }
        escape_xml(&text[at..start], &mut out);
        out.push_str(&format!("<mark name=\"{i}\"/>"));
        at = start;
    }
    escape_xml(&text[at..], &mut out);
    out.push_str("</speak>");
    out
}

/// speech-dispatcher's rate (-100..=100) for `wpm`, by the espeak-ng
/// module's default mapping.
pub fn rate_for_wpm(wpm: u16) -> i32 {
    let wpm = i32::from(wpm.clamp(RATE_MIN, RATE_MAX));
    let (min, normal, max) = (
        i32::from(RATE_MIN),
        i32::from(RATE_NORMAL),
        i32::from(RATE_MAX),
    );
    let r = if wpm <= normal {
        (wpm - normal) * 100 / (normal - min)
    } else {
        (wpm - normal) * 100 / (max - normal)
    };
    r.clamp(-100, 100)
}

/// The wpm the espeak-ng module speaks at for speech-dispatcher rate `r`.
pub fn wpm_for_rate(r: i32) -> u16 {
    let r = r.clamp(-100, 100);
    let (min, normal, max) = (
        i32::from(RATE_MIN),
        i32::from(RATE_NORMAL),
        i32::from(RATE_MAX),
    );
    let wpm = if r < 0 {
        normal + (normal - min) * r / 100
    } else {
        normal + (max - normal) * r / 100
    };
    u16::try_from(wpm).unwrap_or(RATE_NORMAL)
}

/// speech-dispatcher's pitch (-100..=100) for a semitone offset.
pub fn pitch_for_semitones(st: i8) -> i32 {
    (i32::from(st) * 100 / 12).clamp(-100, 100)
}

/// speech-dispatcher's volume (-100..=100) for a percentage.
pub fn volume_for_percent(p: u8) -> i32 {
    (i32::from(p.min(100)) * 2 - 100).clamp(-100, 100)
}

/// An utterance speech-dispatcher is holding.
#[derive(Debug)]
struct Sent {
    msg: u64,
    id: UtteranceId,
    words: Vec<Range<u32>>,
}

/// The speech-dispatcher backend.
pub struct SpeechdBackend {
    writer: Stream,
    replies: Receiver<Message>,
    events: Receiver<Message>,
    reader: Option<JoinHandle<()>>,
    sent: VecDeque<Sent>,
    params: VoiceParams,
    applied: Option<VoiceParams>,
    voices: Option<Vec<Voice>>,
}

impl std::fmt::Debug for SpeechdBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpeechdBackend")
            .field("sent", &self.sent.len())
            .field("params", &self.params)
            .finish_non_exhaustive()
    }
}

fn io_err(e: std::io::Error) -> SpeechError {
    SpeechError::Io(e.to_string())
}

fn read_loop(stream: Stream, replies: &Sender<Message>, events: &Sender<Message>) {
    let mut parser = MessageParser::default();
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        let Some(m) = parser.line(&line) else {
            continue;
        };
        let to = if m.is_event() { events } else { replies };
        if to.send(m).is_err() {
            break;
        }
    }
}

impl SpeechdBackend {
    /// Connects to speech-dispatcher at [`SpeechdAddress::from_env`],
    /// spawning the server if its default socket does not answer.
    pub fn connect_default() -> Result<Self, SpeechError> {
        let addr = SpeechdAddress::from_env().ok_or_else(|| {
            SpeechError::Unavailable("speechd", "no speech-dispatcher address".into())
        })?;
        Self::connect(&addr)
    }

    /// Connects to speech-dispatcher at `addr` and sets up the session:
    /// client name, SSML mode, and every notification.
    pub fn connect(addr: &SpeechdAddress) -> Result<Self, SpeechError> {
        let stream = match Stream::connect(addr) {
            Ok(s) => s,
            Err(first) if addr.is_default() => spawn_and_connect(addr).map_err(|e| {
                SpeechError::Unavailable(
                    "speechd",
                    format!("cannot reach speech-dispatcher ({first}; after spawning: {e})"),
                )
            })?,
            Err(e) => {
                return Err(SpeechError::Unavailable(
                    "speechd",
                    format!("cannot reach speech-dispatcher: {e}"),
                ));
            }
        };
        let read_half = stream.try_clone().map_err(io_err)?;
        let (reply_tx, reply_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let reader = std::thread::Builder::new()
            .name("textweaver-speechd".into())
            .spawn(move || read_loop(read_half, &reply_tx, &event_tx))
            .map_err(io_err)?;
        let mut b = SpeechdBackend {
            writer: stream,
            replies: reply_rx,
            events: event_rx,
            reader: Some(reader),
            sent: VecDeque::new(),
            params: VoiceParams::default(),
            applied: None,
            voices: None,
        };
        b.command("SET SELF CLIENT_NAME user:textweaver:main")?;
        b.command("SET SELF SSML_MODE on")?;
        b.command("SET SELF NOTIFICATION ALL on")?;
        if let Err(e) = b.load_voices() {
            log::warn!("speech-dispatcher voice list: {e}");
            b.voices = Some(Vec::new());
        }
        Ok(b)
    }

    fn send_line(&mut self, line: &str) -> Result<(), SpeechError> {
        self.writer
            .write_all(format!("{line}\r\n").as_bytes())
            .and_then(|()| self.writer.flush())
            .map_err(|e| {
                SpeechError::Engine(format!("lost the connection to speech-dispatcher: {e}"))
            })
    }

    fn reply(&mut self) -> Result<Message, SpeechError> {
        match self.replies.recv_timeout(REPLY_TIMEOUT) {
            Ok(m) => Ok(m),
            Err(RecvTimeoutError::Timeout) => Err(SpeechError::Engine(
                "speech-dispatcher did not answer".into(),
            )),
            Err(RecvTimeoutError::Disconnected) => Err(SpeechError::Engine(
                "the connection to speech-dispatcher closed".into(),
            )),
        }
    }

    /// Sends one command and waits for its reply; a non-2xx reply is an
    /// error.
    fn command(&mut self, line: &str) -> Result<Message, SpeechError> {
        self.send_line(line)?;
        let m = self.reply()?;
        if m.is_ok() {
            Ok(m)
        } else {
            Err(SpeechError::Engine(format!(
                "speech-dispatcher refused \"{line}\": {} {}",
                m.code, m.text
            )))
        }
    }

    fn apply_params(&mut self) -> Result<(), SpeechError> {
        if self.applied.as_ref() == Some(&self.params) {
            return Ok(());
        }
        let p = self.params.clone();
        let old = self.applied.take();
        if old.as_ref().map(|o| &o.voice) != Some(&p.voice)
            && let Some(v) = &p.voice
        {
            if let Some(lang) = self
                .voices
                .as_ref()
                .and_then(|vs| vs.iter().find(|x| &x.id == v))
                .and_then(|x| x.languages.first().cloned())
            {
                self.command(&format!("SET SELF LANGUAGE {lang}"))?;
            }
            self.command(&format!("SET SELF SYNTHESIS_VOICE {v}"))?;
        }
        self.command(&format!("SET SELF RATE {}", rate_for_wpm(p.rate.wpm())))?;
        self.command(&format!(
            "SET SELF PITCH {}",
            pitch_for_semitones(p.pitch.semitones())
        ))?;
        self.command(&format!(
            "SET SELF VOLUME {}",
            volume_for_percent(p.volume.percent())
        ))?;
        self.applied = Some(p);
        Ok(())
    }

    fn deliver(&mut self, sink: &mut dyn EventSink) {
        while let Ok(m) = self.events.try_recv() {
            let Some(ev) = SsipEvent::from_message(&m) else {
                continue;
            };
            let msg = match &ev {
                SsipEvent::Begin(i)
                | SsipEvent::Mark(i, _)
                | SsipEvent::End(i)
                | SsipEvent::Cancelled(i)
                | SsipEvent::Other(i) => *i,
            };
            // Messages not (or no longer) ours: stopped earlier, or another
            // connection's.
            let Some(pos) = self.sent.iter().position(|s| s.msg == msg) else {
                continue;
            };
            let id = self.sent[pos].id;
            match ev {
                SsipEvent::Begin(_) => sink.emit(id, RawEvent::Started),
                SsipEvent::Mark(_, name) => {
                    if let Some(r) = name
                        .parse::<usize>()
                        .ok()
                        .and_then(|i| self.sent[pos].words.get(i))
                    {
                        sink.emit(
                            id,
                            RawEvent::Word {
                                byte_range: r.clone(),
                                audio_ms: None,
                            },
                        );
                    }
                }
                SsipEvent::End(_) => {
                    self.sent.remove(pos);
                    sink.emit(id, RawEvent::Finished);
                }
                SsipEvent::Cancelled(_) => {
                    self.sent.remove(pos);
                    sink.emit(id, RawEvent::Cancelled);
                }
                SsipEvent::Other(_) => {}
            }
        }
    }
}

/// Starts speech-dispatcher (`--spawn`) and retries the connection for two
/// seconds.
fn spawn_and_connect(addr: &SpeechdAddress) -> std::io::Result<Stream> {
    let status = std::process::Command::new("speech-dispatcher")
        .arg("--spawn")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;
    log::debug!("speech-dispatcher --spawn: {status}");
    let until = Instant::now() + Duration::from_secs(2);
    loop {
        match Stream::connect(addr) {
            Ok(s) => return Ok(s),
            Err(e) if Instant::now() >= until => return Err(e),
            Err(_) => std::thread::sleep(Duration::from_millis(50)),
        }
    }
}

impl SpeechBackend for SpeechdBackend {
    fn id(&self) -> BackendId {
        "speechd"
    }

    fn capabilities(&self) -> Caps {
        CAPS
    }

    fn voices(&self) -> Result<Vec<Voice>, SpeechError> {
        // Loaded when connecting (`LIST SYNTHESIS_VOICES`).
        Ok(self.voices.clone().unwrap_or_default())
    }

    fn set_params(&mut self, params: &VoiceParams) -> Result<(), SpeechError> {
        self.params = params.clone();
        self.apply_params()
    }

    fn effective_wpm(&self) -> u16 {
        wpm_for_rate(rate_for_wpm(self.params.rate.wpm()))
    }

    fn speak(
        &mut self,
        utterance: &Utterance,
        _sink: &mut dyn EventSink,
    ) -> Result<(), SpeechError> {
        self.apply_params()?;
        let words = if utterance.kind == UtteranceKind::Text {
            spoken_words(&utterance.text)
        } else {
            Vec::new()
        };
        let ssml = ssml_with_marks(&utterance.text, &words);
        self.command("SPEAK")?;
        // The SSML is one line (line ends were escaped to spaces) that
        // starts with "<speak>", so it never needs SSIP's dot escaping.
        self.send_line(&ssml)?;
        self.send_line(".")?;
        let m = self.reply()?;
        if !m.is_ok() {
            return Err(SpeechError::Engine(format!(
                "speech-dispatcher did not queue the text: {} {}",
                m.code, m.text
            )));
        }
        let msg = m
            .data
            .first()
            .and_then(|d| d.trim().parse().ok())
            .ok_or_else(|| SpeechError::Engine("no message id from speech-dispatcher".into()))?;
        self.sent.push_back(Sent {
            msg,
            id: utterance.id,
            words,
        });
        Ok(())
    }

    fn poll(&mut self, sink: &mut dyn EventSink) {
        self.deliver(sink);
    }

    fn stop(&mut self) {
        if let Err(e) = self.command("CANCEL SELF") {
            log::warn!("{e}");
        }
        // Late events for these messages are ignored.
        self.sent.clear();
    }

    fn pause(&mut self) -> Result<(), SpeechError> {
        self.command("PAUSE SELF").map(|_| ())
    }

    fn resume(&mut self) -> Result<(), SpeechError> {
        self.command("RESUME SELF").map(|_| ())
    }
}

impl SpeechdBackend {
    /// Loads the output module's voices (`LIST SYNTHESIS_VOICES`); later
    /// calls to `voices` return them.
    pub fn load_voices(&mut self) -> Result<&[Voice], SpeechError> {
        let m = self.command("LIST SYNTHESIS_VOICES")?;
        let voices = m
            .data
            .iter()
            .filter_map(|line| {
                let mut f = line.split('\t');
                let name = f.next()?.trim();
                if name.is_empty() {
                    return None;
                }
                let lang = f.next().map(str::trim).unwrap_or("");
                let variant = f.next().map(str::trim).unwrap_or("");
                Some(Voice {
                    id: name.to_owned(),
                    name: name.to_owned(),
                    languages: if lang.is_empty() || lang == "none" {
                        Vec::new()
                    } else {
                        vec![lang.to_owned()]
                    },
                    gender: None,
                    tags: if variant.is_empty() || variant == "none" {
                        Vec::new()
                    } else {
                        vec![variant.to_owned()]
                    },
                })
            })
            .collect();
        Ok(self.voices.insert(voices))
    }
}

impl Drop for SpeechdBackend {
    fn drop(&mut self) {
        let _ = self.send_line("QUIT");
        self.writer.shutdown();
        if let Some(r) = self.reader.take() {
            let _ = r.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses() {
        assert_eq!(
            SpeechdAddress::parse("unix_socket:/run/sd.sock"),
            Some(SpeechdAddress::Unix("/run/sd.sock".into()))
        );
        assert_eq!(
            SpeechdAddress::parse("inet_socket:localhost:7000"),
            Some(SpeechdAddress::Inet {
                host: "localhost".into(),
                port: 7000
            })
        );
        assert_eq!(
            SpeechdAddress::parse("inet_socket:10.0.0.2"),
            Some(SpeechdAddress::Inet {
                host: "10.0.0.2".into(),
                port: 6560
            })
        );
        assert_eq!(SpeechdAddress::parse("carrier_pigeon:x"), None);
        assert_eq!(SpeechdAddress::parse("inet_socket:h:notaport"), None);
    }

    #[test]
    fn messages_and_events_parse() {
        let mut p = MessageParser::default();
        assert_eq!(p.line("700-12\r\n"), None);
        assert_eq!(p.line("700-3"), None);
        assert_eq!(p.line("700-5"), None);
        let m = p.line("700 INDEX MARK\r\n").unwrap();
        assert!(m.is_event());
        assert_eq!(
            SsipEvent::from_message(&m),
            Some(SsipEvent::Mark(12, "5".into()))
        );
        let m = p.line("208 OK CLIENT NAME SET").unwrap();
        assert!(m.is_ok() && m.data.is_empty());
        assert_eq!(m.text, "OK CLIENT NAME SET");
        p.line("225-7");
        let m = p.line("225 OK MESSAGE QUEUED").unwrap();
        assert_eq!(m.data, ["7"]);
        assert_eq!(p.line("garbage"), None);
        p.line("702-7");
        p.line("702-1");
        let m = p.line("702 END").unwrap();
        assert_eq!(SsipEvent::from_message(&m), Some(SsipEvent::End(7)));
        let m = p.line("301 ERR NOT A NUMBER").unwrap();
        assert!(!m.is_ok() && !m.is_event());
    }

    #[test]
    fn ssml_marks_every_word_and_escapes() {
        let text = "Tom & \"Jerry\" <3\nfun";
        let words = spoken_words(text);
        let s = ssml_with_marks(text, &words);
        assert_eq!(
            s,
            "<speak><mark name=\"0\"/>Tom &amp; &quot;<mark name=\"1\"/>Jerry&quot; &lt;\
             <mark name=\"2\"/>3 <mark name=\"3\"/>fun</speak>"
        );
        assert_eq!(ssml_with_marks("", &[]), "<speak></speak>");
    }

    #[test]
    fn parameter_mapping() {
        assert_eq!(rate_for_wpm(170), 0);
        assert_eq!(rate_for_wpm(80), -100);
        assert_eq!(rate_for_wpm(50), -100);
        assert_eq!(rate_for_wpm(449), 100);
        assert_eq!(rate_for_wpm(900), 100);
        assert_eq!(rate_for_wpm(265), 34);
        assert_eq!(wpm_for_rate(34), 264);
        assert_eq!(wpm_for_rate(-50), 125);
        for wpm in (80..=449).step_by(7) {
            let back = wpm_for_rate(rate_for_wpm(wpm));
            assert!(back.abs_diff(wpm) <= 3, "{wpm} -> {back}");
        }
        assert_eq!(pitch_for_semitones(0), 0);
        assert_eq!(pitch_for_semitones(12), 100);
        assert_eq!(pitch_for_semitones(-6), -50);
        assert_eq!(volume_for_percent(100), 100);
        assert_eq!(volume_for_percent(50), 0);
        assert_eq!(volume_for_percent(0), -100);
    }
}

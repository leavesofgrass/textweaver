//! `tw cw`: CW (Morse code) tools. `tw cw monitor` prints what a Morse key
//! adapter sends, MIDI messages and key presses and releases, in words
//! with the time each arrived, so the messages of an adapter can be
//! recorded once. It only prints: nothing is keyed, sent, or played.
//!
//! MIDI comes through midir (the `cw-midi` feature, on in releases): on
//! Windows through its WinRT backend, which also lists Bluetooth LE MIDI
//! devices such as the N6ARA TinyMIDI; ALSA on Linux; CoreMIDI on macOS.
//! Keys come from the terminal: the Windows console reports releases; on
//! Linux and macOS only terminals with the kitty keyboard protocol
//! (kitty, foot, WezTerm, Alacritty) do, and the command says which.

use std::io::Write;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::{Duration, Instant};

use ratatui::crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, KeyboardEnhancementFlags,
    ModifierKeyCode, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use ratatui::crossterm::{execute, terminal};

/// Arguments for `tw cw`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// What to do.
    #[command(subcommand)]
    pub command: Command,
}

/// `tw cw` subcommands.
#[derive(clap::Subcommand, Debug)]
pub enum Command {
    /// Print MIDI messages and key presses and releases in words, with the
    /// time each arrived, until Escape (or for --seconds). For recording
    /// what a key adapter sends; nothing is played or sent.
    Monitor(MonitorArgs),
}

/// Arguments for `tw cw monitor`.
#[derive(clap::Args, Debug)]
pub struct MonitorArgs {
    /// List the MIDI inputs and stop.
    #[arg(long)]
    pub list: bool,
    /// Listen to MIDI only, not to the keyboard.
    #[arg(long, conflicts_with = "no_midi")]
    pub no_keys: bool,
    /// Listen to the keyboard only, not to MIDI.
    #[arg(long)]
    pub no_midi: bool,
    /// Listen only to MIDI inputs whose name contains this text (any
    /// case), such as "TinyMIDI" or "CTR2".
    #[arg(long, value_name = "TEXT")]
    pub port: Option<String>,
    /// Stop after this many seconds instead of waiting for Escape.
    #[arg(long, value_name = "N")]
    pub seconds: Option<u64>,
}

/// Runs `tw cw`.
pub fn run(args: Args) -> anyhow::Result<()> {
    match args.command {
        Command::Monitor(a) => monitor(&a),
    }
}

/// Seconds since `start`, as "12.345 seconds".
fn at(start: Instant) -> String {
    format!("{:.3} seconds", start.elapsed().as_secs_f64())
}

/// What the usual mappings make of MIDI note `note`: the TinyMIDI and the
/// CTR2 MIDI send dit on 20 and dah on 21 (the CTR2's straight key on 30
/// and PTT on 31); the Vail adapter sends straight key 0, dit 1, dah 2.
fn usual_role(note: u8) -> Option<&'static str> {
    Some(match note {
        20 => "dit in the usual mapping",
        21 => "dah in the usual mapping",
        30 => "straight key in the CTR2 extended mapping",
        31 => "PTT in the CTR2 extended mapping",
        0 => "straight key in the Vail mapping",
        1 => "dit in the Vail mapping",
        2 => "dah in the Vail mapping",
        _ => return None,
    })
}

/// One MIDI message in words, meaning first.
pub fn describe_midi(bytes: &[u8]) -> String {
    let hex = || {
        bytes
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ")
    };
    let Some(&status) = bytes.first() else {
        return "Empty MIDI message".into();
    };
    let channel = (status & 0x0F) + 1;
    let data = |i: usize| bytes.get(i).copied().unwrap_or(0);
    let role = |note: u8| usual_role(note).map_or(String::new(), |r| format!(" ({r})"));
    match status & 0xF0 {
        0x90 if data(2) > 0 => format!(
            "Note {} on{}, channel {channel}, velocity {}",
            data(1),
            role(data(1)),
            data(2)
        ),
        // Note on with velocity 0 is a note off.
        0x90 | 0x80 => format!(
            "Note {} off{}, channel {channel}, velocity {}",
            data(1),
            role(data(1)),
            data(2)
        ),
        0xB0 => format!(
            "Controller {} value {}, channel {channel}",
            data(1),
            data(2)
        ),
        0xC0 => format!("Program change {}, channel {channel}", data(1)),
        0xE0 => {
            let bend = i32::from(data(1)) | (i32::from(data(2)) << 7);
            format!("Pitch bend {}, channel {channel}", bend - 8192)
        }
        _ => format!("Other MIDI message, bytes {}", hex()),
    }
}

/// One key event in words, meaning first.
pub fn describe_key(k: &KeyEvent) -> String {
    let what = match k.code {
        KeyCode::Char(' ') => "Space".to_owned(),
        KeyCode::Char(c) => format!("{c} (character {:04X})", u32::from(c)),
        KeyCode::Modifier(m) => match m {
            ModifierKeyCode::LeftControl => "Left Control".into(),
            ModifierKeyCode::RightControl => "Right Control".into(),
            ModifierKeyCode::LeftShift => "Left Shift".into(),
            ModifierKeyCode::RightShift => "Right Shift".into(),
            ModifierKeyCode::LeftAlt => "Left Alt".into(),
            ModifierKeyCode::RightAlt => "Right Alt".into(),
            other => format!("{other:?}"),
        },
        other => format!("{other}"),
    };
    let kind = match k.kind {
        KeyEventKind::Press => "down",
        KeyEventKind::Release => "up",
        KeyEventKind::Repeat => "repeat",
    };
    let mut mods = Vec::new();
    if k.modifiers.contains(KeyModifiers::CONTROL) {
        mods.push("Control");
    }
    if k.modifiers.contains(KeyModifiers::ALT) {
        mods.push("Alt");
    }
    if k.modifiers.contains(KeyModifiers::SHIFT) {
        mods.push("Shift");
    }
    let held = if mods.is_empty() {
        String::new()
    } else {
        format!(", with {} held", mods.join(" and "))
    };
    format!("Key {what} {kind}{held}")
}

/// Puts the terminal back as it was when dropped.
struct RawMode {
    enhanced: bool,
}

impl Drop for RawMode {
    fn drop(&mut self) {
        if self.enhanced {
            let _ = execute!(std::io::stdout(), PopKeyboardEnhancementFlags);
        }
        let _ = terminal::disable_raw_mode();
    }
}

/// Prints `line` on its own line (raw mode needs the carriage return).
fn say(line: &str) {
    let mut out = std::io::stdout();
    let _ = write!(out, "{line}\r\n");
    let _ = out.flush();
}

fn monitor(args: &MonitorArgs) -> anyhow::Result<()> {
    if args.list {
        return list_midi();
    }
    let start = Instant::now();
    let (tx, rx) = channel::<String>();
    let _midi = if args.no_midi {
        None
    } else {
        Some(connect_midi(args.port.as_deref(), &tx, start)?)
    };
    let keys = !args.no_keys && std::io::IsTerminal::is_terminal(&std::io::stdin());
    if !args.no_keys && !keys {
        println!("Keys: not listening, because the input is not a terminal.");
    }
    let deadline = args.seconds.map(|s| start + Duration::from_secs(s));
    match deadline {
        Some(_) if keys => println!("Listening. Escape stops early."),
        Some(_) => println!("Listening."),
        None if keys => println!("Listening. Escape stops."),
        None => println!("Listening. Control+C stops."),
    }
    let _raw = if keys {
        terminal::enable_raw_mode()?;
        let enhanced = !cfg!(windows)
            && terminal::supports_keyboard_enhancement().unwrap_or(false)
            && execute!(
                std::io::stdout(),
                PushKeyboardEnhancementFlags(
                    KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                        | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
                )
            )
            .is_ok();
        let guard = RawMode { enhanced };
        if cfg!(windows) {
            say("Keys: this console reports presses and releases.");
        } else if enhanced {
            say("Keys: this terminal reports presses and releases.");
        } else {
            say("Keys: this terminal reports presses only, never releases.");
        }
        Some(guard)
    } else {
        None
    };
    run_loop(&rx, keys, deadline, start)?;
    drop(_raw);
    println!("Stopped.");
    Ok(())
}

/// Prints MIDI lines as they come, and key events when `keys`, until
/// Escape, Control+C, or `deadline`.
fn run_loop(
    rx: &Receiver<String>,
    keys: bool,
    deadline: Option<Instant>,
    start: Instant,
) -> anyhow::Result<()> {
    loop {
        while let Ok(line) = rx.try_recv() {
            say(&line);
        }
        if deadline.is_some_and(|d| Instant::now() >= d) {
            return Ok(());
        }
        if !keys {
            std::thread::sleep(Duration::from_millis(5));
            continue;
        }
        if !event::poll(Duration::from_millis(5))? {
            continue;
        }
        if let Event::Key(k) = event::read()? {
            say(&format!("{}, at {}", describe_key(&k), at(start)));
            let stop = k.kind == KeyEventKind::Press
                && (k.code == KeyCode::Esc
                    || (k.code == KeyCode::Char('c')
                        && k.modifiers.contains(KeyModifiers::CONTROL)));
            if stop {
                return Ok(());
            }
        }
    }
}

#[cfg(feature = "cw-midi")]
fn list_midi() -> anyhow::Result<()> {
    let input = midir::MidiInput::new("textweaver monitor")?;
    let ports = input.ports();
    if ports.is_empty() {
        println!("No MIDI inputs. Connect or pair the adapter, then try again.");
    }
    for (i, p) in ports.iter().enumerate() {
        let name = input.port_name(p).unwrap_or_else(|_| "unnamed".into());
        println!("MIDI input {}: {name}", i + 1);
    }
    Ok(())
}

#[cfg(not(feature = "cw-midi"))]
fn list_midi() -> anyhow::Result<()> {
    println!("MIDI is not in this build of textweaver (the cw-midi feature).");
    Ok(())
}

/// Keeps the MIDI connections open; dropping it closes them.
struct MidiGuard(std::sync::Arc<std::sync::atomic::AtomicBool>);

impl Drop for MidiGuard {
    fn drop(&mut self) {
        self.0.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Connects to every MIDI input (or those whose name contains `filter`),
/// each on its own thread, since opening a Bluetooth device that is out
/// of range can take a long time. Each message goes to `tx` in words, as
/// do "listening" and failures. The connections stay open until the
/// guard is dropped.
#[cfg(feature = "cw-midi")]
fn connect_midi(
    filter: Option<&str>,
    tx: &Sender<String>,
    start: Instant,
) -> anyhow::Result<MidiGuard> {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    let names: Vec<String> = {
        let input = midir::MidiInput::new("textweaver monitor")?;
        input
            .ports()
            .iter()
            .map(|p| input.port_name(p).unwrap_or_else(|_| "unnamed".into()))
            .collect()
    };
    let wanted =
        |name: &str| filter.is_none_or(|f| name.to_lowercase().contains(&f.to_lowercase()));
    let stop = Arc::new(AtomicBool::new(false));
    let mut opening = 0;
    for (i, name) in names.iter().enumerate() {
        if !wanted(name) {
            continue;
        }
        opening += 1;
        let (tx, name, stop) = (tx.clone(), name.clone(), Arc::clone(&stop));
        std::thread::spawn(move || {
            let mut input = match midir::MidiInput::new("textweaver monitor") {
                Ok(i) => i,
                Err(e) => {
                    let _ = tx.send(format!("MIDI: cannot open {name}: {e}."));
                    return;
                }
            };
            input.ignore(midir::Ignore::None);
            let Some(port) = input.ports().get(i).cloned() else {
                let _ = tx.send(format!("MIDI: {name} is gone."));
                return;
            };
            let (events, label) = (tx.clone(), name.clone());
            let connection = input.connect(
                &port,
                "textweaver-cw-monitor",
                move |_stamp, bytes, _| {
                    let line = format!("{}, at {}, from {label}", describe_midi(bytes), at(start));
                    let _ = events.send(line);
                },
                (),
            );
            match connection {
                Ok(c) => {
                    let _ = tx.send(format!("MIDI: listening to {name}."));
                    while !stop.load(Ordering::SeqCst) {
                        std::thread::sleep(Duration::from_millis(20));
                    }
                    drop(c);
                }
                Err(e) => {
                    let _ = tx.send(format!("MIDI: cannot open {name}: {e}."));
                }
            }
        });
    }
    if opening == 0 {
        println!(
            "MIDI: no inputs{}. Connect or pair the adapter first; tw cw monitor --list lists them.",
            filter.map_or(String::new(), |f| format!(" named like {f}"))
        );
    } else {
        let inputs = if opening == 1 { "input" } else { "inputs" };
        println!("MIDI: opening {opening} {inputs}; each says when it is listening.");
    }
    Ok(MidiGuard(stop))
}

#[cfg(not(feature = "cw-midi"))]
fn connect_midi(
    _filter: Option<&str>,
    _tx: &Sender<String>,
    _start: Instant,
) -> anyhow::Result<MidiGuard> {
    println!("MIDI: not in this build of textweaver (the cw-midi feature); keys only.");
    Ok(MidiGuard(std::sync::Arc::default()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn midi_messages_are_said_in_words() {
        assert_eq!(
            describe_midi(&[0x90, 20, 127]),
            "Note 20 on (dit in the usual mapping), channel 1, velocity 127"
        );
        assert_eq!(
            describe_midi(&[0x90, 21, 0]),
            "Note 21 off (dah in the usual mapping), channel 1, velocity 0"
        );
        assert_eq!(
            describe_midi(&[0x81, 30, 64]),
            "Note 30 off (straight key in the CTR2 extended mapping), channel 2, velocity 64"
        );
        assert_eq!(
            describe_midi(&[0xB0, 100, 65]),
            "Controller 100 value 65, channel 1"
        );
        assert_eq!(
            describe_midi(&[0x90, 60, 1]),
            "Note 60 on, channel 1, velocity 1"
        );
        assert_eq!(describe_midi(&[0xF8]), "Other MIDI message, bytes F8");
        assert_eq!(describe_midi(&[]), "Empty MIDI message");
    }

    #[test]
    fn keys_are_said_in_words() {
        let k = KeyEvent::new_with_kind(
            KeyCode::Char('['),
            KeyModifiers::NONE,
            KeyEventKind::Release,
        );
        assert_eq!(describe_key(&k), "Key [ (character 005B) up");
        let k = KeyEvent::new_with_kind(
            KeyCode::Modifier(ModifierKeyCode::RightControl),
            KeyModifiers::CONTROL,
            KeyEventKind::Press,
        );
        assert_eq!(
            describe_key(&k),
            "Key Right Control down, with Control held"
        );
    }
}

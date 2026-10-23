//! MIDI inputs for Morse keying: the key adapters that send MIDI (the
//! N6ARA TinyMIDI, the CTR2 MIDI), shared by keying in the reader
//! (crate::cw_keying) and by `tw cw monitor`.
//!
//! Through midir (the `cw-midi` feature, on in releases): on Windows
//! through its WinRT backend, the only Windows MIDI interface that lists
//! Bluetooth LE MIDI devices such as the TinyMIDI; ALSA on Linux; CoreMIDI
//! on macOS. Each input opens on its own thread, since opening a
//! Bluetooth device that is out of range can take a long time; every
//! message is handed on from the MIDI thread as it arrives, with the
//! input's name.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// What happened to an input as it opened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MidiStatus {
    /// The input is open and listened to.
    Listening(String),
    /// The input could not be opened: its name and why.
    Failed(String, String),
}

/// Keeps the MIDI inputs open; dropping it closes them.
#[derive(Debug)]
pub struct MidiListener(Arc<AtomicBool>);

impl MidiListener {
    /// A listener with no inputs.
    pub fn none() -> Self {
        MidiListener(Arc::default())
    }
}

impl Drop for MidiListener {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

/// True when this build reads MIDI (the `cw-midi` feature).
pub const AVAILABLE: bool = cfg!(feature = "cw-midi");

/// The names of the MIDI inputs, in order.
#[cfg(feature = "cw-midi")]
pub fn port_names() -> Result<Vec<String>, String> {
    let input = midir::MidiInput::new("textweaver").map_err(|e| e.to_string())?;
    Ok(input
        .ports()
        .iter()
        .map(|p| input.port_name(p).unwrap_or_else(|_| "unnamed".into()))
        .collect())
}

/// The names of the MIDI inputs: none without the `cw-midi` feature.
#[cfg(not(feature = "cw-midi"))]
pub fn port_names() -> Result<Vec<String>, String> {
    Ok(Vec::new())
}

/// Opens every MIDI input (or those whose name contains `filter`, in any
/// case). `on_message` gets each message's bytes and the input's name, on
/// the MIDI thread, and must be quick; `on_status` hears when each input
/// is listening or failed. Returns the listener, which keeps them open,
/// and how many inputs are opening.
#[cfg(feature = "cw-midi")]
pub fn listen(
    filter: Option<&str>,
    on_message: impl Fn(&str, &[u8]) + Send + Clone + 'static,
    on_status: impl Fn(MidiStatus) + Send + Clone + 'static,
) -> Result<(MidiListener, usize), String> {
    use std::time::Duration;
    let names = port_names()?;
    let wanted =
        |name: &str| filter.is_none_or(|f| name.to_lowercase().contains(&f.to_lowercase()));
    let stop = Arc::new(AtomicBool::new(false));
    let mut opening = 0;
    for (i, name) in names.iter().enumerate() {
        if !wanted(name) {
            continue;
        }
        opening += 1;
        let (name, stop) = (name.clone(), Arc::clone(&stop));
        let (on_message, on_status) = (on_message.clone(), on_status.clone());
        let spawned = std::thread::Builder::new()
            .name("cw-midi-input".into())
            .spawn(move || {
                let mut input = match midir::MidiInput::new("textweaver") {
                    Ok(i) => i,
                    Err(e) => return on_status(MidiStatus::Failed(name, e.to_string())),
                };
                input.ignore(midir::Ignore::None);
                let Some(port) = input.ports().get(i).cloned() else {
                    return on_status(MidiStatus::Failed(name, "it is gone".into()));
                };
                let label = name.clone();
                let connection = input.connect(
                    &port,
                    "textweaver-cw",
                    move |_stamp, bytes, _| on_message(&label, bytes),
                    (),
                );
                match connection {
                    Ok(c) => {
                        on_status(MidiStatus::Listening(name));
                        while !stop.load(Ordering::SeqCst) {
                            std::thread::sleep(Duration::from_millis(20));
                        }
                        drop(c);
                    }
                    Err(e) => on_status(MidiStatus::Failed(name, e.to_string())),
                }
            });
        if let Err(e) = spawned {
            return Err(format!("cannot start a MIDI thread: {e}"));
        }
    }
    Ok((MidiListener(stop), opening))
}

/// Without the `cw-midi` feature: no inputs.
#[cfg(not(feature = "cw-midi"))]
pub fn listen(
    _filter: Option<&str>,
    _on_message: impl Fn(&str, &[u8]) + Send + Clone + 'static,
    _on_status: impl Fn(MidiStatus) + Send + Clone + 'static,
) -> Result<(MidiListener, usize), String> {
    Ok((MidiListener::none(), 0))
}

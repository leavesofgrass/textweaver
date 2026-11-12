//! Fuzz target: join links and join codes as other people send them, pasted
//! or read from a QR code. The text is read as a join link
//! (`JoinLink::parse`), searched for one inside a message
//! (`JoinLink::find`), and read as a pasted join code
//! (`JoinCode::from_pasted`). Nothing may panic. A link that reads writes
//! back as a link that reads again the same, and a link found inside the
//! text reads the same on its own.

#![no_main]

use libfuzzer_sys::fuzz_target;
use textweaver_mqtt::{JoinCode, JoinLink};

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    if let Ok(link) = JoinLink::parse(&text) {
        let again = JoinLink::parse(&link.to_string()).expect("a link we wrote reads again");
        assert!(again == link, "a join link changed when written and read again");
    }
    if let Some(Ok(link)) = JoinLink::find(&text) {
        let again = JoinLink::parse(&link.to_string()).expect("a found link writes back");
        assert!(again == link, "a found join link changed when written and read again");
    }
    if let Ok(code) = JoinCode::from_pasted(&text) {
        // A code that reads from a paste reads again as typed, grouped.
        let again = JoinCode::parse(&code.grouped()).expect("a grouped code reads again");
        assert!(again == code, "a join code changed when grouped and read again");
        let _ = code.pin();
    }
});

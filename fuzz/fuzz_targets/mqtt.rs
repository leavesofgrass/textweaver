//! Fuzz target: what an MQTT broker sends to textweaver's live sources.
//! The bytes are read as a stream of MQTT 3.1.1 packets by the client's
//! own reader, with a small size limit so the path that skips an oversize
//! PUBLISH runs too, and decoded as a message payload under small caps
//! (JSON depth and fields, string length) with every string cleaned.
//! Nothing may panic.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    textweaver_mqtt::fuzz_decode(data);
});

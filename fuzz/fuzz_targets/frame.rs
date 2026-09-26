//! Fuzz target: the engine-host frame decoder (ADR-0012). The bytes are
//! read as a stream of length-prefixed frames, the way the app reads a
//! host's output and a host reads the app's requests; every frame body is
//! decoded as each engine's requests and replies, and the ready header is
//! peeked. A bad frame is an error, never a panic; a frame that decodes
//! must encode to a frame that decodes to the same message.

#![no_main]

use std::io::Cursor;

use libfuzzer_sys::fuzz_target;
use textweaver_enginehost::protocol::{
    FrameReader, Message, ReadFrame, ReadyHeader, read_body, read_body_or_skip,
};

fn round_trip<M: Message + PartialEq + std::fmt::Debug>(body: &[u8]) {
    if let Ok(m) = M::decode(body) {
        // `encode` writes a whole frame, length prefix included; `decode`
        // takes the body.
        let framed = m.encode();
        let again = read_body(&mut Cursor::new(&framed))
            .expect("an encoded frame reads")
            .expect("an encoded frame is not empty");
        let again = M::decode(&again).expect("an encoded message decodes");
        assert_eq!(again, m);
    }
}

fn body(b: &[u8]) {
    let _ = ReadyHeader::peek(b);
    if let Ok((_tag, mut r)) = FrameReader::open(b) {
        let _ = r.u32();
        let _ = r.str();
        let _ = r.samples();
    }
    round_trip::<textweaver_eci::protocol::Request>(b);
    round_trip::<textweaver_eci::protocol::Reply>(b);
    round_trip::<textweaver_dectalk::protocol::Request>(b);
    round_trip::<textweaver_dectalk::protocol::Reply>(b);
    round_trip::<textweaver_sapi::protocol::Request>(b);
    round_trip::<textweaver_sapi::protocol::Reply>(b);
}

fuzz_target!(|data: &[u8]| {
    // The whole input as one body, then as a stream of frames.
    body(data);
    let mut stream = Cursor::new(data);
    for _ in 0..64 {
        match read_body_or_skip(&mut stream) {
            Ok(Some(ReadFrame::Body(b))) => body(&b),
            Ok(Some(ReadFrame::Skipped(_))) => {}
            Ok(None) | Err(_) => break,
        }
    }
});

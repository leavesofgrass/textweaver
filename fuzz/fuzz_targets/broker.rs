//! Fuzz target: what a client sends to textweaver's own broker. The bytes
//! are served as one connection, with small caps and a test logger account
//! and join password, through the login, subscriptions, publishing at each
//! QoS, retained messages and the size cap. Nothing may panic.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    textweaver_broker::fuzz_connection(data);
});

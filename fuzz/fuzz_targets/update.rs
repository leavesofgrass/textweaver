//! Fuzz target: what the update check reads from the network. The bytes
//! are read as GitHub's release list (`parse_releases`), and as a checksum
//! file searched for each listed asset's name and for a fixed name
//! (`sum_for`). Nothing may panic; a checksum that is found is 64
//! hexadecimal digits.

#![no_main]

use libfuzzer_sys::fuzz_target;
use textweaver_components::update::{parse_releases, sum_for};

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let mut names = vec!["textweaver-0.1.0-windows-x86_64.zip".to_owned()];
    if let Ok(releases) = parse_releases(data) {
        for r in releases.iter().take(8) {
            assert!(!r.version.starts_with('v'), "the tag's v is dropped");
            names.extend(r.assets.iter().take(8).map(|a| a.name.clone()));
        }
    }
    for name in &names {
        if let Some(sum) = sum_for(&text, name) {
            assert_eq!(sum.len(), 64);
            assert!(sum.bytes().all(|b| b.is_ascii_hexdigit()));
        }
    }
});

//! Fuzz target: the state JSON. The bytes are read as a document's saved
//! state (position, history, bookmarks) and as a folder's sidecar
//! (`.textweaver/progress.json`), which is then merged with itself and with
//! an empty map under every policy. Nothing may panic, and state that reads
//! must write back and read again.

#![no_main]

use libfuzzer_sys::fuzz_target;
use textweaver_core::CharPos;
use textweaver_store::sync::{Prefer, SidecarMap, merge_maps};
use textweaver_store::{ConflictPolicy, DocState};

fuzz_target!(|data: &[u8]| {
    if let Ok(mut state) = serde_json::from_slice::<DocState>(data) {
        let json = serde_json::to_string(&state).expect("state serializes");
        let _: DocState = serde_json::from_str(&json).expect("state reads back");
        let len = 1000;
        let _ = state.sorted_bookmarks();
        let _ = state.next_bookmark(CharPos(500), true);
        let _ = state.next_bookmark_name();
        state.set_position(CharPos(2000), len);
        state.push_history(CharPos(10), 50);
        let _ = state.add_bookmark(None, CharPos(20), len);
    }
    if let Ok(value) = serde_json::from_slice::<serde_json::Value>(data) {
        let map = SidecarMap::from_value(&value);
        let empty = SidecarMap::new();
        for policy in [
            ConflictPolicy::Newest,
            ConflictPolicy::HighestProgress,
            ConflictPolicy::default(),
        ] {
            for prefer in [Prefer::Local, Prefer::Remote] {
                let _ = merge_maps(&map, &map, policy, prefer);
                let _ = merge_maps(&map, &empty, policy, prefer);
                let _ = merge_maps(&empty, &map, policy, prefer);
            }
        }
        let _ = map.to_value();
    }
});

# ECI feasibility spike

A dependency-free 32-bit Rust program that loads Code Factory's `eci.dll` (ETI-Eloquence 6.1), inserts an index mark before every word, synthesizes into its own buffer, and prints the audio time of each mark. It is a reference for `crates/textweaver-eci`, not part of the workspace.

```bash
cargo +1.96 build --release --target i686-pc-windows-msvc --manifest-path tools/eci-spike/Cargo.toml
```

```bash
tools/eci-spike/target/i686-pc-windows-msvc/release/eci-spike.exe eci.wav
```

Result on 2026-09-25 (Windows 11, Eloquence for Windows 6.1): 4,710 ms of audio at 11,025 Hz synthesized in 8 ms; every index mark reported at its word's sample offset (for example "Smith" at 325 ms, "library" at 1,051 ms); `eciWantWordIndex` produced no replies, so explicit `eciInsertIndex` marks are the mechanism. Through SAPI5, the same engine gave one word event per sentence and no bookmark events.

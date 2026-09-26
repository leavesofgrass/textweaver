//! `textweaver-sapi-host`: runs SAPI5 voices in their own process (ADR-0009).
//! Owner: Agent G. Phase 0: reports availability only.

fn main() {
    if textweaver_sapi::available() {
        println!("SAPI5 host (placeholder)");
    } else {
        eprintln!("SAPI5 is only available on Windows");
        std::process::exit(1);
    }
}

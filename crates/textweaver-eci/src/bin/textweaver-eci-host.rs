//! `textweaver-eci-host`: runs the ECI engine in its own process (ADR-0007).
//! Owner: Agent E. Phase 0: reports whether the library is present.

fn main() {
    match textweaver_eci::default_library_path() {
        Some(p) => println!("ECI library found at {}", p.display()),
        None => {
            eprintln!("no ECI library found");
            std::process::exit(1);
        }
    }
}

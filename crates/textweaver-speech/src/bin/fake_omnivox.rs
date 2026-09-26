//! A stand-in for the Omnivox speech server, for tests: reads protocol lines
//! from stdin and appends each one to the file named by the first argument,
//! flushing after every line. Exits when stdin closes.

use std::fs::OpenOptions;
use std::io::{BufRead, Write};

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: tw-fake-omnivox LOG_FILE");
        std::process::exit(2);
    };
    let mut log = match OpenOptions::new().create(true).append(true).open(&path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("cannot open {path}: {e}");
            std::process::exit(1);
        }
    };
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        if writeln!(log, "{line}").and_then(|()| log.flush()).is_err() {
            break;
        }
    }
}

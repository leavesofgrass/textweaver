//! A stand-in for `textweaver` in the launcher's tests: prints its
//! arguments on one line, separated by `|`, and exits with the status
//! named by an `exit=N` argument (0 without one).

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    println!("{}", args.join("|"));
    let code = args
        .iter()
        .find_map(|a| a.strip_prefix("exit="))
        .and_then(|n| n.parse().ok())
        .unwrap_or(0);
    std::process::exit(code);
}

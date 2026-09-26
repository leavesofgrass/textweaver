//! Bulk-conversion benchmark: generates a corpus of Markdown files and
//! converts it, reporting files per second and throughput; `--profile`
//! also times each stage of the hot path on one thread.
//!
//! ```text
//! cargo run --release -p textweaver-convert --example bench_convert -- \
//!     [--files 1000] [--to html] [--engine pulldown|comrak] [--flavor gfm]
//!     [--jobs N] [--runs 5] [--dir PATH] [--profile]
//! ```
//!
//! Peak memory: on Linux the process's peak resident set (`VmHWM`) is
//! printed at the end; on Windows measure it from outside (the report in
//! ADR-0016 shows how).

use std::path::{Path, PathBuf};
use std::time::Instant;

use textweaver_convert::{ConvertOptions, Converter, OutputFormat, write_atomic};
use textweaver_render::{Engine, Flavor, PageOptions, RenderOptions, Templates, render};

struct Args {
    files: usize,
    to: OutputFormat,
    engine: Engine,
    flavor: Flavor,
    jobs: Option<usize>,
    runs: usize,
    dir: Option<PathBuf>,
    profile: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut a = Args {
        files: 1000,
        to: OutputFormat::Html,
        engine: Engine::PulldownCmark,
        flavor: Flavor::Gfm,
        jobs: None,
        runs: 5,
        dir: None,
        profile: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut value = || it.next().ok_or_else(|| format!("{flag} needs a value"));
        match flag.as_str() {
            "--files" => a.files = value()?.parse().map_err(|e| format!("--files: {e}"))?,
            "--to" => a.to = OutputFormat::parse(&value()?).ok_or("unknown --to")?,
            "--engine" => a.engine = Engine::parse(&value()?).ok_or("unknown --engine")?,
            "--flavor" => a.flavor = Flavor::parse(&value()?).ok_or("unknown --flavor")?,
            "--jobs" => a.jobs = Some(value()?.parse().map_err(|e| format!("--jobs: {e}"))?),
            "--runs" => a.runs = value()?.parse().map_err(|e| format!("--runs: {e}"))?,
            "--dir" => a.dir = Some(PathBuf::from(value()?)),
            "--profile" => a.profile = true,
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(a)
}

/// A small deterministic generator (xorshift), so every run sees the
/// same corpus.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

const WORDS: &[&str] = &[
    "the",
    "reader",
    "speaks",
    "every",
    "sentence",
    "clearly",
    "while",
    "students",
    "follow",
    "along",
    "with",
    "highlighted",
    "words",
    "accessible",
    "documents",
    "matter",
    "because",
    "print",
    "disabilities",
    "are",
    "common",
    "chapter",
    "section",
    "table",
    "figure",
    "equation",
    "notes",
    "library",
    "voice",
    "rate",
    "pitch",
    "braille",
    "structure",
    "heading",
    "paragraph",
    "list",
    "link",
    "reference",
    "summary",
    "example",
];

fn sentence(r: &mut Rng) -> String {
    let n = 8 + r.below(14);
    let mut s = String::new();
    for i in 0..n {
        let w = WORDS[r.below(WORDS.len())];
        if i == 0 {
            let mut c = w.chars();
            if let Some(f) = c.next() {
                s.extend(f.to_uppercase());
                s.push_str(c.as_str());
            }
        } else {
            s.push(' ');
            match r.below(20) {
                0 => {
                    s.push_str("**");
                    s.push_str(w);
                    s.push_str("**");
                }
                1 => {
                    s.push('*');
                    s.push_str(w);
                    s.push('*');
                }
                2 => {
                    s.push('`');
                    s.push_str(w);
                    s.push('`');
                }
                3 => s.push_str(&format!("[{w}](https://example.org/{w})")),
                _ => s.push_str(w),
            }
        }
    }
    s.push('.');
    s
}

/// One document of roughly 6 to 20 KB with the usual structures.
fn document(r: &mut Rng, n: usize) -> String {
    let mut d = format!("---\ntitle: Document {n}\nauthor: Bench\n---\n\n# Document {n}\n\n");
    let sections = 3 + r.below(5);
    for s in 0..sections {
        d.push_str(&format!("## Section {s}\n\n"));
        for _ in 0..(2 + r.below(4)) {
            for _ in 0..(3 + r.below(4)) {
                d.push_str(&sentence(r));
                d.push(' ');
            }
            d.push_str("\n\n");
        }
        match r.below(5) {
            0 => {
                d.push_str("| Term | Count | Note |\n|:---|---:|---|\n");
                for i in 0..(3 + r.below(6)) {
                    d.push_str(&format!(
                        "| {} | {} | {} |\n",
                        WORDS[r.below(WORDS.len())],
                        i * 7,
                        WORDS[r.below(WORDS.len())]
                    ));
                }
                d.push('\n');
            }
            1 => {
                d.push_str("```rust\nfn main() {\n    println!(\"hello\");\n}\n```\n\n");
            }
            2 => {
                d.push_str("The area is $\\pi r^2$ and\n\n$$\n\\sum_{i=1}^{n} i = \\frac{n(n+1)}{2}\n$$\n\n");
            }
            3 => {
                for i in 0..(3 + r.below(5)) {
                    d.push_str(&format!(
                        "- [{}] {}\n",
                        if i % 2 == 0 { "x" } else { " " },
                        sentence(r)
                    ));
                }
                d.push('\n');
            }
            _ => {
                d.push_str(&format!(
                    "> {}\n\nA note.[^n{s}]\n\n[^n{s}]: {}\n\n",
                    sentence(r),
                    sentence(r)
                ));
            }
        }
    }
    d
}

fn generate(dir: &Path, files: usize) -> std::io::Result<u64> {
    let mut r = Rng(0x9E37_79B9_7F4A_7C15);
    let mut bytes = 0;
    for n in 0..files {
        let sub = dir.join(format!("part{:02}", n / 100));
        std::fs::create_dir_all(&sub)?;
        let text = document(&mut r, n);
        bytes += text.len() as u64;
        std::fs::write(sub.join(format!("doc{n:04}.md")), text)?;
    }
    Ok(bytes)
}

fn peak_rss() -> Option<String> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    status
        .lines()
        .find(|l| l.starts_with("VmHWM:"))
        .map(|l| l.trim_start_matches("VmHWM:").trim().to_owned())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args()?;
    let base = args.dir.clone().unwrap_or_else(|| {
        std::env::temp_dir().join(format!("textweaver-bench-{}", std::process::id()))
    });
    let input = base.join("corpus");
    let output = base.join("out");
    let _ = std::fs::remove_dir_all(&base);
    let t = Instant::now();
    let corpus_bytes = generate(&input, args.files)?;
    println!(
        "Corpus: {} Markdown files, {:.1} MB, generated in {:.2} s",
        args.files,
        corpus_bytes as f64 / 1e6,
        t.elapsed().as_secs_f64()
    );

    let options = ConvertOptions {
        to: args.to,
        out_dir: Some(output.clone()),
        force: true,
        jobs: args.jobs,
        render: RenderOptions {
            engine: args.engine,
            flavor: args.flavor,
            ..RenderOptions::default()
        },
        ..ConvertOptions::default()
    };
    let conv = Converter::new(options)?;
    let mut rates = Vec::new();
    let mut threads = 0;
    for run in 0..args.runs {
        let s = conv.run(std::slice::from_ref(&input))?;
        threads = s.threads;
        if s.failed > 0 {
            return Err(format!("{} files failed", s.failed).into());
        }
        let fps = s.files_per_second();
        println!(
            "Run {}: {} files in {:.3} s = {:.0} files/s, {:.1} MB/s in, {:.1} MB/s out",
            run + 1,
            s.converted,
            s.seconds,
            fps,
            s.bytes_in as f64 / 1e6 / s.seconds,
            s.bytes_out as f64 / 1e6 / s.seconds
        );
        rates.push(fps);
    }
    rates.sort_by(f64::total_cmp);
    if let (Some(best), Some(median)) = (rates.last(), rates.get(rates.len() / 2)) {
        println!(
            "Result: {} to {} with {} on {threads} threads: median {:.0} files/s, best {:.0} files/s",
            args.files,
            args.to.label(),
            args.engine.name(),
            median,
            best
        );
    }

    // Skip-unchanged: a run where every output is newer than its source.
    let incremental = Converter::new(ConvertOptions {
        force: false,
        ..conv.options().clone()
    })?;
    let s = incremental.run(std::slice::from_ref(&input))?;
    println!("Unchanged run: {} skipped in {:.3} s", s.skipped, s.seconds);

    if args.profile {
        profile(&input, &output, &args)?;
    }
    if let Some(rss) = peak_rss() {
        println!("Peak resident memory (VmHWM): {rss}");
    }
    let _ = std::fs::remove_dir_all(&base);
    Ok(())
}

/// Times each stage of the HTML hot path on one thread.
fn profile(input: &Path, output: &Path, args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let mut paths: Vec<PathBuf> = Vec::new();
    let mut stack = vec![input.to_owned()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d)? {
            let p = e?.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                paths.push(p);
            }
        }
    }
    paths.sort();
    let t = Instant::now();
    let texts: Vec<String> = paths
        .iter()
        .map(std::fs::read_to_string)
        .collect::<Result<_, _>>()?;
    let read = t.elapsed().as_secs_f64();

    let bare = RenderOptions {
        engine: args.engine,
        ..RenderOptions::commonmark()
    };
    let t = Instant::now();
    let mut n = 0;
    for text in &texts {
        n += render(text, &bare).html.len();
    }
    let parse_write = t.elapsed().as_secs_f64();

    let full = RenderOptions {
        engine: args.engine,
        flavor: args.flavor,
        ..RenderOptions::default()
    };
    let t = Instant::now();
    let rendered: Vec<_> = texts.iter().map(|text| render(text, &full)).collect();
    let pipeline = t.elapsed().as_secs_f64();

    // The same pass without math, and without math or heading ids, to
    // attribute the pass's cost.
    let no_math = RenderOptions {
        math: false,
        ..full.clone()
    };
    let t = Instant::now();
    for text in &texts {
        n += render(text, &no_math).html.len();
    }
    let without_math = t.elapsed().as_secs_f64();
    let no_ids = RenderOptions {
        heading_ids: false,
        ..no_math.clone()
    };
    let t = Instant::now();
    for text in &texts {
        n += render(text, &no_ids).html.len();
    }
    let without_ids = t.elapsed().as_secs_f64();

    let templates = Templates::builtin();
    let t = Instant::now();
    let pages: Vec<String> = rendered
        .iter()
        .map(|r| templates.render("default", r, &PageOptions::default()))
        .collect::<Result<_, _>>()?;
    let template = t.elapsed().as_secs_f64();

    let dir = output.join("profile");
    let t = Instant::now();
    for (i, p) in pages.iter().enumerate() {
        write_atomic(&dir.join(format!("{i}.html")), p.as_bytes())?;
    }
    let write = t.elapsed().as_secs_f64();
    let plain_dir = output.join("profile-plain");
    std::fs::create_dir_all(&plain_dir)?;
    let t = Instant::now();
    for (i, p) in pages.iter().enumerate() {
        std::fs::write(plain_dir.join(format!("{i}.html")), p.as_bytes())?;
    }
    let plain = t.elapsed().as_secs_f64();

    let total = read + pipeline + template + write;
    let pct = |x: f64| 100.0 * x / total;
    println!(
        "Profile, one thread, {} files ({n} bytes of bare HTML):",
        texts.len()
    );
    println!(
        "  read files            {:>8.1} ms  {:>5.1}%",
        read * 1e3,
        pct(read)
    );
    println!(
        "  parse + write only    {:>8.1} ms  (CommonMark, no extension pass; for comparison)",
        parse_write * 1e3
    );
    println!(
        "  render ({:>8})     {:>8.1} ms  {:>5.1}%",
        args.flavor.name(),
        pipeline * 1e3,
        pct(pipeline)
    );
    println!(
        "    of which math        {:>8.1} ms  (render without math: {:.1} ms)",
        (pipeline - without_math) * 1e3,
        without_math * 1e3
    );
    println!(
        "    of which heading ids {:>8.1} ms  (without math or ids: {:.1} ms)",
        (without_math - without_ids) * 1e3,
        without_ids * 1e3
    );
    println!(
        "  template              {:>8.1} ms  {:>5.1}%",
        template * 1e3,
        pct(template)
    );
    println!(
        "  write (atomic)        {:>8.1} ms  {:>5.1}%",
        write * 1e3,
        pct(write)
    );
    println!("  write (plain, for comparison) {:>8.1} ms", plain * 1e3);
    println!(
        "  per file              {:>8.3} ms",
        total * 1e3 / texts.len().max(1) as f64
    );
    Ok(())
}

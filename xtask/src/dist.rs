//! `cargo xtask dist [--universal] [--out DIR]`: a release package for
//! this platform (docs/releasing.md).
//!
//! Builds `textweaver` and `tw` in release, the engine hosts for the
//! platform (Windows and Linux), and stages them with the pronunciation
//! dictionaries, the licence, and the user guides in
//! `target/dist/textweaver-VERSION-PLATFORM/`, then archives the folder:
//! a `.zip` on Windows, a `.tar.gz` elsewhere.
//!
//! - Windows builds link the C runtime statically (`+crt-static`), so the
//!   package runs without the Visual C++ redistributable. The static build
//!   uses its own target directory (`target/dist-build`) so it does not
//!   invalidate everyday builds.
//! - `--universal` (macOS) builds for Apple silicon and Intel, joins the
//!   binaries with `lipo`, and signs the result ad hoc (`codesign -s -`),
//!   which Apple silicon requires. The package is not notarized.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, bail};

use crate::eci::{self, HostBuild};

/// The user binaries: (package, binary).
const BINARIES: [(&str, &str); 2] = [("textweaver-tui", "textweaver"), ("textweaver-cli", "tw")];
/// Features for the user binaries (subprocess backends only, so nothing
/// extra is linked).
const FEATURES: &str = "textweaver-tui/omnivox,textweaver-cli/omnivox";
/// The two macOS targets joined by `--universal`.
const MAC_TARGETS: [&str; 2] = ["aarch64-apple-darwin", "x86_64-apple-darwin"];
/// Documents copied into the package: (source, name in the package).
const DOCS: [(&str, &str); 5] = [
    ("README.md", "README.md"),
    ("LICENSE", "LICENSE"),
    ("CHANGELOG.md", "CHANGELOG.md"),
    ("docs/install.md", "INSTALL.md"),
    ("docs/eloquence.md", "docs/eloquence.md"),
];
/// More guides copied when present.
const OPTIONAL_DOCS: [&str; 3] = ["docs/keyboard.md", "docs/docker.md", "docs/themes.md"];

/// Parsed arguments.
#[derive(Debug, Default, PartialEq, Eq)]
struct Args {
    universal: bool,
    out: Option<PathBuf>,
}

fn parse(args: &[String]) -> anyhow::Result<Args> {
    let mut out = Args::default();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--universal" => out.universal = true,
            "--out" => out.out = Some(PathBuf::from(it.next().context("--out needs a directory")?)),
            other => bail!(
                "unknown argument {other} (usage: cargo xtask dist [--universal] [--out DIR])"
            ),
        }
    }
    if out.universal && !cfg!(target_os = "macos") {
        bail!("--universal is for macOS");
    }
    Ok(out)
}

/// The platform part of the package name.
fn platform(universal: bool) -> String {
    let os = std::env::consts::OS;
    let arch = if universal {
        "universal"
    } else {
        std::env::consts::ARCH
    };
    format!("{os}-{arch}")
}

/// The package folder name.
fn package_name(version: &str, platform: &str) -> String {
    format!("textweaver-{version}-{platform}")
}

/// `cargo xtask dist`.
pub fn run() -> anyhow::Result<()> {
    let args = parse(&std::env::args().skip(2).collect::<Vec<_>>())?;
    let root = eci::root();
    let version = env!("CARGO_PKG_VERSION");
    let build_dir = if cfg!(windows) {
        eci::target_dir(&root).join("dist-build")
    } else {
        eci::target_dir(&root)
    };
    let out = args
        .out
        .clone()
        .unwrap_or_else(|| eci::target_dir(&root).join("dist"));
    let name = package_name(version, &platform(args.universal));
    let stage = out.join(&name);
    if stage.exists() {
        fs::remove_dir_all(&stage).with_context(|| format!("clearing {}", stage.display()))?;
    }
    fs::create_dir_all(&stage)?;

    if args.universal {
        for t in MAC_TARGETS {
            build_binaries(&root, &build_dir, Some(t))?;
        }
        for (_, bin) in BINARIES {
            let parts: Vec<PathBuf> = MAC_TARGETS
                .iter()
                .map(|t| build_dir.join(t).join("release").join(bin))
                .collect();
            let dest = stage.join(bin);
            run_tool(
                Command::new("lipo")
                    .arg("-create")
                    .args(&parts)
                    .arg("-output")
                    .arg(&dest),
            )?;
            run_tool(
                Command::new("codesign")
                    .args(["--force", "--sign", "-"])
                    .arg(&dest),
            )?;
            println!("installed {} (universal)", dest.display());
        }
    } else {
        build_binaries(&root, &build_dir, None)?;
        for (_, bin) in BINARIES {
            let file = format!("{bin}{}", std::env::consts::EXE_SUFFIX);
            eci::copy(&build_dir.join("release").join(&file), &stage.join(&file))?;
        }
    }

    // Engine hosts: Eloquence (ECI) and SAPI5 on Windows, ECI for Voxin on
    // Linux. macOS speaks through Apple's own voices in process.
    if !cfg!(target_os = "macos") {
        let hosts = eci::all_hosts();
        build_hosts(&root, &build_dir, &hosts)?;
        for h in &hosts {
            eci::copy(
                &h.built(&build_dir, "release"),
                &stage.join(h.installed_name()),
            )?;
        }
        eci::copy_dictionaries(&root, &stage)?;
    }

    for (src, dest) in DOCS {
        eci::copy(&root.join(src), &stage.join(dest))?;
    }
    for src in OPTIONAL_DOCS {
        let path = root.join(src);
        if path.is_file() {
            eci::copy(&path, &stage.join(src))?;
        }
    }

    let archive = if cfg!(windows) {
        let zip = out.join(format!("{name}.zip"));
        zip_dir(&stage, &name, &zip)?;
        zip
    } else {
        let tgz = out.join(format!("{name}.tar.gz"));
        run_tool(
            Command::new("tar")
                .arg("-czf")
                .arg(&tgz)
                .arg("-C")
                .arg(&out)
                .arg(&name),
        )?;
        tgz
    };
    println!("package {}", archive.display());
    Ok(())
}

/// A cargo command for release builds into `build_dir`, with the static C
/// runtime on Windows.
fn cargo(root: &Path, build_dir: &Path) -> Command {
    let mut cmd = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    cmd.current_dir(root).env("CARGO_TARGET_DIR", build_dir);
    if cfg!(windows) {
        let mut flags = std::env::var("RUSTFLAGS").unwrap_or_default();
        if !flags.contains("crt-static") {
            flags.push_str(" -C target-feature=+crt-static");
        }
        cmd.env("RUSTFLAGS", flags.trim());
    }
    cmd
}

fn build_binaries(root: &Path, build_dir: &Path, target: Option<&str>) -> anyhow::Result<()> {
    let mut cmd = cargo(root, build_dir);
    cmd.args(["build", "--release", "--features", FEATURES]);
    for (package, bin) in BINARIES {
        cmd.args(["-p", package, "--bin", bin]);
    }
    if let Some(t) = target {
        cmd.args(["--target", t]);
    }
    run_tool(&mut cmd).context("building textweaver and tw")
}

fn build_hosts(root: &Path, build_dir: &Path, hosts: &[HostBuild]) -> anyhow::Result<()> {
    let mut targets: Vec<Option<&str>> = Vec::new();
    for h in hosts {
        if !targets.contains(&h.target) {
            targets.push(h.target);
        }
    }
    for target in targets {
        let mut cmd = cargo(root, build_dir);
        cmd.args(["build", "--release", "--no-default-features"]);
        for h in hosts.iter().filter(|h| h.target == target) {
            cmd.args(["-p", h.package, "--bin", h.bin]);
        }
        if let Some(t) = target {
            cmd.args(["--target", t]);
        }
        run_tool(&mut cmd).with_context(|| match target {
            Some(t) => format!("building the engine hosts for {t} (rustup target add {t})"),
            None => "building the engine hosts".to_owned(),
        })?;
    }
    Ok(())
}

fn run_tool(cmd: &mut Command) -> anyhow::Result<()> {
    let status = cmd
        .status()
        .with_context(|| format!("running {:?}", cmd.get_program()))?;
    if !status.success() {
        bail!("{:?} failed ({status})", cmd.get_program());
    }
    Ok(())
}

/// Zips `dir` so its files sit under `prefix/` in the archive.
fn zip_dir(dir: &Path, prefix: &str, zip_path: &Path) -> anyhow::Result<()> {
    use std::io::Write as _;
    use zip::write::SimpleFileOptions;

    let file =
        fs::File::create(zip_path).with_context(|| format!("creating {}", zip_path.display()))?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut files = Vec::new();
    collect(dir, &mut files)?;
    files.sort();
    for path in files {
        let rel = path.strip_prefix(dir)?;
        let name = format!("{prefix}/{}", rel.to_string_lossy().replace('\\', "/"));
        zip.start_file(name, opts)?;
        zip.write_all(&fs::read(&path)?)?;
    }
    zip.finish()?;
    Ok(())
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect(&path, out)?;
        } else {
            out.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(
            package_name("0.1.0-alpha.3", "windows-x86_64"),
            "textweaver-0.1.0-alpha.3-windows-x86_64"
        );
        assert!(platform(false).starts_with(std::env::consts::OS));
        assert!(platform(true).ends_with("-universal"));
    }

    #[test]
    fn arguments() {
        let a = parse(&["--out".into(), "x".into()]).unwrap();
        assert_eq!(a.out, Some(PathBuf::from("x")));
        assert!(!a.universal);
        assert!(parse(&["--bogus".into()]).is_err());
        assert_eq!(
            parse(&["--universal".into()]).is_ok(),
            cfg!(target_os = "macos")
        );
    }

    #[test]
    fn packaged_documents_exist() {
        let root = eci::root();
        for (src, _) in DOCS {
            assert!(root.join(src).is_file(), "{src} is packaged but missing");
        }
    }
}

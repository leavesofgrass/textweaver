//! `tw update`: textweaver updates itself from the command line, headless,
//! for scripts and install scripts (B1-u1).
//!
//! - `tw update --check` reads the public list of textweaver releases on
//!   GitHub and says whether a newer one exists, with its version and size.
//! - `tw update` does the same, then downloads the package for this
//!   computer, checks it against the release's `SHA256SUMS.txt`, and
//!   installs it: at once on Linux and macOS; on Windows when `tw` closes,
//!   through the new package's `tw update --finish`. Running it is the yes;
//!   it never asks. Settings, notes, the library, state, and components are
//!   never touched.
//!
//! Only the public release list is read, with the neutral User-Agent;
//! nothing about the person or the computer is sent. The app's Help, Check
//! for updates does the same in the window and the terminal reader.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use anyhow::bail;
use textweaver_app::components::{Progress, StandardFetcher, can_download, size_text};
use textweaver_app::updates::CURRENT;
use textweaver_app::updates::swap::{self, Installed, Place};
use textweaver_app::updates::update::{self, Target, UpdateError};

/// Arguments for `tw update`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Only say whether a newer release exists, with its version and size.
    #[arg(long)]
    check: bool,
    /// Use the files under this folder instead of the usual place.
    #[arg(long, value_name = "DIR")]
    home: Option<PathBuf>,
    /// Finish an update on Windows: the unpacked package to swap in (used
    /// by textweaver itself).
    #[arg(long, value_name = "DIR", hide = true)]
    finish: Option<PathBuf>,
    /// The install folder the update goes into (with --finish).
    #[arg(long, value_name = "DIR", hide = true, requires = "finish")]
    into: Option<PathBuf>,
    /// The program to wait for before swapping (with --finish).
    #[arg(long, value_name = "FILE", hide = true, requires = "finish")]
    wait_for: Option<PathBuf>,
    /// The program to start again afterwards (with --finish).
    #[arg(long, value_name = "FILE", hide = true, requires = "finish")]
    start: Option<PathBuf>,
}

/// How long `--finish` waits for textweaver to close.
const CLOSE_WAIT: Duration = Duration::from_secs(120);

/// How often download progress is said, at most.
const PROGRESS_EVERY: Duration = Duration::from_secs(10);

/// Runs `tw update`.
pub fn run(args: Args) -> anyhow::Result<()> {
    if let Some(new_root) = &args.finish {
        let (Some(into), Some(wait_for)) = (&args.into, &args.wait_for) else {
            bail!("--finish needs --into and --wait-for");
        };
        return finish(new_root, into, wait_for, args.start.as_deref());
    }
    if !can_download() {
        bail!("Updates are not in this version.");
    }
    let Some(target) = Target::this() else {
        bail!("No update packages for this system.");
    };
    eprintln!("Checking for updates.");
    let offer = update::check(&StandardFetcher, update::RELEASES, CURRENT, &target)
        .map_err(|e| anyhow::anyhow!("Update check failed: {e}"))?;
    let Some(offer) = offer else {
        crate::cmd::outln!("No update: {CURRENT} is the newest.");
        return Ok(());
    };
    crate::cmd::outln!(
        "Update available: textweaver {}, {}.",
        offer.version,
        offer.size_text()
    );
    if args.check {
        return Ok(());
    }
    let place = Place::find(&target).map_err(|e| anyhow::anyhow!(not_installed(&e)))?;
    let paths = super::paths(args.home.as_deref())?;
    let dir = paths.cache_dir.join("updates").join(&offer.version);
    eprintln!("Downloading the update, {}.", offer.size_text());
    let (mut said_at, total) = (Instant::now(), offer.package.size);
    let package = update::download_update(
        &offer,
        &StandardFetcher,
        &dir,
        &mut |p: Progress| {
            if p.done < total && said_at.elapsed() >= PROGRESS_EVERY {
                eprintln!("Update {} percent downloaded.", p.percent());
                said_at = Instant::now();
            }
        },
        &AtomicBool::new(false),
    )
    .map_err(|e| anyhow::anyhow!(not_installed(&e)))?;
    eprintln!(
        "Update verified: {} matches its checksum.",
        size_text(total)
    );
    match swap::install(&package, &target, &place, &dir) {
        Ok(Installed::Done) => {
            crate::cmd::outln!("Update installed. Restart textweaver to use it.");
            Ok(())
        }
        Ok(Installed::OnClose { new_root, install }) => {
            let exe = std::env::current_exe()?;
            swap::start_finisher(&new_root, &install, &exe, None)
                .map_err(|e| anyhow::anyhow!(not_installed(&e)))?;
            crate::cmd::outln!("Update verified. It installs when tw closes.");
            Ok(())
        }
        Err(e) => bail!(not_installed(&e)),
    }
}

/// One line for a failed update, meaning first.
fn not_installed(e: &UpdateError) -> String {
    match e {
        UpdateError::Mismatch(name) => {
            format!("Update refused: {name} does not match its checksum. Nothing changed.")
        }
        UpdateError::NotPackage(dir) => format!(
            "Not updated: {} is not a release package; update textweaver the way it was installed.",
            dir.display()
        ),
        other => format!("Update not installed: {other}. Nothing changed."),
    }
}

/// `tw update --finish`: waits for `wait_for` to close, swaps the unpacked
/// package into `into`, and starts `start` again when given.
fn finish(
    new_root: &Path,
    into: &Path,
    wait_for: &Path,
    start: Option<&Path>,
) -> anyhow::Result<()> {
    if !swap::wait_until_closed(wait_for, CLOSE_WAIT) {
        bail!(
            "Not updated: {} did not close. Nothing changed.",
            wait_for.display()
        );
    }
    let n = swap::swap_in(new_root, into, false).map_err(|e| anyhow::anyhow!(not_installed(&e)))?;
    crate::cmd::outln!("Update installed: {n} entries replaced.");
    if let Some(program) = start {
        textweaver_app::core::process::command(program).spawn()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Windows swap through `tw update --finish`, with a fake install
    /// folder: the package's files are replaced, the rest kept.
    #[test]
    fn finish_swaps_the_package_into_a_fake_install() {
        let tmp = tempfile::tempdir().unwrap();
        let install = tmp.path().join("install");
        std::fs::create_dir_all(&install).unwrap();
        std::fs::write(install.join("NOTICE"), b"old").unwrap();
        std::fs::write(install.join("tw.exe"), b"old tw").unwrap();
        std::fs::write(install.join("install-manifest.txt"), b"kept").unwrap();
        let new = tmp.path().join("new");
        std::fs::create_dir_all(&new).unwrap();
        std::fs::write(new.join("NOTICE"), b"new").unwrap();
        std::fs::write(new.join("tw.exe"), b"new tw").unwrap();
        finish(&new, &install, &install.join("tw.exe"), None).unwrap();
        assert_eq!(std::fs::read(install.join("tw.exe")).unwrap(), b"new tw");
        assert_eq!(
            std::fs::read(install.join("install-manifest.txt")).unwrap(),
            b"kept"
        );
    }

    #[test]
    fn a_mismatch_says_nothing_changed() {
        let line = not_installed(&UpdateError::Mismatch("p.zip".into()));
        assert!(line.starts_with("Update refused: p.zip"), "{line}");
        assert!(line.ends_with("Nothing changed."));
    }
}

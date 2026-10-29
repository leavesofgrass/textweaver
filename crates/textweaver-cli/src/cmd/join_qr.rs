//! `tw join-qr IMAGE`: reads the join link out of a QR code in a PNG or
//! JPEG image (a photo or a screenshot of textweaver's join QR code) and
//! prints it, meaning first: `Join link: twjoin://...`. With no join link
//! in the image, one line says why and the exit status is 1.
//!
//! [`write_qr`] writes a join link's QR code for `tw broker --qr FILE` and
//! `tw share --qr FILE`: a PNG, or an SVG when the file name ends in
//! `.svg`. Only where the user names; textweaver keeps no copy.
//!
//! A join link holds the join code. It is printed only on request, to
//! standard output, and never in an error.

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use textweaver_mqtt::JoinLink;

use crate::cmd::{Plain, outln};

/// Arguments for `tw join-qr`.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// The image with the QR code: a PNG or JPEG file.
    pub image: PathBuf,
}

/// Runs `tw join-qr`.
pub fn run(args: Args) -> Result<()> {
    let link = read_link(&args.image)?;
    outln!("Join link: {link}");
    Ok(())
}

/// The join link in the QR code in the image at `path`, or why there is
/// none, in a sentence that never holds the code.
fn read_link(path: &Path) -> Result<JoinLink> {
    let texts = textweaver_qr::decode_file(path)
        .map_err(|e| Plain(format!("No join link: {}.", sentence(&e.to_string()))))?;
    let mut bad = None;
    for text in &texts {
        match JoinLink::find(text) {
            Some(Ok(link)) => return Ok(link),
            Some(Err(reason)) => bad = Some(reason),
            None => {}
        }
    }
    match bad {
        Some(reason) => bail!(Plain(format!(
            "No join link: {}",
            reason.strip_prefix("Join link error: ").unwrap_or(&reason)
        ))),
        None => bail!(Plain(
            "No join link: the QR code holds something else.".into()
        )),
    }
}

/// `text` without a final period, to end a sentence of its own.
fn sentence(text: &str) -> &str {
    text.trim_end_matches('.')
}

/// The accessible name of a join QR code: "QR code for joining HOST".
/// Never the link, which holds the code.
pub(crate) fn qr_title(link: &JoinLink) -> String {
    format!("QR code for joining {}", link.host())
}

/// Checks that `path` names a PNG or SVG file, before anything starts:
/// true for an SVG.
pub(crate) fn check_qr_path(path: &Path) -> Result<bool> {
    let is = |ext: &str| {
        path.extension()
            .is_some_and(|e| e.eq_ignore_ascii_case(ext))
    };
    if is("svg") {
        return Ok(true);
    }
    if is("png") {
        return Ok(false);
    }
    bail!(Plain(format!(
        "QR code not saved: {} does not end in .png or .svg.",
        path.display()
    )))
}

/// Writes `link`'s QR code to `path`: an SVG when its name ends in
/// `.svg`, else a PNG. Says where, meaning first.
pub(crate) fn write_qr(path: &Path, link: &JoinLink) -> Result<()> {
    let svg = check_qr_path(path)?;
    let matrix = textweaver_qr::encode(&link.to_string())
        .map_err(|e| Plain(format!("QR code not saved: {}.", sentence(&e.to_string()))))?;
    let bytes = if svg {
        matrix.to_svg(&qr_title(link)).into_bytes()
    } else {
        matrix
            .to_png(textweaver_qr::DEFAULT_MODULE_PIXELS)
            .map_err(|e| Plain(format!("QR code not saved: {}.", sentence(&e.to_string()))))?
    };
    std::fs::write(path, bytes).map_err(|e| {
        Plain(format!(
            "QR code not saved: cannot write {}: {e}.",
            path.display()
        ))
    })?;
    outln!("QR code saved: {}", path.display());
    Ok(())
}

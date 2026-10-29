//! Round trips through PNG, SVG and pixels, the decoder's harder cases, and
//! the errors, said in words without the text of a code.

use std::path::PathBuf;

use textweaver_qr::{
    GrayImage, QUIET_ZONE, QrError, decode_bytes, decode_file, decode_image, decode_rgba, encode,
};

/// An example join link: a made-up host and code, no real secret.
const LINK: &str = "twjoin://broker.example:8883/ABCD-EFGH-JKMN-PQRS?topic=textweaver/live/demo";

/// The code for `text` as a greyscale image, `scale` pixels to a module,
/// with its quiet zone.
fn image_of(text: &str, scale: u32) -> GrayImage {
    let png = encode(text).unwrap().to_png(scale).unwrap();
    GrayImage::decode(&png).unwrap()
}

/// `inner` pasted onto a `w` by `h` canvas of grey `ground` at `x`, `y`.
fn paste(inner: &GrayImage, w: u32, h: u32, x: u32, y: u32, ground: u8) -> GrayImage {
    let mut data = vec![ground; (w * h) as usize];
    for row in 0..inner.height {
        for col in 0..inner.width {
            let v = inner.data[(row * inner.width + col) as usize];
            data[((y + row) * w + x + col) as usize] = v;
        }
    }
    GrayImage::new(w, h, data).unwrap()
}

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/b5/qr")
}

#[test]
fn a_join_link_round_trips_through_png_at_every_scale() {
    for scale in [1, 2, 3, 4, 8, 16] {
        let png = encode(LINK).unwrap().to_png(scale).unwrap();
        assert_eq!(
            decode_bytes(&png).unwrap(),
            vec![LINK.to_string()],
            "{scale}"
        );
    }
}

#[test]
fn texts_of_every_kind_round_trip() {
    let long: String = "abcdefghij".repeat(100);
    for text in [
        "0123456789",
        "HTTPS://EXAMPLE.ORG/ABC",
        "Ada Example, caf\u{e9}, \u{6771}\u{4eac}, \u{1f426}",
        long.as_str(),
        LINK,
    ] {
        let img = image_of(text, 4);
        assert_eq!(decode_image(&img).unwrap(), vec![text.to_string()]);
    }
}

#[test]
fn rotated_codes_read() {
    let img = image_of(LINK, 4);
    for turns in 1..4 {
        assert_eq!(
            decode_image(&img.rotate(turns)).unwrap()[0],
            LINK,
            "{turns}"
        );
    }
}

#[test]
fn light_codes_on_dark_ground_read() {
    let mut img = image_of(LINK, 4);
    for v in &mut img.data {
        *v = 255 - *v;
    }
    let framed = paste(&img, img.width + 80, img.height + 80, 40, 40, 0);
    assert_eq!(decode_image(&framed).unwrap(), vec![LINK.to_string()]);
}

#[test]
fn a_code_in_a_larger_picture_reads() {
    let img = image_of(LINK, 3);
    let canvas = paste(&img, 900, 700, 517, 301, 200);
    assert_eq!(decode_image(&canvas).unwrap(), vec![LINK.to_string()]);
}

#[test]
fn a_very_wide_screenshot_is_scaled_down_and_reads() {
    let img = image_of(LINK, 6);
    let canvas = paste(&img, 4600, img.height + 20, 3000, 10, 255);
    assert_eq!(decode_image(&canvas).unwrap(), vec![LINK.to_string()]);
}

#[test]
fn two_codes_in_one_picture_both_read() {
    let a = image_of("first code", 4);
    let b = image_of("second code", 4);
    let w = a.width + b.width + 40;
    let h = a.height.max(b.height) + 20;
    let mut canvas = paste(&a, w, h, 10, 10, 255);
    for row in 0..b.height {
        for col in 0..b.width {
            canvas.data[((10 + row) * w + a.width + 30 + col) as usize] =
                b.data[(row * b.width + col) as usize];
        }
    }
    let mut texts = decode_image(&canvas).unwrap();
    texts.sort();
    assert_eq!(texts, vec!["first code", "second code"]);
}

#[test]
fn clipboard_pixels_in_color_read() {
    let img = image_of(LINK, 4);
    // Dark blue on pale yellow, fully opaque, plus a transparent border
    // that reads as white.
    let (w, h) = (img.width + 20, img.height + 20);
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            let inside = (10..10 + img.width).contains(&x) && (10..10 + img.height).contains(&y);
            if !inside {
                continue; // transparent
            }
            let dark = img.data[((y - 10) * img.width + x - 10) as usize] < 128;
            let px = if dark {
                [20, 30, 110, 255]
            } else {
                [250, 245, 200, 255]
            };
            rgba[i..i + 4].copy_from_slice(&px);
        }
    }
    assert_eq!(decode_rgba(w, h, &rgba).unwrap(), vec![LINK.to_string()]);
}

#[test]
fn a_blank_picture_has_no_code() {
    let blank = GrayImage::new(200, 120, vec![255; 200 * 120]).unwrap();
    assert!(matches!(decode_image(&blank), Err(QrError::NotFound)));
}

#[test]
fn a_damaged_code_is_found_but_unreadable() {
    let m = encode(LINK).unwrap();
    let scale = 4usize;
    let mut img = image_of(LINK, scale as u32);
    // Blank out a broad band through the middle, between the finder
    // patterns, so the grid is still found but the data is lost.
    let side = img.width as usize;
    let start = (QUIET_ZONE + 9) * scale;
    let end = (QUIET_ZONE + m.size() - 9) * scale;
    for y in start..end {
        for x in (QUIET_ZONE * scale)..(side - QUIET_ZONE * scale) {
            img.data[y * side + x] = 255;
        }
    }
    let err = decode_image(&img).unwrap_err();
    assert!(matches!(err, QrError::Unreadable), "{err}");
    assert!(!err.to_string().contains("ABCD"));
}

#[test]
fn errors_are_said_in_words() {
    assert_eq!(
        encode("").unwrap_err().to_string(),
        "there is nothing to put in a QR code"
    );
    let huge = "x".repeat(4000);
    assert_eq!(
        encode(&huge).unwrap_err().to_string(),
        "the link is too long to fit in a QR code"
    );
    let m = encode(LINK).unwrap();
    assert!(matches!(m.to_png(0), Err(QrError::Write(_))));
    assert!(matches!(m.to_png(65), Err(QrError::Write(_))));
    assert_eq!(
        decode_bytes(b"plain text, not a picture")
            .unwrap_err()
            .to_string(),
        "only PNG and JPEG images can be recognized"
    );
    let missing = fixtures().join("no-such-file.png");
    assert!(matches!(decode_file(&missing), Err(QrError::Io(_))));
}

#[test]
fn the_png_carries_no_text() {
    let png = encode(LINK).unwrap().to_png(8).unwrap();
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    for chunk in [&b"tEXt"[..], b"zTXt", b"iTXt"] {
        assert!(!png.windows(4).any(|w| w == chunk));
    }
    assert!(!png.windows(4).any(|w| w == b"ABCD"));
    let m = encode(LINK).unwrap();
    let img = GrayImage::decode(&png).unwrap();
    let side = (m.size() + 2 * QUIET_ZONE) as u32 * 8;
    assert_eq!((img.width, img.height), (side, side));
}

/// The SVG's path, read back module by module, is the code.
#[test]
fn the_svg_draws_the_code_and_names_itself() {
    let m = encode(LINK).unwrap();
    let svg = m.to_svg("QR code for joining \"demo\" & more");
    assert!(svg.contains("role=\"img\""));
    assert!(svg.contains("aria-label=\"QR code for joining &quot;demo&quot; &amp; more\""));
    assert!(svg.contains("<title>QR code for joining &quot;demo&quot; &amp; more</title>"));
    assert!(!svg.contains("ABCD"));
    let n = m.size() + 2 * QUIET_ZONE;
    assert!(svg.contains(&format!("viewBox=\"0 0 {n} {n}\"")));

    let d = svg
        .split("d=\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .unwrap();
    let mut drawn = vec![false; n * n];
    for seg in d.split('M').filter(|s| !s.is_empty()) {
        // "x yhNv1h-Nz"
        let (xy, rest) = seg.split_once('h').unwrap();
        let (x, y) = xy.split_once(' ').unwrap();
        let (x, y): (usize, usize) = (x.parse().unwrap(), y.parse().unwrap());
        let run: usize = rest.split_once('v').unwrap().0.parse().unwrap();
        for i in x..x + run {
            assert!(!drawn[y * n + i], "drawn twice at {i}, {y}");
            drawn[y * n + i] = true;
        }
    }
    for y in 0..n {
        for x in 0..n {
            let want =
                x >= QUIET_ZONE && y >= QUIET_ZONE && m.is_dark(x - QUIET_ZONE, y - QUIET_ZONE);
            assert_eq!(drawn[y * n + x], want, "{x}, {y}");
        }
    }
}

#[test]
fn rows_cover_the_matrix() {
    let m = encode("textweaver").unwrap();
    assert_eq!(m.size(), 21);
    assert_eq!(m.rows().count(), 21);
    assert!(m.rows().all(|r| r.len() == 21));
    assert!(!m.is_dark(21, 0) && !m.is_dark(0, 21));
}

/// The fuzz seeds: generated codes, kept small. Set
/// `TEXTWEAVER_UPDATE_FIXTURES=1` to write them again.
#[test]
fn the_fixtures_read() {
    let dir = fixtures();
    let mut dark = image_of(LINK, 3);
    for v in &mut dark.data {
        *v = 255 - *v;
    }
    let cases: [(&str, Vec<u8>, &str); 3] = [
        (
            "join-link.png",
            encode(LINK).unwrap().to_png(3).unwrap(),
            LINK,
        ),
        ("dark-theme.png", dark.to_png().unwrap(), LINK),
        (
            "plain-text.png",
            encode("textweaver").unwrap().to_png(2).unwrap(),
            "textweaver",
        ),
    ];
    let update = std::env::var_os("TEXTWEAVER_UPDATE_FIXTURES").is_some();
    for (name, bytes, text) in cases {
        let path = dir.join(name);
        if update {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(&path, &bytes).unwrap();
        }
        assert_eq!(
            decode_file(&path).unwrap(),
            vec![text.to_string()],
            "{name}"
        );
    }
}

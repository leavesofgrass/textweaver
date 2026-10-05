//! The app's icon: the loom, the owner's approved logo
//! (`docs/assets/textweaver-logo.svg`, and `textweaver-logo-mono.svg` for
//! the one-color form). It replaces W9a-w's lettermark.
//!
//! The logo is drawn here from the same shapes as the SVG ([`paint_logo`]),
//! so nothing new is needed to render it: `cargo run -p textweaver-xilem
//! --example icons` draws it with Vello's CPU renderer at every size
//! Windows asks for ([`SIZES`]) and writes the committed files in
//! `assets/icons/` ([`write_assets`]): one PNG per size, a multi-size
//! `.ico`, the high-contrast `.ico`, and `textweaver.res`, the compiled
//! resource each Windows program links (`build.rs`) so Explorer, shortcuts
//! and the taskbar show the loom.
//!
//! In Windows High Contrast the icon is the one-color logo, white on black,
//! so it stays clear whatever the system's colors.

use masonry::imaging::Painter;
use masonry::kurbo::{Affine, BezPath, Circle, Join, Line, Point, RoundedRect, Stroke};
use masonry::peniko::Color;

/// The sizes, in pixels, of the icon's images: what Windows asks for at
/// 100 to 200 percent in the title bar, the taskbar, Explorer, and the
/// Start menu.
pub const SIZES: [u32; 8] = [16, 20, 24, 32, 40, 48, 64, 256];

/// The resource id of the usual icon group in `textweaver.res` (the first,
/// so Explorer and shortcuts use it).
pub const RESOURCE_ICON: u16 = 1;
/// The resource id of the high-contrast icon group.
pub const RESOURCE_ICON_HIGH_CONTRAST: u16 = 2;

/// The 64-pixel images, for the window's icon where the resource is not
/// linked (Linux, and a build without it).
const PNG_64: &[u8] = include_bytes!("../assets/icons/textweaver-64.png");
const PNG_64_HIGH_CONTRAST: &[u8] =
    include_bytes!("../assets/icons/textweaver-high-contrast-64.png");

/// The logo's drawing space: a 256 by 256 square, as in the SVG.
const SPACE: f64 = 256.0;

/// The parchment of the badge.
const PARCHMENT: Color = Color::from_rgb8(0xf6, 0xef, 0xe0);
/// The ink: outlines, warp threads, and the t stem.
const INK: Color = Color::from_rgb8(0x22, 0x23, 0x4a);
/// The loom's wood: beams and posts.
const WOOD: Color = Color::from_rgb8(0x3b, 0x3f, 0x7f);
/// The weft thread.
const WEFT: Color = Color::from_rgb8(0xe8, 0x84, 0x2a);

/// The weft's zigzag, the pointed w.
const WEFT_POINTS: [(f64, f64); 5] = [
    (40.0, 62.0),
    (94.0, 170.0),
    (128.0, 118.0),
    (162.0, 170.0),
    (216.0, 62.0),
];
/// Warp threads the weft passes under (drawn before it).
const WARP_UNDER: [f64; 4] = [80.0, 112.0, 144.0, 176.0];
/// Warp threads the weft passes over (drawn after it).
const WARP_OVER: [f64; 4] = [64.0, 96.0, 160.0, 192.0];

fn polygon(points: &[(f64, f64)]) -> BezPath {
    let mut p = BezPath::new();
    for (i, &(x, y)) in points.iter().enumerate() {
        if i == 0 {
            p.move_to(Point::new(x, y));
        } else {
            p.line_to(Point::new(x, y));
        }
    }
    p.close_path();
    p
}

fn polyline(points: &[(f64, f64)]) -> BezPath {
    let mut p = BezPath::new();
    for (i, &(x, y)) in points.iter().enumerate() {
        if i == 0 {
            p.move_to(Point::new(x, y));
        } else {
            p.line_to(Point::new(x, y));
        }
    }
    p
}

fn rounded(x: f64, y: f64, w: f64, h: f64, r: f64) -> RoundedRect {
    RoundedRect::new(x, y, x + w, y + h, r)
}

fn warp(x: f64) -> Line {
    Line::new((x, 56.0), (x, 200.0))
}

fn weft_stroke(width: f64) -> Stroke {
    Stroke::new(width)
        .with_join(Join::Miter)
        .with_miter_limit(6.0)
}

/// Draws the logo in a `side` by `side` square at the origin: the color
/// logo, or with `high_contrast` the one-color logo in white on black.
pub fn paint_logo(painter: &mut Painter<'_>, side: f64, high_contrast: bool) {
    let t = Affine::scale(side / SPACE);
    let badge = rounded(8.0, 8.0, 240.0, 240.0, 44.0);
    let stem = polygon(&[(121.0, 20.0), (135.0, 28.0), (135.0, 174.0), (121.0, 174.0)]);
    let foot = polygon(&[
        (128.0, 168.0),
        (141.0, 184.0),
        (128.0, 200.0),
        (115.0, 184.0),
    ]);
    let beams = [
        rounded(26.0, 38.0, 204.0, 18.0, 5.0),
        rounded(26.0, 200.0, 204.0, 18.0, 5.0),
    ];
    let posts = [
        rounded(34.0, 42.0, 18.0, 172.0, 5.0),
        rounded(204.0, 42.0, 18.0, 172.0, 5.0),
    ];
    let weft = polyline(&WEFT_POINTS);
    let thin = Stroke::new(3.0);
    if high_contrast {
        // The one-color logo (`textweaver-logo-mono.svg`) on a black
        // badge. Its masks are drawn as black over white: the threads the
        // weft passes under are covered by the weft's full width, and the
        // weft is a hollow ribbon, its middle black.
        let (fg, bg) = (Color::WHITE, Color::BLACK);
        painter.fill(badge, bg).transform(t).draw();
        painter
            .stroke(badge, &Stroke::new(6.0), fg)
            .transform(t)
            .draw();
        for x in WARP_UNDER {
            painter.stroke(warp(x), &thin, fg).transform(t).draw();
        }
        painter.fill(&stem, fg).transform(t).draw();
        painter.fill(&foot, fg).transform(t).draw();
        painter
            .stroke(&weft, &weft_stroke(24.0), bg)
            .transform(t)
            .draw();
        for b in beams {
            painter.fill(b, fg).transform(t).draw();
        }
        painter
            .stroke(&weft, &weft_stroke(24.0), fg)
            .transform(t)
            .draw();
        painter
            .stroke(&weft, &weft_stroke(14.0), bg)
            .transform(t)
            .draw();
        for x in WARP_OVER {
            painter.stroke(warp(x), &thin, fg).transform(t).draw();
        }
        for p in posts {
            painter.fill(p, fg).transform(t).draw();
        }
        return;
    }
    painter.fill(badge, PARCHMENT).transform(t).draw();
    painter
        .stroke(badge, &Stroke::new(6.0), INK)
        .transform(t)
        .draw();
    for x in WARP_UNDER {
        painter.stroke(warp(x), &thin, INK).transform(t).draw();
    }
    painter.fill(&stem, INK).transform(t).draw();
    painter.fill(&foot, INK).transform(t).draw();
    for b in beams {
        painter.fill(b, WOOD).transform(t).draw();
        painter
            .stroke(b, &Stroke::new(4.0), INK)
            .transform(t)
            .draw();
    }
    painter
        .stroke(&weft, &weft_stroke(24.0), INK)
        .transform(t)
        .draw();
    painter
        .stroke(&weft, &weft_stroke(14.0), WEFT)
        .transform(t)
        .draw();
    for x in WARP_OVER {
        painter.stroke(warp(x), &thin, INK).transform(t).draw();
    }
    for p in posts {
        painter.fill(p, WOOD).transform(t).draw();
        painter
            .stroke(p, &Stroke::new(4.0), INK)
            .transform(t)
            .draw();
    }
    for (x, y) in [(43.0, 47.0), (213.0, 47.0), (43.0, 209.0), (213.0, 209.0)] {
        let rivet = Circle::new((x, y), 3.5);
        painter.fill(rivet, PARCHMENT).transform(t).draw();
        painter
            .stroke(rivet, &Stroke::new(2.0), INK)
            .transform(t)
            .draw();
    }
}

/// The 64-pixel icon as RGBA pixels: the usual one, or the high-contrast
/// one. `None` if the committed image cannot be read.
pub fn rgba(high_contrast: bool) -> Option<(Vec<u8>, u32, u32)> {
    let bytes = if high_contrast {
        PNG_64_HIGH_CONTRAST
    } else {
        PNG_64
    };
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::ALPHA);
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    buf.truncate(info.buffer_size());
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return None;
    }
    Some((buf, info.width, info.height))
}

/// The icon for winit, or `None` if it cannot be made. On Windows it is
/// the group linked into the program (every size, so the title bar and
/// the taskbar each get a sharp one); elsewhere, or without the
/// resource, the 64-pixel image.
pub fn window_icon(high_contrast: bool) -> Option<masonry_winit::winit::window::Icon> {
    #[cfg(windows)]
    {
        use masonry_winit::winit::platform::windows::IconExtWindows;
        let id = if high_contrast {
            RESOURCE_ICON_HIGH_CONTRAST
        } else {
            RESOURCE_ICON
        };
        if let Ok(icon) = masonry_winit::winit::window::Icon::from_resource(id, None) {
            return Some(icon);
        }
    }
    let (pixels, w, h) = rgba(high_contrast)?;
    masonry_winit::winit::window::Icon::from_rgba(pixels, w, h).ok()
}

/// A multi-size `.ico` file holding `images`: (side in pixels, PNG bytes),
/// smallest first. Windows Vista and later read PNG images in icons.
pub fn ico(images: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let count = u16::try_from(images.len()).unwrap_or(u16::MAX);
    let mut out = Vec::new();
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    let mut offset = 6 + 16 * images.len();
    for (side, png) in images {
        out.extend_from_slice(&dir_entry_head(*side));
        out.extend_from_slice(&len32(png).to_le_bytes());
        out.extend_from_slice(&u32::try_from(offset).unwrap_or(u32::MAX).to_le_bytes());
        offset += png.len();
    }
    for (_, png) in images {
        out.extend_from_slice(png);
    }
    out
}

/// An icon directory entry's first 8 bytes: width and height (0 means
/// 256), no palette, one plane, 32 bits per pixel. The image's size and
/// its offset (in a file) or id (in a resource) follow.
fn dir_entry_head(side: u32) -> [u8; 8] {
    let s = u8::try_from(side).unwrap_or(0);
    let mut e = [0u8; 8];
    e[0] = s;
    e[1] = s;
    e[4..6].copy_from_slice(&1u16.to_le_bytes());
    e[6..8].copy_from_slice(&32u16.to_le_bytes());
    e
}

/// One image of an icon: its side in pixels and its PNG bytes.
pub type IconImage = (u32, Vec<u8>);

fn len32(b: &[u8]) -> u32 {
    u32::try_from(b.len()).unwrap_or(u32::MAX)
}

/// A compiled Windows resource file (`.res`) holding icon groups: each
/// group is (group id, images), and its images get ids from 1 up. The
/// Microsoft linker takes a `.res` file as an input, so a program links
/// it with no resource compiler.
pub fn res(groups: &[(u16, &[IconImage])]) -> Vec<u8> {
    const RT_ICON: u16 = 3;
    const RT_GROUP_ICON: u16 = 14;
    const LANG_EN_US: u16 = 0x0409;
    fn entry(out: &mut Vec<u8>, kind: u16, id: u16, flags: u16, lang: u16, data: &[u8]) {
        out.extend_from_slice(&len32(data).to_le_bytes());
        out.extend_from_slice(&32u32.to_le_bytes());
        out.extend_from_slice(&0xFFFFu16.to_le_bytes());
        out.extend_from_slice(&kind.to_le_bytes());
        out.extend_from_slice(&0xFFFFu16.to_le_bytes());
        out.extend_from_slice(&id.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&flags.to_le_bytes());
        out.extend_from_slice(&lang.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(data);
        while !out.len().is_multiple_of(4) {
            out.push(0);
        }
    }
    let mut out = Vec::new();
    // The empty first entry every 32-bit resource file starts with.
    entry(&mut out, 0, 0, 0, 0, &[]);
    let mut next_image: u16 = 1;
    for (group, images) in groups {
        let mut dir = Vec::new();
        dir.extend_from_slice(&0u16.to_le_bytes());
        dir.extend_from_slice(&1u16.to_le_bytes());
        dir.extend_from_slice(
            &u16::try_from(images.len())
                .unwrap_or(u16::MAX)
                .to_le_bytes(),
        );
        for (side, png) in images.iter() {
            entry(&mut out, RT_ICON, next_image, 0x1010, LANG_EN_US, png);
            dir.extend_from_slice(&dir_entry_head(*side));
            dir.extend_from_slice(&len32(png).to_le_bytes());
            dir.extend_from_slice(&next_image.to_le_bytes());
            next_image += 1;
        }
        entry(&mut out, RT_GROUP_ICON, *group, 0x1030, LANG_EN_US, &dir);
    }
    out
}

/// Draws the logo in a `side`-pixel square with Vello's CPU renderer, as
/// straight (not premultiplied) RGBA, transparent outside the badge (the
/// harness already gives straight alpha: a corner pixel of the ink border
/// keeps the ink's color at partial alpha).
#[cfg(feature = "screenshot")]
pub fn render(side: u32, high_contrast: bool) -> Vec<u8> {
    use masonry::core::NewWidget;
    use masonry_testing::{TestHarness, TestHarnessParams};
    let mut params = TestHarnessParams::default();
    params.window_size = (side, side).into();
    params.background_color = Color::TRANSPARENT;
    params.root_padding = 0;
    params.scale_factor = 1.0;
    let palette = crate::theme::Palette::named("galaxy");
    let mut h = TestHarness::create_with(
        crate::theme::default_properties(&palette),
        NewWidget::new(LogoView { high_contrast }),
        params,
    );
    h.render().into_raw()
}

/// A PNG of `pixels` (straight RGBA, `side` square).
pub fn png_bytes(pixels: &[u8], side: u32) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    {
        let mut e = png::Encoder::new(&mut out, side, side);
        e.set_color(png::ColorType::Rgba);
        e.set_depth(png::BitDepth::Eight);
        e.set_compression(png::Compression::High);
        let mut w = e.write_header().map_err(|e| e.to_string())?;
        w.write_image_data(pixels).map_err(|e| e.to_string())?;
    }
    Ok(out)
}

/// Writes the icon files into `dir` (the crate's `assets/icons`): a PNG
/// per size and variant, `textweaver.ico`, `textweaver-high-contrast.ico`,
/// and `textweaver.res`. Returns the files written.
#[cfg(feature = "screenshot")]
pub fn write_assets(dir: &std::path::Path) -> Result<Vec<std::path::PathBuf>, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut written = Vec::new();
    let mut write = |name: String, bytes: &[u8]| -> Result<(), String> {
        let path = dir.join(name);
        std::fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        written.push(path);
        Ok(())
    };
    let mut sets: Vec<Vec<(u32, Vec<u8>)>> = Vec::new();
    for (hc, stem) in [(false, "textweaver"), (true, "textweaver-high-contrast")] {
        let mut images = Vec::new();
        for side in SIZES {
            let png = png_bytes(&render(side, hc), side)?;
            write(format!("{stem}-{side}.png"), &png)?;
            images.push((side, png));
        }
        write(format!("{stem}.ico"), &ico(&images))?;
        sets.push(images);
    }
    let res = res(&[
        (RESOURCE_ICON, sets[0].as_slice()),
        (RESOURCE_ICON_HIGH_CONTRAST, sets[1].as_slice()),
    ]);
    write("textweaver.res".to_owned(), &res)?;
    Ok(written)
}

/// The logo as a widget, filling its box, for rendering the icon files.
#[cfg(feature = "screenshot")]
struct LogoView {
    high_contrast: bool,
}

#[cfg(feature = "screenshot")]
mod view {
    use masonry::accesskit::{Node, Role};
    use masonry::core::{
        AccessCtx, ChildrenIds, LayoutCtx, MeasureCtx, NoAction, PaintCtx, PropertiesRef,
        RegisterCtx, Widget,
    };
    use masonry::imaging::Painter;
    use masonry::kurbo::{Axis, Size};
    use masonry::layout::{LenReq, Length};

    use super::LogoView;

    impl Widget for LogoView {
        type Action = NoAction;

        fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

        fn measure(
            &mut self,
            _ctx: &mut MeasureCtx<'_>,
            _props: &PropertiesRef<'_>,
            _axis: Axis,
            len_req: LenReq,
            _cross: Option<Length>,
        ) -> Length {
            match len_req {
                LenReq::FitContent(space) => space,
                _ => Length::px(256.0),
            }
        }

        fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

        fn paint(
            &mut self,
            ctx: &mut PaintCtx<'_>,
            _props: &PropertiesRef<'_>,
            painter: &mut Painter<'_>,
        ) {
            let size = ctx.content_box().size();
            super::paint_logo(painter, size.width.min(size.height), self.high_contrast);
        }

        fn accessibility_role(&self) -> Role {
            Role::Image
        }

        fn accessibility(
            &mut self,
            _ctx: &mut AccessCtx<'_>,
            _props: &PropertiesRef<'_>,
            node: &mut Node,
        ) {
            node.set_label("textweaver");
        }

        fn children_ids(&self) -> ChildrenIds {
            ChildrenIds::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pixel(p: &[u8], side: u32, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * side + x) * 4) as usize;
        [p[i], p[i + 1], p[i + 2], p[i + 3]]
    }

    #[test]
    fn the_committed_icons_are_the_loom() {
        for hc in [false, true] {
            let (p, w, h) = rgba(hc).expect("the committed 64-pixel icon");
            assert_eq!((w, h), (64, 64));
            assert_eq!(p.len(), 64 * 64 * 4);
            // The corner outside the rounded badge is transparent.
            assert_eq!(pixel(&p, 64, 0, 0)[3], 0, "high contrast {hc}");
            // The middle of the bottom beam is solid.
            assert_eq!(pixel(&p, 64, 32, 52)[3], 255, "high contrast {hc}");
        }
        let (hc, _, _) = rgba(true).expect("the high-contrast icon");
        // High contrast: the badge is black and the beam white.
        assert_eq!(pixel(&hc, 64, 32, 52), [255, 255, 255, 255]);
        assert_eq!(&pixel(&hc, 64, 5, 32)[..3], &[0, 0, 0]);
        assert!(window_icon(false).is_some());
    }

    #[test]
    fn an_ico_lists_its_images() {
        let images = vec![(16, vec![1, 2, 3]), (256, vec![4, 5])];
        let f = ico(&images);
        assert_eq!(&f[..6], &[0, 0, 1, 0, 2, 0]);
        // The first entry: 16 by 16, 3 bytes at offset 6 + 32.
        assert_eq!(f[6], 16);
        assert_eq!(&f[6 + 8..6 + 12], &3u32.to_le_bytes());
        assert_eq!(&f[6 + 12..6 + 16], &38u32.to_le_bytes());
        // The second: 256 is written as 0.
        assert_eq!(f[22], 0);
        assert_eq!(&f[38..], &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn the_committed_files_hold_every_size() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/icons");
        for stem in ["textweaver", "textweaver-high-contrast"] {
            let f = std::fs::read(dir.join(format!("{stem}.ico"))).expect("the .ico");
            assert_eq!(&f[..4], &[0, 0, 1, 0]);
            assert_eq!(usize::from(u16::from_le_bytes([f[4], f[5]])), SIZES.len());
            for (i, side) in SIZES.iter().enumerate() {
                let w = f[6 + 16 * i];
                assert_eq!(if *side == 256 { 0 } else { u32::from(w) }, *side % 256);
                assert!(dir.join(format!("{stem}-{side}.png")).is_file());
            }
        }
        let r = std::fs::read(dir.join("textweaver.res")).expect("the .res");
        // The empty first entry, then the first icon image (RT_ICON 3).
        assert_eq!(&r[..8], &[0, 0, 0, 0, 32, 0, 0, 0]);
        assert_eq!(&r[32 + 8..32 + 12], &[0xFF, 0xFF, 3, 0]);
    }

    #[cfg(feature = "screenshot")]
    #[test]
    fn the_logo_renders_at_every_size() {
        for side in [16, 64] {
            let p = render(side, false);
            assert_eq!(p.len(), (side * side * 4) as usize);
            assert_eq!(pixel(&p, side, 0, 0)[3], 0, "transparent corner");
            let mid = side / 2;
            assert_eq!(pixel(&p, side, mid, mid)[3], 255, "solid middle");
        }
    }
}

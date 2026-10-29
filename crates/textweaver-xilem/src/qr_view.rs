//! The join QR code in the textweaver app (beta 5, phase 7c): the image a
//! phone or another computer scans, inside the Show the join QR code
//! dialog (built in `crate::gui`).
//!
//! [`QrImage`] draws the app's code (`textweaver_app::live::qr::JoinQrView`)
//! black on white in every theme, with its quiet zone, since scanners need
//! that contrast; the colors carry no meaning of their own. To a screen
//! reader it is one image node named "QR code for joining HOST": never the
//! link, which holds the join code and is shown as text beside it.

use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, ChildrenIds, LayoutCtx, MeasureCtx, NoAction, PaintCtx, PropertiesRef, RegisterCtx,
    Widget,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Rect, Size};
use masonry::layout::{LenReq, Length};
use masonry::peniko::Color;
use textweaver_app::live::qr::QrModules;

/// Pixels to a module, at most: large enough for a phone across a desk.
pub const MODULE_PX: f64 = 6.0;

/// The smallest side the image is drawn at, in pixels.
pub const MIN_SIDE: f64 = 120.0;

/// The join QR code as an image.
pub struct QrImage {
    modules: QrModules,
    name: String,
}

impl QrImage {
    /// The code `modules`, named `name` for screen readers.
    pub fn new(modules: QrModules, name: impl Into<String>) -> Self {
        QrImage {
            modules,
            name: name.into(),
        }
    }

    /// The side it is drawn at in `room` pixels: whole pixels to a module
    /// where there is room for them, never less than [`MIN_SIDE`].
    pub fn side_in(&self, room: f64) -> f64 {
        let n = self.modules.side().max(1) as f64;
        let per = (room / n).floor().clamp(1.0, MODULE_PX);
        (per * n).max(MIN_SIDE.min(room))
    }

    /// Its accessible name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Widget for QrImage {
    type Action = NoAction;

    fn accepts_pointer_interaction(&self) -> bool {
        false
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(
        &mut self,
        _ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        _axis: Axis,
        len_req: LenReq,
        cross: Option<Length>,
    ) -> Length {
        let room = match len_req {
            LenReq::FitContent(space) => space.get(),
            _ => cross.map_or(f64::MAX, Length::get),
        };
        let full = self.modules.side() as f64 * MODULE_PX;
        Length::px(self.side_in(room.min(full)))
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(
        &mut self,
        ctx: &mut PaintCtx<'_>,
        _props: &PropertiesRef<'_>,
        painter: &mut Painter<'_>,
    ) {
        let size = ctx.content_box().size();
        let side = self.side_in(size.width.min(size.height));
        let n = self.modules.side().max(1);
        let per = side / n as f64;
        let x0 = ((size.width - side) / 2.0).max(0.0);
        let y0 = ((size.height - side) / 2.0).max(0.0);
        painter
            .fill(Rect::new(x0, y0, x0 + side, y0 + side), Color::WHITE)
            .draw();
        for y in 0..n {
            for x in 0..n {
                if self.modules.is_dark(x, y) {
                    let r = Rect::new(
                        x0 + x as f64 * per,
                        y0 + y as f64 * per,
                        x0 + (x + 1) as f64 * per,
                        y0 + (y + 1) as f64 * per,
                    );
                    painter.fill(r, Color::BLACK).draw();
                }
            }
        }
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
        node.set_label(self.name.as_str());
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }

    fn get_debug_text(&self) -> Option<String> {
        Some(self.name.clone())
    }
}

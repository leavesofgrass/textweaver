//! Smoke test of the toolchain.

use masonry::core::NewWidget;
use masonry::core::{ErasedAction, WidgetId};
use masonry::widgets::Label;
use masonry_winit::app::{AppDriver, DriverCtx, NewWindow, WindowId};
use masonry_winit::winit::window::Window;

struct Driver;

impl AppDriver for Driver {
    fn on_action(
        &mut self,
        _w: WindowId,
        _ctx: &mut DriverCtx<'_>,
        _id: WidgetId,
        _a: ErasedAction,
    ) {
    }
}

fn main() {
    let label = NewWidget::new(Label::new("textweaver"));
    let attrs = Window::default_attributes().with_title("textweaver");
    let _ = masonry_winit::app::run(
        vec![NewWindow::new(attrs, label.erased())],
        Driver,
        masonry::theme::default_property_set(),
    );
}

//! A thin draggable divider between two regions. It doesn't resize
//! anything by itself -- it just reports the drag delta along its own axis
//! via a callback, leaving the caller to decide what that means (e.g.
//! calling `CodeEditor::set_height`/adjusting a sidebar's width).

use std::any::Any;
use std::sync::RwLock;

use super::context::{BasicContext, Context};
use super::{Element, ViewLimits, ViewStretch};
use crate::support::color::Color;
use crate::support::point::Point;
use crate::support::theme::get_theme;
use crate::view::{CursorTracking, MouseButton, MouseButtonKind};

pub type SplitterDragCallback = Box<dyn Fn(f32) + Send + Sync>;

/// Which way a [`Splitter`] is drawn/dragged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitterOrientation {
    /// A horizontal bar, dragged up/down -- divides regions stacked
    /// vertically (e.g. in a `VTile`).
    Horizontal,
    /// A vertical bar, dragged left/right -- divides regions placed
    /// side by side (e.g. in an `HTile`).
    Vertical,
}

/// A thin draggable divider. See [`splitter`] (horizontal) and
/// [`vsplitter`] (vertical).
pub struct Splitter {
    orientation: SplitterOrientation,
    color: Color,
    active_color: Color,
    thickness: f32,
    dragging: RwLock<bool>,
    hovering: RwLock<bool>,
    // The drag axis's coordinate (`pos.y` for `Horizontal`, `pos.x` for
    // `Vertical`) at the last drag event, so `handle_drag` reports the
    // *incremental* delta since then rather than since the drag started.
    drag_last: RwLock<f32>,
    on_drag: Option<SplitterDragCallback>,
}

impl Splitter {
    fn with_orientation(orientation: SplitterOrientation) -> Self {
        let theme = get_theme();
        Self {
            orientation,
            color: theme.scrollbar_color,
            active_color: theme.scrollbar_color.level(1.3),
            thickness: 6.0,
            dragging: RwLock::new(false),
            hovering: RwLock::new(false),
            drag_last: RwLock::new(0.0),
            on_drag: None,
        }
    }

    /// Called on every drag movement with the delta (in points, positive
    /// down for a horizontal splitter, positive right for a vertical one)
    /// since the last call.
    pub fn on_drag<F: Fn(f32) + Send + Sync + 'static>(mut self, callback: F) -> Self {
        self.on_drag = Some(Box::new(callback));
        self
    }

    /// The drag axis's coordinate for a given point.
    fn axis_pos(&self, p: Point) -> f32 {
        match self.orientation {
            SplitterOrientation::Horizontal => p.y,
            SplitterOrientation::Vertical => p.x,
        }
    }
}

impl Default for Splitter {
    fn default() -> Self {
        Self::with_orientation(SplitterOrientation::Horizontal)
    }
}

impl Element for Splitter {
    fn limits(&self, _ctx: &BasicContext) -> ViewLimits {
        match self.orientation {
            SplitterOrientation::Horizontal => ViewLimits::min_size(0.0, self.thickness),
            SplitterOrientation::Vertical => ViewLimits::min_size(self.thickness, 0.0),
        }
    }

    fn stretch(&self) -> ViewStretch {
        match self.orientation {
            SplitterOrientation::Horizontal => ViewStretch::new(1.0, 0.0),
            SplitterOrientation::Vertical => ViewStretch::new(0.0, 1.0),
        }
    }

    fn draw(&self, ctx: &Context) {
        let mut canvas = ctx.canvas.borrow_mut();
        let active = *self.dragging.read().unwrap() || *self.hovering.read().unwrap();
        let color = if active { self.active_color } else { self.color };
        canvas.fill_style(color.with_alpha(if active { 0.7 } else { 0.4 }));
        canvas.fill_rect(ctx.bounds);
    }

    fn hit_test(
        &self,
        ctx: &Context,
        p: Point,
        _leaf: bool,
        _control: bool,
    ) -> Option<&dyn Element> {
        if ctx.bounds.contains(p) {
            Some(self)
        } else {
            None
        }
    }

    fn wants_control(&self) -> bool {
        true
    }

    fn handle_click(&self, _ctx: &Context, btn: MouseButton) -> bool {
        if btn.button != MouseButtonKind::Left {
            return false;
        }
        if btn.down {
            *self.dragging.write().unwrap() = true;
            *self.drag_last.write().unwrap() = self.axis_pos(btn.pos);
        } else {
            *self.dragging.write().unwrap() = false;
        }
        true
    }

    fn drag(&mut self, ctx: &Context, btn: MouseButton) {
        self.handle_drag(ctx, btn);
    }

    fn handle_drag(&self, _ctx: &Context, btn: MouseButton) {
        if !*self.dragging.read().unwrap() {
            return;
        }
        let mut last = self.drag_last.write().unwrap();
        let pos = self.axis_pos(btn.pos);
        let delta = pos - *last;
        *last = pos;
        if delta != 0.0 {
            if let Some(ref callback) = self.on_drag {
                callback(delta);
            }
        }
    }

    fn cursor(&mut self, _ctx: &Context, _p: Point, status: CursorTracking) -> bool {
        match status {
            CursorTracking::Entering | CursorTracking::Hovering => {
                *self.hovering.write().unwrap() = true
            }
            CursorTracking::Leaving => *self.hovering.write().unwrap() = false,
        }
        true
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Creates a horizontal (drag up/down) splitter, for dividing regions
/// stacked vertically (e.g. in a `VTile`).
pub fn splitter() -> Splitter {
    Splitter::with_orientation(SplitterOrientation::Horizontal)
}

/// Creates a vertical (drag left/right) splitter, for dividing regions
/// placed side by side (e.g. in an `HTile`).
pub fn vsplitter() -> Splitter {
    Splitter::with_orientation(SplitterOrientation::Vertical)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::tile::HTile;
    use crate::element::{share, Element};
    use crate::support::canvas::Canvas;
    use crate::view::MouseButtonKind;
    use std::cell::RefCell as StdRefCell;
    use std::sync::{Arc as StdArc, Mutex as StdMutex};

    /// A dummy element with a fixed min-size and configurable stretch, so
    /// the test controls the exact layout without depending on any real
    /// widget's own sizing quirks.
    struct Dummy {
        min: Point,
        stretch: ViewStretch,
    }
    impl Element for Dummy {
        fn limits(&self, _ctx: &BasicContext) -> ViewLimits {
            ViewLimits::min_size(self.min.x, self.min.y)
        }
        fn stretch(&self) -> ViewStretch {
            self.stretch
        }
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }

    #[test]
    fn dragging_a_vsplitter_nested_in_an_htile_reports_deltas() {
        let delta_total = StdArc::new(StdMutex::new(0.0f32));
        let recorded = delta_total.clone();

        let sidebar = Dummy { min: Point::new(240.0, 720.0), stretch: ViewStretch::new(0.0, 1.0) };
        let split = vsplitter().on_drag(move |delta| *recorded.lock().unwrap() += delta);
        let main = Dummy { min: Point::new(200.0, 720.0), stretch: ViewStretch::new(1.0, 1.0) };

        let htile = HTile::from_vec(vec![share(sidebar), share(split), share(main)]);

        let view = crate::view::View::new(crate::support::point::Extent::new(900.0, 720.0));
        let canvas = StdRefCell::new(Canvas::new(900, 720).unwrap());
        let bounds = crate::support::rect::Rect::new(0.0, 0.0, 900.0, 720.0);
        let ctx = Context::new(&view, &canvas, bounds);

        // Splitter should sit right after the 240pt-wide sidebar.
        let click_pos = Point::new(242.0, 300.0);
        assert!(
            htile.hit_test(&ctx, click_pos, false, false).is_some(),
            "hit_test should find something at the splitter's expected position"
        );

        let down = MouseButton {
            down: true,
            click_count: 1,
            button: MouseButtonKind::Left,
            modifiers: 0,
            pos: click_pos,
        };
        assert!(htile.handle_click(&ctx, down), "mouse-down on the splitter should be handled");

        let dragged = MouseButton { pos: Point::new(300.0, 300.0), ..down };
        htile.handle_drag(&ctx, dragged);

        let up = MouseButton { down: false, ..dragged };
        htile.handle_click(&ctx, up);

        assert_eq!(
            *delta_total.lock().unwrap(),
            58.0,
            "expected the full 300-242=58pt drag to reach the splitter's callback"
        );
    }

    /// Mirrors MKIDE's actual nesting exactly: an outer `VTile` (page
    /// chrome above/below) containing an `HTile` (sidebar | vsplitter |
    /// main area) whose third child is itself a `VTile` (tab bar |
    /// splitter | output) -- three levels deep, vs. the previous test's
    /// one. If `drag_capture` breaks down at deeper nesting, this is where
    /// it would show up.
    #[test]
    fn dragging_a_splitter_three_levels_deep_reports_deltas() {
        use crate::element::tile::VTile;

        let v_delta = StdArc::new(StdMutex::new(0.0f32));
        let h_delta = StdArc::new(StdMutex::new(0.0f32));

        // Every size below is chosen so the window's total height exactly
        // equals the sum of every min-height along the way -- zero "extra"
        // space anywhere, so `stretch` factors can't redistribute anything
        // and each element's position is exactly its neighbors' mins added
        // up (no need to separately account for an HTile row stretching
        // every child, including the sidebar, to match its tallest one).
        let sidebar = Dummy { min: Point::new(240.0, 496.0), stretch: ViewStretch::new(0.0, 1.0) };
        let vsplit = vsplitter().on_drag({
            let v_delta = v_delta.clone();
            move |delta| *v_delta.lock().unwrap() += delta
        });

        let tab_bar = Dummy { min: Point::new(200.0, 400.0), stretch: ViewStretch::new(1.0, 1.0) };
        let hsplit = splitter().on_drag({
            let h_delta = h_delta.clone();
            move |delta| *h_delta.lock().unwrap() += delta
        });
        let output = Dummy { min: Point::new(200.0, 90.0), stretch: ViewStretch::new(1.0, 0.0) };
        let main_area = VTile::from_vec(vec![share(tab_bar), share(hsplit), share(output)]);

        let body = HTile::from_vec(vec![share(sidebar), share(vsplit), share(main_area)]);

        let top = Dummy { min: Point::new(900.0, 20.0), stretch: ViewStretch::new(1.0, 0.0) };
        let status = Dummy { min: Point::new(900.0, 20.0), stretch: ViewStretch::new(1.0, 0.0) };
        let content = VTile::from_vec(vec![share(top), share(body), share(status)]);

        let window_height = 20.0 + 496.0 + 20.0;
        let view = crate::view::View::new(crate::support::point::Extent::new(900.0, window_height));
        let canvas = StdRefCell::new(Canvas::new(900, window_height as u32).unwrap());
        let bounds = crate::support::rect::Rect::new(0.0, 0.0, 900.0, window_height);
        let ctx = Context::new(&view, &canvas, bounds);

        // Vertical splitter: right after the sidebar, well within the body row's height.
        let v_click = Point::new(242.0, 300.0);
        let down = MouseButton { down: true, click_count: 1, button: MouseButtonKind::Left, modifiers: 0, pos: v_click };
        assert!(content.handle_click(&ctx, down), "click on the vertical splitter should be handled");
        content.handle_drag(&ctx, MouseButton { pos: Point::new(300.0, 300.0), ..down });
        content.handle_click(&ctx, MouseButton { down: false, pos: Point::new(300.0, 300.0), ..down });
        assert_eq!(*v_delta.lock().unwrap(), 58.0, "vertical splitter delta should reach its callback through 2 levels of nesting");

        // Horizontal splitter: `top`(20) + `tab_bar`(400) + a couple px
        // into the 6pt-thick splitter itself.
        let h_click = Point::new(500.0, 422.0);
        assert_eq!(20.0 + 400.0 + 2.0, h_click.y);
        let down = MouseButton { down: true, click_count: 1, button: MouseButtonKind::Left, modifiers: 0, pos: h_click };
        assert!(content.handle_click(&ctx, down), "click on the horizontal splitter should be handled");
        content.handle_drag(&ctx, MouseButton { pos: Point::new(500.0, 460.0), ..down });
        content.handle_click(&ctx, MouseButton { down: false, pos: Point::new(500.0, 460.0), ..down });
        assert_eq!(*h_delta.lock().unwrap(), 38.0, "horizontal splitter delta should reach its callback through 3 levels of nesting");
    }
}

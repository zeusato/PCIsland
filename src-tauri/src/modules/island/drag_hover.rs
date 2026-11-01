//! Arm the file receiver before OLE hover reaches a tiny/click-through island.
use crate::platform::window::PhysRect;

#[derive(Default)]
pub struct DragHover {
    start: Option<(i32, i32)>,
    from_outside: bool,
    pub active: bool,
}

impl DragHover {
    pub fn update(&mut self, pressed: bool, point: (i32, i32), inside: bool, scale: f64) -> bool {
        if !pressed {
            *self = Self::default();
            return false;
        }
        // The same width as the expanded shelf; minimal never requires pixel hunting.
        let band = |bottom: f64| PhysRect {
            left: (40.0 * scale) as i32,
            top: 0,
            right: (440.0 * scale) as i32,
            bottom: (bottom * scale) as i32,
        };
        let start = *self.start.get_or_insert_with(|| {
            // A drag that starts inside the top band is a tab reorder, a title-bar drag or
            // a text selection, not a file brought to the island.
            self.from_outside = !inside && !band(64.0).contains(point.0, point.1);
            point
        });
        let moved =
            (point.0 - start.0).abs().max((point.1 - start.1).abs()) >= (8.0 * scale) as i32;
        let receiver = band(if self.active { 220.0 } else { 64.0 });
        self.active = self.from_outside && moved && receiver.contains(point.0, point.1);
        self.active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_drag_opens_near_minimal_and_stays_open_in_expanded_shelf() {
        for scale in [1.0, 1.25, 1.5, 2.0] {
            let mut drag = DragHover::default();
            let p = |x, y| ((x * scale) as i32, (y * scale) as i32);
            assert!(!drag.update(true, p(240.0, 500.0), false, scale));
            assert!(drag.update(true, p(240.0, 50.0), false, scale));
            assert!(drag.update(true, p(240.0, 200.0), true, scale));
            assert!(!drag.update(true, p(450.0, 200.0), false, scale));
            assert!(drag.update(true, p(240.0, 20.0), false, scale));
            assert!(!drag.update(false, p(240.0, 20.0), true, scale));
        }
    }

    #[test]
    fn drags_that_start_in_the_top_band_do_not_arm_receiver() {
        // Browser tab reorder / maximized title bar: press at the top edge, move along it.
        let mut drag = DragHover::default();
        assert!(!drag.update(true, (100, 10), false, 1.0));
        assert!(!drag.update(true, (240, 12), false, 1.0));
        assert!(!drag.update(true, (300, 40), false, 1.0));
    }

    #[test]
    fn ordinary_hover_click_and_dragging_out_of_island_do_not_arm_receiver() {
        let mut drag = DragHover::default();
        assert!(!drag.update(false, (240, 20), true, 1.0));
        assert!(!drag.update(true, (240, 20), true, 1.0));
        assert!(!drag.update(true, (260, 30), true, 1.0));
        assert!(!drag.update(true, (240, 500), false, 1.0));
        assert!(!drag.update(true, (240, 30), true, 1.0));
        drag.update(false, (240, 30), true, 1.0);
        assert!(!drag.update(true, (240, 50), false, 1.0));
        assert!(!drag.update(true, (241, 51), false, 1.0));
    }
}

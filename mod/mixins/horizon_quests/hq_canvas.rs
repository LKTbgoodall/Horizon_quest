// The graphical quest book's maths: the canvas view (pan and zoom), where
// things land on screen, and dependency lines.
// Pure: no game types, so `cargo test` runs it. Coordinates are GUI pixels
// on screen and "world" units on the canvas (1 grid cell = GRID units).

/// The canvas's world units between two grid cells.
pub const GRID: f32 = 32.0;
/// A quest node's side, in world units.
pub const NODE: f32 = 24.0;
pub const MIN_ZOOM: f32 = 0.25;
pub const MAX_ZOOM: f32 = 4.0;
/// How much one wheel notch zooms.
pub const ZOOM_STEP: f32 = 1.15;
/// The book's band at the bottom, in GUI pixels (no band at the top).
pub const FOOTER: f32 = 14.0;

/// How the canvas is looked at: `pan` is the world point at the canvas's
/// centre, `zoom` the screen pixels per world unit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub pan: [f32; 2],
    pub zoom: f32,
}

impl Default for View {
    fn default() -> Self {
        Self { pan: [0.0, 0.0], zoom: 1.0 }
    }
}

impl View {
    /// Where world point `world` lands on screen, `origin` being the
    /// canvas's centre on screen.
    pub fn to_screen(&self, origin: [f32; 2], world: [f32; 2]) -> [f32; 2] {
        [
            origin[0] + (world[0] - self.pan[0]) * self.zoom,
            origin[1] + (world[1] - self.pan[1]) * self.zoom,
        ]
    }

    /// The world point under screen point `screen`.
    pub fn to_world(&self, origin: [f32; 2], screen: [f32; 2]) -> [f32; 2] {
        [
            (screen[0] - origin[0]) / self.zoom + self.pan[0],
            (screen[1] - origin[1]) / self.zoom + self.pan[1],
        ]
    }

    /// The pointer dragged the canvas by `delta` screen pixels: the world
    /// follows the pointer.
    pub fn drag(&mut self, delta: [f32; 2]) {
        self.pan[0] -= delta[0] / self.zoom;
        self.pan[1] -= delta[1] / self.zoom;
    }

    /// The wheel turned `steps` notches (positive zooms in) with the pointer
    /// at `cursor`: zoom, keeping the world point under the pointer still.
    pub fn zoom_at(&mut self, origin: [f32; 2], cursor: [f32; 2], steps: f64) {
        if !steps.is_finite() || steps == 0.0 {
            return;
        }
        let before = self.to_world(origin, cursor);
        let factor = ZOOM_STEP.powf(steps as f32);
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        let after = self.to_world(origin, cursor);
        self.pan[0] += before[0] - after[0];
        self.pan[1] += before[1] - after[1];
    }
}

/// The centre of a rect.
pub fn centre(rect: [f32; 4]) -> [f32; 2] {
    [rect[0] + rect[2] / 2.0, rect[1] + rect[3] / 2.0]
}

/// Whether point `p` is inside `rect`.
pub fn inside(rect: [f32; 4], p: [f32; 2]) -> bool {
    p[0] >= rect[0] && p[1] >= rect[1] && p[0] < rect[0] + rect[2] && p[1] < rect[1] + rect[3]
}

/// Whether box `b` overlaps `rect` at all (anything else is not drawn:
/// there is no 2D clip helper, so the book culls what is outside).
pub fn overlaps(rect: [f32; 4], b: [f32; 4]) -> bool {
    b[0] < rect[0] + rect[2] && b[0] + b[2] > rect[0] && b[1] < rect[1] + rect[3] && b[1] + b[3] > rect[1]
}

/// A node at grid cell `cell`: its centre in world units.
pub fn cell_centre(cell: [f32; 2]) -> [f32; 2] {
    [cell[0] * GRID, cell[1] * GRID]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: [f32; 2], b: [f32; 2]) -> bool {
        (a[0] - b[0]).abs() < 1e-3 && (a[1] - b[1]).abs() < 1e-3
    }

    #[test]
    fn screen_and_world_round_trip() {
        let view = View { pan: [10.0, -5.0], zoom: 2.0 };
        let origin = [200.0, 120.0];
        let p = [37.5, 12.25];
        assert!(close(view.to_world(origin, view.to_screen(origin, p)), p));
        // The pan point sits at the origin.
        assert!(close(view.to_screen(origin, view.pan), origin));
    }

    #[test]
    fn dragging_moves_the_world_with_the_pointer() {
        let mut view = View { pan: [0.0, 0.0], zoom: 2.0 };
        let origin = [100.0, 100.0];
        let world = [5.0, 5.0];
        let before = view.to_screen(origin, world);
        view.drag([12.0, -4.0]);
        let after = view.to_screen(origin, world);
        assert!(close(after, [before[0] + 12.0, before[1] - 4.0]));
    }

    #[test]
    fn zoom_keeps_the_point_under_the_cursor() {
        let mut view = View::default();
        let origin = [160.0, 90.0];
        let cursor = [220.0, 40.0];
        let under = view.to_world(origin, cursor);
        view.zoom_at(origin, cursor, 3.0);
        assert!(view.zoom > 1.0);
        assert!(close(view.to_world(origin, cursor), under));
    }

    #[test]
    fn zoom_is_clamped_and_ignores_nonsense() {
        let mut view = View::default();
        view.zoom_at([0.0, 0.0], [0.0, 0.0], 1000.0);
        assert_eq!(view.zoom, MAX_ZOOM);
        view.zoom_at([0.0, 0.0], [0.0, 0.0], -1000.0);
        assert_eq!(view.zoom, MIN_ZOOM);
        let before = view;
        view.zoom_at([0.0, 0.0], [5.0, 5.0], f64::NAN);
        view.zoom_at([0.0, 0.0], [5.0, 5.0], 0.0);
        assert_eq!(view, before);
    }

    #[test]
    fn rect_tests() {
        let r = [0.0, 0.0, 10.0, 10.0];
        assert!(inside(r, [0.0, 0.0]));
        assert!(!inside(r, [10.0, 5.0]));
        assert!(overlaps(r, [9.0, 9.0, 5.0, 5.0]));
        assert!(!overlaps(r, [10.0, 0.0, 5.0, 5.0]));
        assert_eq!(centre([10.0, 20.0, 100.0, 50.0]), [60.0, 45.0]);
        assert_eq!(cell_centre([1.0, -2.0]), [GRID, -2.0 * GRID]);
    }
}

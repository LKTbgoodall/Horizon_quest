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

/// A line from `a` to `b`, `width` pixels thick, as the four corners of a
/// quad (in order round it). None when the ends are the same point.
pub fn line_quad(a: [f32; 2], b: [f32; 2], width: f32) -> Option<[[f32; 2]; 4]> {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len = (dx * dx + dy * dy).sqrt();
    if !len.is_finite() || len < 0.001 {
        return None;
    }
    let (nx, ny) = (-dy / len * width / 2.0, dx / len * width / 2.0);
    Some([
        [a[0] + nx, a[1] + ny],
        [b[0] + nx, b[1] + ny],
        [b[0] - nx, b[1] - ny],
        [a[0] - nx, a[1] - ny],
    ])
}

/// The most chevrons one line carries.
const CHEVRONS_MAX: usize = 64;

/// The chevrons along a line from `a` to `b` that is `width` pixels thick,
/// pointing towards `b`, one every `spacing` pixels, all moved `phase`
/// pixels towards `b` (an animation: from 0 to `spacing`, they slide one
/// step): each chevron is two quads, its arms meeting at its tip.
pub fn chevrons(a: [f32; 2], b: [f32; 2], width: f32, spacing: f32, phase: f32) -> Vec<[[f32; 2]; 4]> {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len = (dx * dx + dy * dy).sqrt();
    if !len.is_finite() || len < 0.001 || !(spacing > 0.0) || !(width > 0.0) {
        return Vec::new();
    }
    let (ux, uy) = (dx / len, dy / len);
    let (nx, ny) = (-uy, ux);
    let (half_length, half_width, thickness) = (width * 0.3, width * 0.32, (width * 0.24).max(1.0));
    let mut quads = Vec::new();
    let mut t = (spacing / 2.0 + phase.max(0.0)) % spacing;
    while t < len && quads.len() < 2 * CHEVRONS_MAX {
        let (px, py) = (a[0] + ux * t, a[1] + uy * t);
        let tip = [px + ux * half_length, py + uy * half_length];
        for side in [1.0, -1.0] {
            let back = [px - ux * half_length + nx * half_width * side, py - uy * half_length + ny * half_width * side];
            quads.extend(line_quad(back, tip, thickness));
        }
        t += spacing;
    }
    quads
}

/// The bounding box of a quad's corners.
pub fn quad_bounds(q: &[[f32; 2]; 4]) -> [f32; 4] {
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for p in q {
        x0 = x0.min(p[0]);
        y0 = y0.min(p[1]);
        x1 = x1.max(p[0]);
        y1 = y1.max(p[1]);
    }
    [x0, y0, x1 - x0, y1 - y0]
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

    #[test]
    fn lines_are_quads_of_the_right_width() {
        let q = line_quad([0.0, 0.0], [10.0, 0.0], 2.0).unwrap();
        assert!(close(q[0], [0.0, 1.0]) && close(q[1], [10.0, 1.0]));
        assert!(close(q[2], [10.0, -1.0]) && close(q[3], [0.0, -1.0]));
        assert_eq!(quad_bounds(&q), [0.0, -1.0, 10.0, 2.0]);
        assert_eq!(line_quad([3.0, 3.0], [3.0, 3.0], 2.0), None);
    }

    #[test]
    fn chevrons_point_along_the_line_at_even_steps() {
        // A line 40 long to the right, a chevron every 10: at 5, 15, 25, 35.
        let quads = chevrons([0.0, 0.0], [40.0, 0.0], 10.0, 10.0, 0.0);
        assert_eq!(quads.len(), 8);
        // Each chevron's two arms meet at its tip, ahead of its middle, and spread behind it.
        let tip = |q: &[[f32; 2]; 4]| [(q[1][0] + q[2][0]) / 2.0, (q[1][1] + q[2][1]) / 2.0];
        let back = |q: &[[f32; 2]; 4]| [(q[0][0] + q[3][0]) / 2.0, (q[0][1] + q[3][1]) / 2.0];
        assert!(close(tip(&quads[0]), [8.0, 0.0]) && close(tip(&quads[1]), [8.0, 0.0]));
        assert!(close(back(&quads[0]), [2.0, 3.2]) && close(back(&quads[1]), [2.0, -3.2]));
        assert!(close(tip(&quads[6]), [38.0, 0.0]));
        // Reversed, they point the other way.
        let back_again = chevrons([40.0, 0.0], [0.0, 0.0], 10.0, 10.0, 0.0);
        assert!(close(tip(&back_again[0]), [32.0, 0.0]));
        // Nothing for a point, a bad spacing, or a line too short for one.
        assert!(chevrons([1.0, 1.0], [1.0, 1.0], 10.0, 10.0, 0.0).is_empty());
        assert!(chevrons([0.0, 0.0], [40.0, 0.0], 10.0, 0.0, 0.0).is_empty());
        assert!(chevrons([0.0, 0.0], [4.0, 0.0], 10.0, 10.0, 0.0).is_empty());
        // Moved by a phase, they slide towards the end; a whole step brings them back.
        let moved = chevrons([0.0, 0.0], [40.0, 0.0], 10.0, 10.0, 3.0);
        assert!(close(tip(&moved[0]), [11.0, 0.0]));
        assert_eq!(chevrons([0.0, 0.0], [40.0, 0.0], 10.0, 10.0, 10.0), quads);
        // A very long line carries a bounded number.
        assert_eq!(chevrons([0.0, 0.0], [1.0e6, 0.0], 10.0, 1.0, 0.0).len(), 128);
    }
}

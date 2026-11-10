// The graphical quest book's maths: the canvas view (pan and zoom), where
// things land on screen, what the pointer is over, and dependency lines.
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
/// The header and footer bands of the book, in GUI pixels.
pub const HEADER: f32 = 24.0;
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

/// The canvas area inside the book's `rect` (`[x, y, w, h]`): the rect
/// without its header and footer bands.
pub fn canvas_rect(rect: [f32; 4]) -> [f32; 4] {
    let [x, y, w, h] = rect;
    [x, y + HEADER, w, (h - HEADER - FOOTER).max(0.0)]
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

/// A node's box on screen, centred on its cell.
pub fn node_box(view: &View, origin: [f32; 2], cell: [f32; 2]) -> [f32; 4] {
    let [sx, sy] = view.to_screen(origin, cell_centre(cell));
    let side = NODE * view.zoom;
    [sx - side / 2.0, sy - side / 2.0, side, side]
}

/// The node under screen point `p`, the last drawn (on top) first.
pub fn hit(view: &View, origin: [f32; 2], cells: &[[f32; 2]], p: [f32; 2]) -> Option<usize> {
    cells
        .iter()
        .enumerate()
        .rev()
        .find(|&(_, &cell)| inside(node_box(view, origin, cell), p))
        .map(|(i, _)| i)
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

/// One node of the spike's demo chapter: its grid cell, its icon item and
/// its label. Real quests replace these in phase 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DemoNode {
    pub cell: [f32; 2],
    pub item: &'static str,
    pub label: &'static str,
}

pub const DEMO: [DemoNode; 6] = [
    DemoNode { cell: [-3.0, 0.0], item: "minecraft:oak_log", label: "Wood" },
    DemoNode { cell: [-1.0, 0.0], item: "minecraft:crafting_table", label: "Crafting table" },
    DemoNode { cell: [1.0, -1.0], item: "minecraft:wooden_pickaxe", label: "First pickaxe" },
    DemoNode { cell: [1.0, 1.0], item: "minecraft:furnace", label: "Furnace" },
    DemoNode { cell: [3.0, -1.0], item: "minecraft:iron_pickaxe", label: "Iron tools" },
    DemoNode { cell: [3.0, 1.0], item: "minecraft:bread", label: "Food" },
];

/// The demo's dependency lines: (from, to) indexes into `DEMO`.
pub const DEMO_LINKS: [(usize, usize); 5] = [(0, 1), (1, 2), (1, 3), (2, 4), (3, 4)];

/// The demo nodes' cells, as `hit` takes them.
pub fn demo_cells() -> [[f32; 2]; 6] {
    DEMO.map(|n| n.cell)
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
    fn canvas_leaves_room_for_header_and_footer() {
        assert_eq!(canvas_rect([0.0, 0.0, 320.0, 240.0]), [0.0, HEADER, 320.0, 240.0 - HEADER - FOOTER]);
        assert_eq!(canvas_rect([0.0, 0.0, 100.0, 10.0])[3], 0.0);
        assert_eq!(centre([10.0, 20.0, 100.0, 50.0]), [60.0, 45.0]);
    }

    #[test]
    fn rect_tests() {
        let r = [0.0, 0.0, 10.0, 10.0];
        assert!(inside(r, [0.0, 0.0]));
        assert!(!inside(r, [10.0, 5.0]));
        assert!(overlaps(r, [9.0, 9.0, 5.0, 5.0]));
        assert!(!overlaps(r, [10.0, 0.0, 5.0, 5.0]));
    }

    #[test]
    fn hits_find_the_node_under_the_pointer() {
        let view = View::default();
        let origin = [0.0, 0.0];
        let cells = demo_cells();
        // The centre of node 2 (cell [1, -1]) is at (32, -32).
        assert_eq!(hit(&view, origin, &cells, [32.0, -32.0]), Some(2));
        assert_eq!(hit(&view, origin, &cells, [32.0 + NODE, -32.0]), None);
        assert_eq!(hit(&view, origin, &cells, [0.0, 0.0]), None);
        // Zoomed in, the node is bigger.
        let zoomed = View { pan: [0.0, 0.0], zoom: 2.0 };
        assert_eq!(hit(&zoomed, origin, &cells, [64.0 + NODE * 0.9, -64.0]), Some(2));
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
    fn demo_links_point_at_demo_nodes() {
        for (a, b) in DEMO_LINKS {
            assert!(a < DEMO.len() && b < DEMO.len() && a != b);
        }
    }
}

use rand::Rng;
use rand_chacha::ChaCha8Rng;

use crate::generator::bsp::{Rect, Room};

#[derive(Debug, Clone)]
pub struct Corridor {
    pub segments: Vec<Rect>,
}

impl Corridor {
    fn horizontal(x1: u32, x2: u32, y: u32) -> Rect {
        let (x_min, x_max) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
        Rect::new(x_min, y, x_max - x_min + 1, 1)
    }

    fn vertical(y1: u32, y2: u32, x: u32) -> Rect {
        let (y_min, y_max) = if y1 <= y2 { (y1, y2) } else { (y2, y1) };
        Rect::new(x, y_min, 1, y_max - y_min + 1)
    }
}

pub fn stitch(rooms: &[Room], rng: &mut ChaCha8Rng) -> Vec<Corridor> {
    if rooms.len() < 2 {
        return Vec::new();
    }

    let mut corridors = Vec::new();

    // Connect each room to the next in sequence (simple chain ensures connectivity)
    for pair in rooms.windows(2) {
        let (cx1, cy1) = pair[0].bounds.center();
        let (cx2, cy2) = pair[1].bounds.center();

        let horizontal_first = rng.gen_bool(0.5);
        let segments = if horizontal_first {
            vec![
                Corridor::horizontal(cx1, cx2, cy1),
                Corridor::vertical(cy1, cy2, cx2),
            ]
        } else {
            vec![
                Corridor::vertical(cy1, cy2, cx1),
                Corridor::horizontal(cx1, cx2, cy2),
            ]
        };

        corridors.push(Corridor { segments });
    }

    corridors
}

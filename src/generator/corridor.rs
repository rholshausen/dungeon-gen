use std::f32::consts::PI;

use rand::Rng;
use rand_chacha::ChaCha8Rng;

use crate::config::Difficulty;
use crate::data::event::Event;
use crate::generator::bsp::{Rect, Room, RoomShape};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoorKind {
    Wooden,
    Iron,
    Secret,
}

#[derive(Debug, Clone)]
pub struct Door {
    pub kind: DoorKind,
    pub locked: bool,
    /// Difficulty description for picking the lock; `None` when the door is not locked.
    pub lock_difficulty: Option<String>,
    /// Grid x (fractional tiles):
    ///   horizontal corridor → wall boundary (shape-aware; may be inset from bounding box)
    ///   vertical corridor   → visual centre column of the corridor
    pub x: f32,
    /// Grid y (fractional tiles):
    ///   horizontal corridor → visual centre row of the corridor
    ///   vertical corridor   → wall boundary (shape-aware; may be inset from bounding box)
    pub y: f32,
    /// True when the corridor runs left–right (door face is perpendicular, i.e. vertical).
    pub in_horizontal_corridor: bool,
}

/// Returns a game-system-agnostic lock-picking difficulty label scaled to door kind and dungeon
/// difficulty. Iron locks are one tier harder than wooden locks at the same dungeon difficulty.
pub fn lock_picking_difficulty(kind: DoorKind, difficulty: Difficulty) -> &'static str {
    match (kind, difficulty) {
        (DoorKind::Wooden, Difficulty::Easy)   => "Simple",
        (DoorKind::Wooden, Difficulty::Medium) => "Moderate",
        (DoorKind::Wooden, Difficulty::Hard)   => "Difficult",
        (DoorKind::Wooden, Difficulty::Deadly) => "Expert",
        (DoorKind::Iron,   Difficulty::Easy)   => "Moderate",
        (DoorKind::Iron,   Difficulty::Medium) => "Difficult",
        (DoorKind::Iron,   Difficulty::Hard)   => "Expert",
        (DoorKind::Iron,   Difficulty::Deadly) => "Master",
        _                                      => "Moderate", // Secret doors are never locked
    }
}

#[derive(Debug, Clone)]
pub struct Corridor {
    pub id: usize,
    pub segments: Vec<Rect>,
    pub assigned_event: Option<Event>,
    pub doors: Vec<Door>,
}

impl Corridor {
    fn horizontal(x1: u32, x2: u32, y: u32) -> Rect {
        let (x_min, x_max) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
        Rect::new(x_min, y, x_max - x_min, 1)
    }

    fn vertical(y1: u32, y2: u32, x: u32) -> Rect {
        let (y_min, y_max) = if y1 <= y2 { (y1, y2) } else { (y2, y1) };
        Rect::new(x, y_min, 1, y_max - y_min)
    }
}

fn random_door_kind(rng: &mut ChaCha8Rng) -> DoorKind {
    let roll: f32 = rng.gen_range(0.0..1.0);
    if roll < 0.65 {
        DoorKind::Wooden
    } else if roll < 0.90 {
        DoorKind::Iron
    } else {
        DoorKind::Secret
    }
}

/// X position (in grid tile units, possibly fractional) where a horizontal corridor meets the
/// room's left or right wall, accounting for the actual wall position of each room shape.
///
/// Hex rooms are pointy-top (start_angle PI/2), so the leftmost/rightmost vertices sit at
/// ±rx·cos(PI/6) ≈ ±0.866·rx rather than ±rx.
/// Oct rooms (start_angle PI/8) have their flat left/right edges at ±rx·cos(PI/8) ≈ ±0.924·rx.
fn room_horizontal_wall_x(room: &Room, going_right: bool) -> f32 {
    let cx = room.bounds.x as f32 + room.bounds.width as f32 / 2.0;
    let rx = room.bounds.width as f32 / 2.0;
    let offset = match room.shape {
        RoomShape::Rectangle | RoomShape::Round => rx,
        RoomShape::Hexagonal => rx * (PI / 6.0).cos(), // √3/2 ≈ 0.866
        RoomShape::Octagonal => rx * (PI / 8.0).cos(), // ≈ 0.924
    };
    if going_right { cx + offset } else { cx - offset }
}

/// Y position (in grid tile units, possibly fractional) where a vertical corridor meets the
/// room's top or bottom wall (grid Y increases downward).
///
/// Hex rooms have vertices at the exact top/bottom (full ry).
/// Oct rooms (start_angle PI/8) have their flat top/bottom edges at ±ry·cos(PI/8) ≈ ±0.924·ry.
fn room_vertical_wall_y(room: &Room, going_down: bool) -> f32 {
    let cy = room.bounds.y as f32 + room.bounds.height as f32 / 2.0;
    let ry = room.bounds.height as f32 / 2.0;
    let offset = match room.shape {
        RoomShape::Rectangle | RoomShape::Round | RoomShape::Hexagonal => ry,
        RoomShape::Octagonal => ry * (PI / 8.0).cos(), // ≈ 0.924
    };
    if going_down { cy + offset } else { cy - offset }
}

/// Wall-boundary position where the corridor leaves `room` (room-A end of the corridor).
/// Returns (door_x, door_y, in_horizontal_corridor) in fractional grid-tile units.
fn exit_door_pos(room: &Room, cx1: u32, cy1: u32, cx2: u32, cy2: u32, seg_is_horiz: bool) -> (f32, f32, bool) {
    if seg_is_horiz {
        let going_right = cx2 >= cx1;
        (room_horizontal_wall_x(room, going_right), cy1 as f32, true)
    } else {
        let going_down = cy2 >= cy1;
        (cx1 as f32, room_vertical_wall_y(room, going_down), false)
    }
}

/// Wall-boundary position where the corridor enters `room` (room-B end of the corridor).
/// Returns (door_x, door_y, in_horizontal_corridor) in fractional grid-tile units.
fn entry_door_pos(room: &Room, cx1: u32, cy1: u32, cx2: u32, cy2: u32, seg_is_horiz: bool) -> (f32, f32, bool) {
    if seg_is_horiz {
        // Entering B from the opposite side to the exit direction.
        let going_right = cx2 >= cx1;
        (room_horizontal_wall_x(room, !going_right), cy2 as f32, true)
    } else {
        let going_down = cy2 >= cy1;
        (cx2 as f32, room_vertical_wall_y(room, !going_down), false)
    }
}

fn maybe_door(x: f32, y: f32, in_horiz: bool, difficulty: Difficulty, rng: &mut ChaCha8Rng) -> Option<Door> {
    if !rng.gen_bool(0.5) {
        return None;
    }
    let kind = random_door_kind(rng);
    let locked = !matches!(kind, DoorKind::Secret) && rng.gen_bool(0.3);
    let lock_difficulty = locked.then(|| lock_picking_difficulty(kind, difficulty).to_string());
    Some(Door { kind, locked, lock_difficulty, x, y, in_horizontal_corridor: in_horiz })
}

pub fn stitch(rooms: &[Room], difficulty: Difficulty, rng: &mut ChaCha8Rng) -> Vec<Corridor> {
    if rooms.len() < 2 {
        return Vec::new();
    }

    let mut corridors = Vec::new();

    for pair in rooms.windows(2) {
        let room_a = &pair[0];
        let room_b = &pair[1];
        let (cx1, cy1) = room_a.bounds.center();
        let (cx2, cy2) = room_b.bounds.center();

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

        let seg0_valid = segments[0].width > 0 && segments[0].height > 0;
        let seg1_valid = segments[1].width > 0 && segments[1].height > 0;

        let mut doors = Vec::new();
        if seg0_valid || seg1_valid {
            // seg0 is horizontal when horizontal_first, vertical otherwise.
            let seg0_is_horiz = horizontal_first;

            // room_a always connects via seg0 if valid; if not, fall back to seg1.
            let a_is_horiz = if seg0_valid { seg0_is_horiz } else { !seg0_is_horiz };
            // room_b always connects via seg1 if valid; if not, fall back to seg0.
            let b_is_horiz = if seg1_valid { !seg0_is_horiz } else { seg0_is_horiz };

            let (ax, ay, ah) = exit_door_pos(room_a, cx1, cy1, cx2, cy2, a_is_horiz);
            let (bx, by, bh) = entry_door_pos(room_b, cx1, cy1, cx2, cy2, b_is_horiz);

            if let Some(d) = maybe_door(ax, ay, ah, difficulty, rng) {
                doors.push(d);
            }
            if let Some(d) = maybe_door(bx, by, bh, difficulty, rng) {
                doors.push(d);
            }
        }

        corridors.push(Corridor { id: corridors.len(), segments, assigned_event: None, doors });
    }

    corridors
}

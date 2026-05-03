use rand::Rng;
use rand_chacha::ChaCha8Rng;

use crate::config::Difficulty;
use crate::data::event::Event;
use crate::generator::bsp::{Rect, Room};

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
    /// Grid x:
    ///   horizontal corridor → wall boundary tile index (room's left or right edge)
    ///   vertical corridor   → visual centre column of the corridor
    pub x: u32,
    /// Grid y:
    ///   horizontal corridor → visual centre row of the corridor
    ///   vertical corridor   → wall boundary tile index (room's top or bottom edge)
    pub y: u32,
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

/// Wall-boundary position where the corridor leaves `room` (room-A end of the corridor).
/// Returns (door_x, door_y, in_horizontal_corridor).
fn exit_door_pos(room: &Room, cx1: u32, cy1: u32, cx2: u32, cy2: u32, seg_is_horiz: bool) -> (u32, u32, bool) {
    if seg_is_horiz {
        let wall_x = if cx2 >= cx1 { room.bounds.x + room.bounds.width } else { room.bounds.x };
        (wall_x, cy1, true)
    } else {
        let wall_y = if cy2 >= cy1 { room.bounds.y + room.bounds.height } else { room.bounds.y };
        (cx1, wall_y, false)
    }
}

/// Wall-boundary position where the corridor enters `room` (room-B end of the corridor).
/// Returns (door_x, door_y, in_horizontal_corridor).
fn entry_door_pos(room: &Room, cx1: u32, cy1: u32, cx2: u32, cy2: u32, seg_is_horiz: bool) -> (u32, u32, bool) {
    if seg_is_horiz {
        let wall_x = if cx2 >= cx1 { room.bounds.x } else { room.bounds.x + room.bounds.width };
        (wall_x, cy2, true)
    } else {
        let wall_y = if cy2 >= cy1 { room.bounds.y } else { room.bounds.y + room.bounds.height };
        (cx2, wall_y, false)
    }
}

fn maybe_door(x: u32, y: u32, in_horiz: bool, difficulty: Difficulty, rng: &mut ChaCha8Rng) -> Option<Door> {
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

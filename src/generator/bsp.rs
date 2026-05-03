use rand::Rng;
use rand_chacha::ChaCha8Rng;

use crate::config::DungeonConfig;
use crate::data::creature::Creature;
use crate::data::event::Event;

const MIN_PARTITION_SIZE: u32 = 8;
const ROOM_MARGIN: u32 = 2;

#[derive(Debug, Clone)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Rect { x, y, width, height }
    }

    pub fn center(&self) -> (u32, u32) {
        (self.x + self.width / 2, self.y + self.height / 2)
    }

    pub fn right(&self) -> u32 {
        self.x + self.width
    }

    pub fn bottom(&self) -> u32 {
        self.y + self.height
    }
}

#[derive(Debug, Clone)]
pub struct Room {
    pub id: usize,
    pub bounds: Rect,
    pub assigned_event: Option<Event>,
    pub assigned_creatures: Vec<Creature>,
}

impl Room {
    fn new(id: usize, bounds: Rect) -> Self {
        Room {
            id,
            bounds,
            assigned_event: None,
            assigned_creatures: Vec::new(),
        }
    }
}

enum Partition {
    Leaf(Rect),
    Split {
        left: Box<Partition>,
        right: Box<Partition>,
    },
}

impl Partition {
    fn split(rect: Rect, rng: &mut ChaCha8Rng) -> Self {
        let can_split_h = rect.height >= MIN_PARTITION_SIZE * 2;
        let can_split_v = rect.width >= MIN_PARTITION_SIZE * 2;

        if !can_split_h && !can_split_v {
            return Partition::Leaf(rect);
        }

        let split_horizontal = if can_split_h && can_split_v {
            rng.gen_bool(0.5)
        } else {
            can_split_h
        };

        if split_horizontal {
            let split_at = rng.gen_range(MIN_PARTITION_SIZE..rect.height - MIN_PARTITION_SIZE + 1);
            let top = Rect::new(rect.x, rect.y, rect.width, split_at);
            let bottom = Rect::new(rect.x, rect.y + split_at, rect.width, rect.height - split_at);
            Partition::Split {
                left: Box::new(Partition::split(top, rng)),
                right: Box::new(Partition::split(bottom, rng)),
            }
        } else {
            let split_at = rng.gen_range(MIN_PARTITION_SIZE..rect.width - MIN_PARTITION_SIZE + 1);
            let left_rect = Rect::new(rect.x, rect.y, split_at, rect.height);
            let right_rect = Rect::new(rect.x + split_at, rect.y, rect.width - split_at, rect.height);
            Partition::Split {
                left: Box::new(Partition::split(left_rect, rng)),
                right: Box::new(Partition::split(right_rect, rng)),
            }
        }
    }

    fn collect_rooms(&self, rooms: &mut Vec<Room>, rng: &mut ChaCha8Rng) {
        match self {
            Partition::Leaf(rect) => {
                let margin_x = rng.gen_range(ROOM_MARGIN..=(rect.width / 4).max(ROOM_MARGIN));
                let margin_y = rng.gen_range(ROOM_MARGIN..=(rect.height / 4).max(ROOM_MARGIN));
                let room_width = rect.width.saturating_sub(margin_x * 2).max(3);
                let room_height = rect.height.saturating_sub(margin_y * 2).max(3);
                let room_rect = Rect::new(
                    rect.x + margin_x,
                    rect.y + margin_y,
                    room_width,
                    room_height,
                );
                let id = rooms.len();
                rooms.push(Room::new(id, room_rect));
            }
            Partition::Split { left, right } => {
                left.collect_rooms(rooms, rng);
                right.collect_rooms(rooms, rng);
            }
        }
    }
}

pub fn generate(config: &DungeonConfig, rng: &mut ChaCha8Rng) -> Vec<Room> {
    let root = Rect::new(0, 0, config.width, config.height);
    let tree = Partition::split(root, rng);
    let mut rooms = Vec::new();
    tree.collect_rooms(&mut rooms, rng);

    let target = rng.gen_range(config.room_count.min..=config.room_count.max) as usize;
    if rooms.len() > target {
        rooms.truncate(target);
        // Re-assign IDs after truncation
        for (i, room) in rooms.iter_mut().enumerate() {
            room.id = i;
        }
    }

    rooms
}

use rand::seq::SliceRandom;
use rand::Rng;
use rand_chacha::ChaCha8Rng;

use crate::config::{Difficulty, Theme};
use crate::data::creature::Creature;
use crate::data::event::{AppliesTo, Event};
use crate::generator::bsp::Room;
use crate::generator::corridor::Corridor;

pub fn seed(
    rooms: &mut Vec<Room>,
    corridors: &mut Vec<Corridor>,
    creatures: &[Creature],
    events: &[Event],
    difficulty: Difficulty,
    theme: Theme,
    rng: &mut ChaCha8Rng,
) {
    let base_events: Vec<&Event> = events
        .iter()
        .filter(|e| e.themes.contains(&theme) && e.difficulty.contains(&difficulty))
        .collect();

    let room_events: Vec<&Event> = base_events
        .iter()
        .copied()
        .filter(|e| matches!(e.applies_to, AppliesTo::Room | AppliesTo::All))
        .collect();

    let corridor_events: Vec<&Event> = base_events
        .iter()
        .copied()
        .filter(|e| matches!(e.applies_to, AppliesTo::Corridor | AppliesTo::All))
        .collect();

    let matching_creatures: Vec<&Creature> = creatures
        .iter()
        .filter(|c| c.themes.contains(&theme) && c.difficulty.contains(&difficulty))
        .collect();

    for room in rooms.iter_mut() {
        // ~60% chance of an event
        if rng.gen_bool(0.6) {
            if let Some(&event) = room_events.choose(rng) {
                room.assigned_event = Some(event.clone());
            }
        }

        // ~50% chance of creatures; 1–3 if assigned
        if rng.gen_bool(0.5) && !matching_creatures.is_empty() {
            let count = rng.gen_range(1..=3.min(matching_creatures.len()));
            for _ in 0..count {
                if let Some(&creature) = matching_creatures.choose(rng) {
                    room.assigned_creatures.push(creature.clone());
                }
            }
        }
    }

    for corridor in corridors.iter_mut() {
        // ~40% chance of an event in a corridor
        if rng.gen_bool(0.4) {
            if let Some(&event) = corridor_events.choose(rng) {
                corridor.assigned_event = Some(event.clone());
            }
        }
    }
}

pub fn unique_creatures(rooms: &[Room]) -> Vec<Creature> {
    let mut seen = std::collections::HashSet::new();
    let mut unique = Vec::new();
    for room in rooms {
        for creature in &room.assigned_creatures {
            if seen.insert(creature.name.clone()) {
                unique.push(creature.clone());
            }
        }
    }
    unique
}

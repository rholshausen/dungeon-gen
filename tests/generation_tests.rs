use dungeon_gen::config::{Difficulty, DungeonConfig, RoomCountRange, RoomShapeWeights};
use dungeon_gen::generator::bsp;
use dungeon_gen::generator::corridor;
use dungeon_gen::seed::make_rng;

#[test]
fn generates_rooms_within_count_range() {
    let cfg = DungeonConfig {
        width: 80,
        height: 50,
        room_count: RoomCountRange { min: 6, max: 14 },
        seed: Some(42),
        room_shapes: RoomShapeWeights::default(),
    };
    let (mut rng, _) = make_rng(cfg.seed);
    let rooms = bsp::generate(&cfg, &mut rng);
    assert!(!rooms.is_empty());
    assert!(rooms.len() <= 14);
}

#[test]
fn same_seed_produces_same_rooms() {
    let cfg = DungeonConfig {
        width: 80,
        height: 50,
        room_count: RoomCountRange { min: 6, max: 14 },
        seed: Some(12345),
        room_shapes: RoomShapeWeights::default(),
    };

    let (mut rng1, _) = make_rng(cfg.seed);
    let rooms1 = bsp::generate(&cfg, &mut rng1);

    let (mut rng2, _) = make_rng(cfg.seed);
    let rooms2 = bsp::generate(&cfg, &mut rng2);

    assert_eq!(rooms1.len(), rooms2.len());
    for (r1, r2) in rooms1.iter().zip(rooms2.iter()) {
        assert_eq!(r1.bounds.x, r2.bounds.x);
        assert_eq!(r1.bounds.y, r2.bounds.y);
        assert_eq!(r1.bounds.width, r2.bounds.width);
        assert_eq!(r1.bounds.height, r2.bounds.height);
    }
}

#[test]
fn corridors_connect_rooms() {
    let cfg = DungeonConfig {
        width: 80,
        height: 50,
        room_count: RoomCountRange { min: 4, max: 8 },
        seed: Some(99),
        room_shapes: RoomShapeWeights::default(),
    };
    let (mut rng, _) = make_rng(cfg.seed);
    let rooms = bsp::generate(&cfg, &mut rng);
    let corridors = corridor::stitch(&rooms, Difficulty::Medium, &mut rng);

    if rooms.len() >= 2 {
        assert!(!corridors.is_empty());
        // One corridor per adjacent pair
        assert_eq!(corridors.len(), rooms.len() - 1);
    }
}

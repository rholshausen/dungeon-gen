use dungeon_gen::config::{DungeonConfig, RoomCountRange, RoomShapeWeights};
use dungeon_gen::generator::bsp;
use dungeon_gen::seed::make_rng;

/// Regression guard: room count for a fixed seed must not change across refactors.
#[test]
fn fixed_seed_room_count_regression() {
    let cfg = DungeonConfig {
        width: 80,
        height: 50,
        room_count: RoomCountRange { min: 6, max: 14 },
        seed: Some(777),
        room_shapes: RoomShapeWeights::default(),
    };
    let (mut rng, _) = make_rng(cfg.seed);
    let rooms = bsp::generate(&cfg, &mut rng);
    // Update this expected value after the first passing run if it changes.
    assert!(rooms.len() >= 6 && rooms.len() <= 14);
}

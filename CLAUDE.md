# dungeon-gen — project context for Claude

Procedural tabletop dungeon generator. Produces a multi-page PDF: page 1 is the map (grid + rooms + corridors + doors + labels), subsequent pages are DM notes (room key, corridor events, creature stat blocks).

## Build & run

```
cargo build
cargo test
cargo run -- --seed 42                          # random with fixed seed
cargo run -- --config config.ron --seed 99      # custom config
```

Output is `dungeon-<seed>.pdf` unless `--output` is passed.

## Key design rules

- **Rust edition 2024** — `gen` is a reserved keyword; always use `gen_range`, `gen_bool`, etc.
- **printpdf 0.7** — `PdfLayerReference`; `use_text` inherits the current fill colour so always reset fill to black before any text call after drawing coloured shapes.
- **PDF coordinate system** — y increases upward from bottom-left corner. Grid y increases downward (screen convention). Conversions in `map.rs`: `pdf_y = page_h - MARGIN - grid_y * tile_size`.
- **No git remote** — this is a local hobby project.

## Module map

```
src/
  main.rs                   entry point: load config → generate → populate → render
  cli.rs                    clap CLI struct (--seed, --config, --output, --difficulty, --theme)
  config.rs                 Config/DungeonConfig/OutputConfig/Difficulty/Theme/RoomShapeWeights
  seed.rs                   make_rng(seed) -> (ChaCha8Rng, u64)
  data/
    creature.rs             Creature struct + RON loader
    event.rs                Event struct, AppliesTo enum, RON loader
  generator/
    bsp.rs                  BSP room generation → Vec<Room>
    corridor.rs             stitch(rooms, difficulty, rng) → Vec<Corridor>; Door/DoorKind
    populator.rs            seed(...) assigns events & creatures to rooms and corridors
  render/
    map.rs                  all PDF drawing (grid, rooms, corridors, doors, labels, legend)
    pdf.rs                  top-level render() that assembles pages
data/
  creatures/*.ron           Creature data files
  events/encounters.ron     room/all events
  events/traps.ron          room/all events
  events/corridors.ron      corridor-specific events (applies_to: Corridor)
tests/
  generation_tests.rs       room count, determinism, corridor connectivity
  snapshot_tests.rs         fixed-seed room count regression
```

## Core types

### `Room` (`generator/bsp.rs`)
```rust
pub struct Room {
    pub id: usize,
    pub bounds: Rect,           // grid tiles, y-down
    pub shape: RoomShape,       // Rectangle | Round | Hexagonal | Octagonal
    pub assigned_event: Option<Event>,
    pub assigned_creatures: Vec<Creature>,
}
pub struct Rect { pub x: u32, pub y: u32, pub width: u32, pub height: u32 }
```

### `Corridor` (`generator/corridor.rs`)
```rust
pub struct Corridor {
    pub id: usize,
    pub segments: Vec<Rect>,    // L-shaped: 2 segments (one may be degenerate width=0/height=0)
    pub assigned_event: Option<Event>,
    pub doors: Vec<Door>,
}
pub struct Door {
    pub kind: DoorKind,         // Wooden | Iron | Secret
    pub locked: bool,
    pub lock_difficulty: Option<String>,
    pub x: f32,                 // fractional grid-tile units (shape-aware, not bounding-box edge)
    pub y: f32,
    pub in_horizontal_corridor: bool,
}
```

### `Event` (`data/event.rs`)
```rust
pub struct Event {
    pub id: String, pub name: String, pub trigger: String,
    pub difficulty_rating: Option<String>, pub effect: String,
    pub themes: Vec<Theme>, pub difficulty: Vec<Difficulty>,
    #[serde(default)]
    pub applies_to: AppliesTo,  // Room | Corridor | All (default = All)
}
```

## Rendering details (`render/map.rs`)

Constants: `MAP_MARGIN_MM = 15.0`, `MAX_TILE_SIZE_MM = 5.0`

**Draw order:** grid → corridors (grey fill) → rooms (white fill + black stroke) → doors → room labels (circled numbers)

**Room shapes** — all use `n_gon_points(cx, cy, rx, ry, n, start_angle)`:
- Rectangle: 4-point rect
- Round: 32-gon, start 0
- Hexagonal (pointy-top): 6-gon, start `PI/2`
- Octagonal (flat-top): 8-gon, start `PI/8`

**Corridor segments** — drawn with half-tile shift in the narrow axis:
- Horizontal: shifted up by `half` in pdf-y, extended ±`half` in x
- Vertical: shifted left by `half` in pdf-x, extended ±`half` in y

**Door wall positions** — shape-aware (`corridor.rs`):
- Hex horizontal wall at `cx ± rx·cos(PI/6)` (≈0.866·rx, NOT full rx)
- Oct horizontal wall at `cx ± rx·cos(PI/8)` (≈0.924·rx)
- Hex vertical wall at `cy ± ry` (full ry; vertices at exact top/bottom)
- Oct vertical wall at `cy ± ry·cos(PI/8)`
- Rectangle/Round: full rx/ry

**Corridor label placement** (`corridor_label_position`):
- Picks the non-degenerate segment whose grid-space centre is *furthest* from all room centres (avoids labels landing inside a room that the corridor passes through).
- Mirrors the half-tile draw offset when converting to PDF mm coordinates.

**Room labels** — 24-gon circle, radius `max(1.5mm, half_text_width + 0.4mm)`, white fill + black stroke, then black bold text centred inside.

**Legend** (`draw_legend`) — bottom-right box, 74×40mm. After each coloured mini-symbol rectangle, fill is reset to black before the `use_text` call.

## PDF pages

1. **Map** — grid, rooms, corridors, doors, corridor-event labels (C1…), room labels (circled numbers), door legend, seed footer.
2. **Room Key** — one entry per room: header (bold), event trigger/difficulty/effect, creature list. Spills to additional page if full.
3. **Corridor Events** — appended to page 2 if space remains (y ≥ 40mm), otherwise gets its own page.
4. **Creature Stat Blocks** — one entry per unique creature: name, stat line, appearance, tactics, attacks.

## Potential next steps (from prior sessions)

- More room shapes or shape-specific events
- Multi-page room key pagination (currently just truncates at page bottom)
- Trap/puzzle room rendering hints on map
- Export to image formats (PNG/SVG) in addition to PDF
- More creature variety in `data/creatures/`

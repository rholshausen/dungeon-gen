use std::path::Path;

use anyhow::Result;
use printpdf::{BuiltinFont, IndirectFontRef, Mm, PdfDocument, PdfLayerReference};

use crate::config::{Difficulty, OutputConfig, PaperSize};
use crate::data::creature::Creature;
use crate::generator::bsp::Room;
use crate::generator::corridor::Corridor;
use crate::render::map;

/// y-coordinate below which a new entry must spill to the next page.
const PAGE_BOTTOM_MM: f32 = 15.0;

fn estimate_room_height(room: &Room) -> f32 {
    let mut h = 6.0;
    if room.assigned_contents.is_some() {
        h += 5.0;
    }
    if let Some(event) = &room.assigned_event {
        h += 5.0;
        if event.difficulty_rating.is_some() {
            h += 5.0;
        }
        h += 5.0;
    }
    if !room.assigned_creatures.is_empty() {
        h += 5.0;
    }
    h + 4.0
}

fn estimate_corridor_height(corridor: &Corridor) -> f32 {
    let mut h = 6.0;
    if let Some(event) = &corridor.assigned_event {
        h += 5.0;
        if event.difficulty_rating.is_some() {
            h += 5.0;
        }
        h += 5.0;
    }
    h + 4.0
}

fn estimate_creature_height(creature: &Creature) -> f32 {
    let mut h = 5.0;
    if !creature.stats.is_empty() {
        h += 5.0;
    }
    h += 5.0 + 5.0;
    h += 5.0 * creature.attacks.len() as f32;
    h + 3.0
}

fn paper_dims(size: PaperSize) -> (Mm, Mm) {
    match size {
        PaperSize::A4 => (Mm(210.0), Mm(297.0)),
        PaperSize::Letter => (Mm(215.9), Mm(279.4)),
    }
}

fn write_event_lines(layer: &PdfLayerReference, event: &crate::data::event::Event, y: &mut f32, font: &IndirectFontRef) {
    layer.use_text(
        format!("  Trigger: {}", event.trigger),
        9.0,
        Mm(10.0),
        Mm(*y),
        font,
    );
    *y -= 5.0;
    if let Some(ref rating) = event.difficulty_rating {
        layer.use_text(
            format!("  Difficulty: {rating}"),
            9.0,
            Mm(10.0),
            Mm(*y),
            font,
        );
        *y -= 5.0;
    }
    layer.use_text(
        format!("  Effect: {}", event.effect),
        9.0,
        Mm(10.0),
        Mm(*y),
        font,
    );
    *y -= 5.0;
}

pub fn render(
    rooms: &[Room],
    corridors: &[Corridor],
    creatures_used: &[Creature],
    config: &OutputConfig,
    difficulty: Difficulty,
    seed: u64,
    output: &Path,
    grid_w: u32,
    grid_h: u32,
) -> Result<()> {
    let (page_w, page_h) = paper_dims(config.paper_size);
    let tile_size = map::compute_tile_size(page_w.0, page_h.0, grid_w, grid_h);

    let (doc, page1, layer1) = PdfDocument::new("Dungeon", page_w, page_h, "Map");
    let font = doc.add_builtin_font(BuiltinFont::Courier)?;
    let bold_font = doc.add_builtin_font(BuiltinFont::CourierBold)?;

    // --- Page 1: Map ---
    let map_layer = doc.get_page(page1).get_layer(layer1);
    map::draw_map(&map_layer, rooms, corridors, page_w.0, page_h.0, grid_w, grid_h, tile_size);

    // Room number labels — white circle with centred bold number.
    map::draw_room_labels(&map_layer, &bold_font, rooms, page_h.0, tile_size);

    // Corridor event labels — only drawn for corridors that have an assigned event.
    const LABEL_PT: f32 = 7.0;
    const PT_TO_MM: f32 = 25.4 / 72.0;
    let char_w_mm = LABEL_PT * 0.6 * PT_TO_MM;
    let half_cap_mm = LABEL_PT * 0.7 * PT_TO_MM / 2.0;
    for corridor in corridors {
        if corridor.assigned_event.is_none() {
            continue;
        }
        let label = format!("C{}", corridor.id + 1);
        if let Some((lx, ly)) = map::corridor_label_position(corridor, rooms, page_h.0, tile_size) {
            let cx = Mm(lx.0 - label.len() as f32 * char_w_mm / 2.0);
            let cy = Mm(ly.0 - half_cap_mm);
            map_layer.use_text(label, LABEL_PT, cx, cy, &font);
        }
    }

    // Door-type legend (bottom-right corner of map page)
    map::draw_legend(&map_layer, &font, &bold_font, page_w.0, difficulty);

    // Footer: seed
    map_layer.use_text(
        format!("Seed: {seed}"),
        8.0,
        Mm(10.0),
        Mm(8.0),
        &font,
    );

    // Helper: add a new section page with header + seed footer.
    let new_section = |title: &str, header_text: &str, header_size: f32| -> (PdfLayerReference, f32) {
        let (p, l) = doc.add_page(page_w, page_h, title);
        let layer = doc.get_page(p).get_layer(l);
        layer.use_text(header_text, header_size, Mm(10.0), Mm(page_h.0 - 20.0), &bold_font);
        layer.use_text(format!("Seed: {seed}"), 8.0, Mm(10.0), Mm(8.0), &font);
        (layer, page_h.0 - 35.0)
    };

    // --- Pages 2..N: Room key ---
    if config.include_dm_notes {
        let (mut notes_layer, mut y) = new_section("Room Key", "Room Key", 16.0);

        for room in rooms {
            let needed = estimate_room_height(room);
            if y - needed < PAGE_BOTTOM_MM {
                let (l, ny) = new_section("Room Key (cont.)", "Room Key (cont.)", 16.0);
                notes_layer = l;
                y = ny;
            }

            let header = match (&room.assigned_contents, &room.assigned_event) {
                (Some(c), _) => format!("Room {} — {}", room.id + 1, c.name),
                (None, Some(e)) => format!("Room {} — {}", room.id + 1, e.name),
                (None, None) => format!("Room {} — Empty", room.id + 1),
            };
            notes_layer.use_text(&header, 10.0, Mm(10.0), Mm(y), &bold_font);
            y -= 6.0;

            if let Some(contents) = &room.assigned_contents {
                notes_layer.use_text(
                    format!("  {}", contents.description),
                    9.0,
                    Mm(10.0),
                    Mm(y),
                    &font,
                );
                y -= 5.0;
            }

            if let Some(event) = &room.assigned_event {
                write_event_lines(&notes_layer, event, &mut y, &font);
            }

            if !room.assigned_creatures.is_empty() {
                let names: Vec<&str> = room.assigned_creatures.iter().map(|c| c.name.as_str()).collect();
                notes_layer.use_text(
                    format!("  Creatures: {}", names.join(", ")),
                    9.0,
                    Mm(10.0),
                    Mm(y),
                    &font,
                );
                y -= 5.0;
            }

            y -= 4.0;
        }

        // Corridor events — share the current page if header + first entry fit, else
        // start a fresh page. Subsequent entries spill onto continuation pages.
        let corridors_with_events: Vec<&Corridor> =
            corridors.iter().filter(|c| c.assigned_event.is_some()).collect();

        if !corridors_with_events.is_empty() {
            let first_block = 12.0 + estimate_corridor_height(corridors_with_events[0]);
            if y - first_block < PAGE_BOTTOM_MM {
                let (l, ny) = new_section("Corridor Events", "Corridor Events", 12.0);
                notes_layer = l;
                y = ny;
            } else {
                y -= 4.0;
                notes_layer.use_text("Corridor Events", 12.0, Mm(10.0), Mm(y), &bold_font);
                y -= 8.0;
            }

            for corridor in corridors_with_events {
                let needed = estimate_corridor_height(corridor);
                if y - needed < PAGE_BOTTOM_MM {
                    let (l, ny) = new_section(
                        "Corridor Events (cont.)",
                        "Corridor Events (cont.)",
                        12.0,
                    );
                    notes_layer = l;
                    y = ny;
                }
                let event = corridor.assigned_event.as_ref().unwrap();
                let header = format!("Corridor {} — {}", corridor.id + 1, event.name);
                notes_layer.use_text(&header, 10.0, Mm(10.0), Mm(y), &bold_font);
                y -= 6.0;
                write_event_lines(&notes_layer, event, &mut y, &font);
                y -= 4.0;
            }
        }
    }

    // --- Stat block pages ---
    if !creatures_used.is_empty() {
        let (mut stat_layer, mut y) =
            new_section("Stat Blocks", "Creature Stat Blocks", 16.0);

        for creature in creatures_used {
            let needed = estimate_creature_height(creature);
            if y - needed < PAGE_BOTTOM_MM {
                let (l, ny) = new_section(
                    "Stat Blocks (cont.)",
                    "Creature Stat Blocks (cont.)",
                    16.0,
                );
                stat_layer = l;
                y = ny;
            }

            stat_layer.use_text(&creature.name, 10.0, Mm(10.0), Mm(y), &bold_font);
            y -= 5.0;

            // Render whatever stats the RON file defined — label: value pairs
            let stat_line = creature
                .stats
                .iter()
                .map(|s| format!("{}: {}", s.label, s.value))
                .collect::<Vec<_>>()
                .join("  |  ");
            if !stat_line.is_empty() {
                stat_layer.use_text(&stat_line, 9.0, Mm(12.0), Mm(y), &font);
                y -= 5.0;
            }

            stat_layer.use_text(&creature.description.appearance, 9.0, Mm(12.0), Mm(y), &font);
            y -= 5.0;
            stat_layer.use_text(&creature.description.tactics, 9.0, Mm(12.0), Mm(y), &font);
            y -= 5.0;

            for attack in &creature.attacks {
                let attack_stats = attack
                    .stats
                    .iter()
                    .map(|s| format!("{}: {}", s.label, s.value))
                    .collect::<Vec<_>>()
                    .join("  |  ");
                stat_layer.use_text(
                    format!("  {}  {}", attack.name, attack_stats),
                    9.0,
                    Mm(10.0),
                    Mm(y),
                    &font,
                );
                y -= 5.0;
            }

            y -= 3.0;
        }
    }

    doc.save(&mut std::io::BufWriter::new(std::fs::File::create(output)?))?;
    Ok(())
}

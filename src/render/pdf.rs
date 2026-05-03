use std::path::Path;

use anyhow::Result;
use printpdf::{BuiltinFont, IndirectFontRef, Mm, PdfDocument, PdfLayerReference};

use crate::config::{OutputConfig, PaperSize};
use crate::data::creature::Creature;
use crate::generator::bsp::Room;
use crate::generator::corridor::Corridor;
use crate::render::map;

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
    map::draw_map(&map_layer, rooms, corridors, page_h.0, tile_size);

    // Room number labels — centred inside each room.
    // use_text anchors at the left edge of the text baseline, so we correct for both:
    //   x: subtract half the text width (Courier char width = 0.6em)
    //   y: subtract half the cap height (≈ 0.7em) so glyphs straddle the room centre
    const LABEL_PT: f32 = 7.0;
    const PT_TO_MM: f32 = 25.4 / 72.0;
    let char_w_mm = LABEL_PT * 0.6 * PT_TO_MM;
    let half_cap_mm = LABEL_PT * 0.7 * PT_TO_MM / 2.0;

    for room in rooms {
        let label = format!("{}", room.id + 1);
        let (lx, ly) = map::room_label_position(room, page_h.0, tile_size);
        let cx = Mm(lx.0 - label.len() as f32 * char_w_mm / 2.0);
        let cy = Mm(ly.0 - half_cap_mm);
        map_layer.use_text(label, LABEL_PT, cx, cy, &bold_font);
    }

    // Corridor event labels — only drawn for corridors that have an assigned event.
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

    // Footer: seed
    map_layer.use_text(
        format!("Seed: {seed}"),
        8.0,
        Mm(10.0),
        Mm(8.0),
        &font,
    );

    // --- Pages 2..N: Room key ---
    if config.include_dm_notes {
        let (page2, layer2) = doc.add_page(page_w, page_h, "Room Key");
        let notes_layer = doc.get_page(page2).get_layer(layer2);

        notes_layer.use_text("Room Key", 16.0, Mm(10.0), Mm(page_h.0 - 20.0), &bold_font);

        let mut y = page_h.0 - 35.0;
        for room in rooms {
            if y < 20.0 {
                // Simple overflow: just stop (could add new pages in a full implementation)
                break;
            }

            let header = if let Some(event) = &room.assigned_event {
                format!("Room {} — {}", room.id + 1, event.name)
            } else {
                format!("Room {}", room.id + 1)
            };
            notes_layer.use_text(&header, 10.0, Mm(10.0), Mm(y), &bold_font);
            y -= 6.0;

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

        // Corridor events — spill to a new page if there isn't room for at least the header
        // plus one entry (~40mm).
        let corridors_with_events: Vec<&Corridor> =
            corridors.iter().filter(|c| c.assigned_event.is_some()).collect();

        if !corridors_with_events.is_empty() {
            let (corr_layer, mut corr_y) = if y >= 40.0 {
                y -= 4.0;
                (doc.get_page(page2).get_layer(layer2), y)
            } else {
                let (page_c, layer_c) = doc.add_page(page_w, page_h, "Corridor Events");
                let layer = doc.get_page(page_c).get_layer(layer_c);
                layer.use_text(format!("Seed: {seed}"), 8.0, Mm(10.0), Mm(8.0), &font);
                (layer, page_h.0 - 20.0)
            };

            corr_layer.use_text("Corridor Events", 12.0, Mm(10.0), Mm(corr_y), &bold_font);
            corr_y -= 8.0;

            for corridor in corridors_with_events {
                if corr_y < 20.0 {
                    break;
                }
                let event = corridor.assigned_event.as_ref().unwrap();
                let header = format!("Corridor {} — {}", corridor.id + 1, event.name);
                corr_layer.use_text(&header, 10.0, Mm(10.0), Mm(corr_y), &bold_font);
                corr_y -= 6.0;
                write_event_lines(&corr_layer, event, &mut corr_y, &font);
                corr_y -= 4.0;
            }
        }

        // Footer on room key page
        notes_layer.use_text(format!("Seed: {seed}"), 8.0, Mm(10.0), Mm(8.0), &font);
    }

    // --- Stat block pages ---
    if !creatures_used.is_empty() {
        let (stat_page, stat_layer_id) = doc.add_page(page_w, page_h, "Stat Blocks");
        let stat_layer = doc.get_page(stat_page).get_layer(stat_layer_id);

        stat_layer.use_text("Creature Stat Blocks", 16.0, Mm(10.0), Mm(page_h.0 - 20.0), &bold_font);

        let mut y = page_h.0 - 35.0;
        for creature in creatures_used {
            if y < 20.0 {
                break;
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

        stat_layer.use_text(format!("Seed: {seed}"), 8.0, Mm(10.0), Mm(8.0), &font);
    }

    doc.save(&mut std::io::BufWriter::new(std::fs::File::create(output)?))?;
    Ok(())
}

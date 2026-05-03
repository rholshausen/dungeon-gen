use std::path::Path;

use anyhow::Result;
use printpdf::{BuiltinFont, Mm, PdfDocument};

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

pub fn render(
    rooms: &[Room],
    corridors: &[Corridor],
    creatures_used: &[Creature],
    config: &OutputConfig,
    seed: u64,
    output: &Path,
) -> Result<()> {
    let (page_w, page_h) = paper_dims(config.paper_size);

    let (doc, page1, layer1) = PdfDocument::new("Dungeon", page_w, page_h, "Map");
    let font = doc.add_builtin_font(BuiltinFont::Courier)?;
    let bold_font = doc.add_builtin_font(BuiltinFont::CourierBold)?;

    // --- Page 1: Map ---
    let map_layer = doc.get_page(page1).get_layer(layer1);
    map::draw_map(&map_layer, rooms, corridors, page_h.0);

    // Room number labels
    for room in rooms {
        let (lx, ly) = map::room_label_position(room, page_h.0);
        map_layer.use_text(format!("{}", room.id + 1), 7.0, lx, ly, &bold_font);
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

            notes_layer.use_text(
                format!("Room {} — no special event", room.id + 1),
                10.0,
                Mm(10.0),
                Mm(y),
                &bold_font,
            );
            y -= 6.0;

            if let Some(event) = &room.assigned_event {
                notes_layer.use_text(
                    format!("  Event: {}", event.name),
                    9.0,
                    Mm(10.0),
                    Mm(y),
                    &font,
                );
                y -= 5.0;
                notes_layer.use_text(
                    format!("  Trigger: {}", event.trigger),
                    9.0,
                    Mm(10.0),
                    Mm(y),
                    &font,
                );
                y -= 5.0;
                if let Some(dc) = event.dc {
                    notes_layer.use_text(
                        format!("  DC: {dc}"),
                        9.0,
                        Mm(10.0),
                        Mm(y),
                        &font,
                    );
                    y -= 5.0;
                }
                notes_layer.use_text(
                    format!("  Effect: {}", event.effect),
                    9.0,
                    Mm(10.0),
                    Mm(y),
                    &font,
                );
                y -= 5.0;
            } else {
                // Overwrite the "no special event" line header
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

        // Footer
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

            stat_layer.use_text(
                format!(
                    "{} — CR {} | HP: {} (avg {:.0}) | AC: {}",
                    creature.name,
                    creature.cr,
                    creature.hp.notation(),
                    creature.hp.average(),
                    creature.ac,
                ),
                10.0,
                Mm(10.0),
                Mm(y),
                &bold_font,
            );
            y -= 6.0;

            stat_layer.use_text(&creature.description.appearance, 9.0, Mm(12.0), Mm(y), &font);
            y -= 5.0;
            stat_layer.use_text(&creature.description.tactics, 9.0, Mm(12.0), Mm(y), &font);
            y -= 5.0;

            for attack in &creature.attacks {
                stat_layer.use_text(
                    format!(
                        "  {} +{} to hit | {} {:?}",
                        attack.name, attack.bonus, attack.damage.0.notation(), attack.damage.1
                    ),
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

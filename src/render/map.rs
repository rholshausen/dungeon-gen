use std::f32::consts::PI;

use printpdf::path::{PaintMode, WindingOrder};
use printpdf::{Mm, PdfLayerReference, Point, Polygon, Rgb};

use printpdf::IndirectFontRef;

use crate::data::room_contents::DrawCommand;
use crate::generator::bsp::{Rect, Room, RoomShape};
use crate::generator::corridor::{Corridor, Door, DoorKind, lock_picking_difficulty};
use crate::config::Difficulty;

const MAP_MARGIN_MM: f32 = 15.0;
const MAX_TILE_SIZE_MM: f32 = 5.0;

/// Number of polygon vertices used to approximate a round room.
const ELLIPSE_STEPS: usize = 32;

pub fn compute_tile_size(page_w_mm: f32, page_h_mm: f32, grid_w: u32, grid_h: u32) -> f32 {
    let available_w = page_w_mm - 2.0 * MAP_MARGIN_MM;
    let available_h = page_h_mm - 2.0 * MAP_MARGIN_MM;
    let tile_from_w = available_w / grid_w as f32;
    let tile_from_h = available_h / grid_h as f32;
    tile_from_w.min(tile_from_h).min(MAX_TILE_SIZE_MM)
}

fn tile_to_mm(tile: u32, tile_size: f32) -> Mm {
    Mm(tile as f32 * tile_size)
}

/// Returns the bottom-left corner of a rect in PDF coordinates (Y increases upward).
fn rect_origin(rect: &Rect, page_height_mm: f32, tile_size: f32) -> (Mm, Mm) {
    let x = MAP_MARGIN_MM + rect.x as f32 * tile_size;
    let y = page_height_mm - MAP_MARGIN_MM - (rect.y as f32 + rect.height as f32) * tile_size;
    (Mm(x), Mm(y))
}

fn rect_points(ox: Mm, oy: Mm, w: Mm, h: Mm) -> Vec<(Point, bool)> {
    vec![
        (Point::new(ox, oy), false),
        (Point::new(Mm(ox.0 + w.0), oy), false),
        (Point::new(Mm(ox.0 + w.0), Mm(oy.0 + h.0)), false),
        (Point::new(ox, Mm(oy.0 + h.0)), false),
    ]
}

/// Regular n-gon inscribed in an ellipse with semi-axes rx × ry, centred at (cx, cy).
/// `start_angle` rotates the first vertex (radians, standard maths convention: 0 = right).
fn n_gon_points(cx: f32, cy: f32, rx: f32, ry: f32, n: usize, start_angle: f32) -> Vec<(Point, bool)> {
    (0..n)
        .map(|i| {
            let angle = start_angle + i as f32 * 2.0 * PI / n as f32;
            (Point::new(Mm(cx + rx * angle.cos()), Mm(cy + ry * angle.sin())), false)
        })
        .collect()
}

fn add_polygon(layer: &PdfLayerReference, points: Vec<(Point, bool)>, mode: PaintMode) {
    layer.add_polygon(Polygon {
        rings: vec![points],
        mode,
        winding_order: WindingOrder::NonZero,
    });
}

/// Each corridor segment is centred on its path (half-tile shift in the narrow axis) and
/// extended by half a tile at both ends in the path axis. The room-end extension hides under
/// the room's white fill; the junction-end extension reaches exactly the outer edge of the
/// crossing segment, closing the corner without creating a cross arm.
fn draw_corridor_segment(layer: &PdfLayerReference, rect: &Rect, page_height_mm: f32, tile_size: f32) {
    let (ox, oy) = rect_origin(rect, page_height_mm, tile_size);
    let w = tile_to_mm(rect.width, tile_size);
    let h = tile_to_mm(rect.height, tile_size);
    let half = tile_size / 2.0;

    // For horizontal segments: centre in y, extend both x ends by half a tile.
    // For vertical segments:   centre in x, extend both y ends by half a tile.
    let (ox, oy, w, h) = if rect.height <= rect.width {
        (Mm(ox.0 - half), Mm(oy.0 + half), Mm(w.0 + tile_size), h)
    } else {
        (Mm(ox.0 - half), Mm(oy.0 - half), w, Mm(h.0 + tile_size))
    };

    add_polygon(layer, rect_points(ox, oy, w, h), PaintMode::Fill);
}

fn draw_room(layer: &PdfLayerReference, room: &Room, page_height_mm: f32, tile_size: f32) {
    let (ox, oy) = rect_origin(&room.bounds, page_height_mm, tile_size);
    let w = tile_to_mm(room.bounds.width, tile_size);
    let h = tile_to_mm(room.bounds.height, tile_size);

    // Centre of the room in PDF mm coordinates
    let cx = ox.0 + w.0 / 2.0;
    let cy = oy.0 + h.0 / 2.0;
    let rx = w.0 / 2.0;
    let ry = h.0 / 2.0;

    let points = match room.shape {
        RoomShape::Rectangle => rect_points(ox, oy, w, h),
        // Round: 32-vertex ellipse inscribed in the bounding box
        RoomShape::Round => n_gon_points(cx, cy, rx, ry, ELLIPSE_STEPS, 0.0),
        // Hexagonal: pointy-top — first vertex at 90° (top centre)
        RoomShape::Hexagonal => n_gon_points(cx, cy, rx, ry, 6, PI / 2.0),
        // Octagonal: flat-top — vertices at 22.5° intervals starting at 22.5°,
        // giving horizontal flat edges at top/bottom and vertical flat edges at left/right
        RoomShape::Octagonal => n_gon_points(cx, cy, rx, ry, 8, PI / 8.0),
    };

    add_polygon(layer, points, PaintMode::FillStroke);
}

/// Draw a single door straddling the wall between a room and its corridor.
///
/// Door rectangle dimensions:
///   thickness (narrow axis) = 30 % of tile_size
///   span      (wide axis)   = 100 % of tile_size (full corridor width)
///
/// Coordinate conventions (matching draw_corridor_segment):
///   horizontal corridor centre y = page_h − MARGIN − door.y × tile_size
///   vertical   corridor centre x = MARGIN + door.x × tile_size
fn draw_door(layer: &PdfLayerReference, door: &Door, page_height_mm: f32, tile_size: f32) {
    let thick = tile_size * 0.30;
    let span = tile_size;

    let (ox, oy, dw, dh) = if door.in_horizontal_corridor {
        let cx = MAP_MARGIN_MM + door.x * tile_size;
        let cy = page_height_mm - MAP_MARGIN_MM - door.y * tile_size;
        (cx - thick / 2.0, cy - span / 2.0, thick, span)
    } else {
        let cx = MAP_MARGIN_MM + door.x * tile_size;
        let cy = page_height_mm - MAP_MARGIN_MM - door.y * tile_size;
        (cx - span / 2.0, cy - thick / 2.0, span, thick)
    };

    match door.kind {
        DoorKind::Wooden => {
            layer.set_fill_color(printpdf::Color::Rgb(Rgb::new(1.0, 1.0, 1.0, None)));
            layer.set_outline_color(printpdf::Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)));
            layer.set_outline_thickness(0.5);
        }
        DoorKind::Iron => {
            layer.set_fill_color(printpdf::Color::Rgb(Rgb::new(0.45, 0.45, 0.45, None)));
            layer.set_outline_color(printpdf::Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)));
            layer.set_outline_thickness(0.8);
        }
        DoorKind::Secret => {
            layer.set_fill_color(printpdf::Color::Rgb(Rgb::new(0.85, 0.85, 0.85, None)));
            layer.set_outline_color(printpdf::Color::Rgb(Rgb::new(0.55, 0.55, 0.55, None)));
            layer.set_outline_thickness(0.3);
        }
    }

    add_polygon(
        layer,
        rect_points(Mm(ox), Mm(oy), Mm(dw), Mm(dh)),
        PaintMode::FillStroke,
    );

    // Locked indicator: small filled square centred on the door.
    if door.locked {
        let dot = thick * 0.5;
        let dx = ox + dw / 2.0 - dot / 2.0;
        let dy = oy + dh / 2.0 - dot / 2.0;
        layer.set_fill_color(printpdf::Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)));
        layer.set_outline_thickness(0.0);
        add_polygon(
            layer,
            rect_points(Mm(dx), Mm(dy), Mm(dot), Mm(dot)),
            PaintMode::Fill,
        );
    }
}

fn draw_grid(
    layer: &PdfLayerReference,
    page_width_mm: f32,
    page_height_mm: f32,
    grid_w: u32,
    grid_h: u32,
    tile_size: f32,
) {
    layer.set_outline_color(printpdf::Color::Rgb(Rgb::new(0.82, 0.82, 0.82, None)));
    layer.set_outline_thickness(0.15);

    let left = MAP_MARGIN_MM;
    let right = MAP_MARGIN_MM + grid_w as f32 * tile_size;
    let top = page_height_mm - MAP_MARGIN_MM;
    let bottom = top - grid_h as f32 * tile_size;

    let _ = page_width_mm; // used only to keep the signature symmetric with page_height_mm

    for col in 0..=grid_w {
        let x = MAP_MARGIN_MM + col as f32 * tile_size;
        add_polygon(
            layer,
            vec![(Point::new(Mm(x), Mm(bottom)), false), (Point::new(Mm(x), Mm(top)), false)],
            PaintMode::Stroke,
        );
    }
    for row in 0..=grid_h {
        let y = top - row as f32 * tile_size;
        add_polygon(
            layer,
            vec![(Point::new(Mm(left), Mm(y)), false), (Point::new(Mm(right), Mm(y)), false)],
            PaintMode::Stroke,
        );
    }
}

pub fn draw_map(
    layer: &PdfLayerReference,
    rooms: &[Room],
    corridors: &[Corridor],
    page_width_mm: f32,
    page_height_mm: f32,
    grid_w: u32,
    grid_h: u32,
    tile_size: f32,
) {
    draw_grid(layer, page_width_mm, page_height_mm, grid_w, grid_h, tile_size);

    layer.set_fill_color(printpdf::Color::Rgb(Rgb::new(0.6, 0.6, 0.6, None)));
    layer.set_outline_thickness(0.0);

    for corridor in corridors {
        for segment in &corridor.segments {
            // Skip degenerate segments (can arise when two room centres share a coordinate)
            if segment.width == 0 || segment.height == 0 {
                continue;
            }
            draw_corridor_segment(layer, segment, page_height_mm, tile_size);
        }
    }

    layer.set_fill_color(printpdf::Color::Rgb(Rgb::new(1.0, 1.0, 1.0, None)));
    layer.set_outline_color(printpdf::Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)));
    layer.set_outline_thickness(1.0);

    for room in rooms {
        draw_room(layer, room, page_height_mm, tile_size);
    }

    // Doors drawn last so they appear on top of room fills, straddling the wall.
    for corridor in corridors {
        for door in &corridor.doors {
            draw_door(layer, door, page_height_mm, tile_size);
        }
    }

    layer.set_fill_color(printpdf::Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)));
}

/// Draw each room's icon (if any) centred on the room, scaled to fit inside the room shape.
///
/// Icons are defined as `Vec<DrawCommand>` in the RON data files using normalised (−1..1)
/// coordinates. The renderer transforms them to PDF space without any per-type Rust code.
/// Drawn after room fills so the icon sits on the white interior, and before room-number
/// labels so the circled number appears on top.
pub fn draw_room_icons(
    layer: &PdfLayerReference,
    rooms: &[Room],
    page_height_mm: f32,
    tile_size: f32,
) {
    let icon_colour = printpdf::Color::Rgb(Rgb::new(0.55, 0.55, 0.55, None));

    for room in rooms {
        let Some(contents) = &room.assigned_contents else { continue };
        if contents.icon.is_empty() { continue }

        let (cx, cy) = room_label_position(room, page_height_mm, tile_size);
        let w_mm = room.bounds.width as f32 * tile_size;
        let h_mm = room.bounds.height as f32 * tile_size;
        let scale = w_mm.min(h_mm) * 0.3;

        layer.set_outline_color(icon_colour.clone());
        layer.set_fill_color(icon_colour.clone());
        layer.set_outline_thickness(0.5);

        for cmd in &contents.icon {
            match cmd {
                DrawCommand::Line { x1, y1, x2, y2 } => {
                    let p1 = Point::new(Mm(cx.0 + x1 * scale), Mm(cy.0 + y1 * scale));
                    let p2 = Point::new(Mm(cx.0 + x2 * scale), Mm(cy.0 + y2 * scale));
                    add_polygon(layer, vec![(p1, false), (p2, false)], PaintMode::Stroke);
                }
                DrawCommand::Rect { x, y, w, h, filled } => {
                    let ox = Mm(cx.0 + x * scale);
                    let oy = Mm(cy.0 + y * scale);
                    let rw = Mm(w * scale);
                    let rh = Mm(h * scale);
                    let mode = if *filled { PaintMode::FillStroke } else { PaintMode::Stroke };
                    add_polygon(layer, rect_points(ox, oy, rw, rh), mode);
                }
                DrawCommand::NGon { cx: ncx, cy: ncy, r, n, start_angle, filled } => {
                    let pcx = cx.0 + ncx * scale;
                    let pcy = cy.0 + ncy * scale;
                    let pr = r * scale;
                    let mode = if *filled { PaintMode::FillStroke } else { PaintMode::Stroke };
                    add_polygon(
                        layer,
                        n_gon_points(pcx, pcy, pr, pr, *n as usize, *start_angle),
                        mode,
                    );
                }
            }
        }
    }

    layer.set_fill_color(printpdf::Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)));
    layer.set_outline_color(printpdf::Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)));
}

pub fn room_label_position(room: &Room, page_height_mm: f32, tile_size: f32) -> (Mm, Mm) {
    let cx = MAP_MARGIN_MM + (room.bounds.x as f32 + room.bounds.width as f32 / 2.0) * tile_size;
    let cy = page_height_mm
        - MAP_MARGIN_MM
        - (room.bounds.y as f32 + room.bounds.height as f32 / 2.0) * tile_size;
    (Mm(cx), Mm(cy))
}

/// Draw a white-filled circle with the room number centred inside for each room.
///
/// Circles are drawn after `draw_map` so they sit on top of room fills and doors.
/// The radius grows slightly for multi-digit labels to avoid clipping.
pub fn draw_room_labels(
    layer: &PdfLayerReference,
    bold_font: &IndirectFontRef,
    rooms: &[Room],
    page_height_mm: f32,
    tile_size: f32,
) {
    const LABEL_PT: f32 = 7.0;
    const PT_TO_MM: f32 = 25.4 / 72.0;
    let char_w_mm = LABEL_PT * 0.6 * PT_TO_MM;
    let half_cap_mm = LABEL_PT * 0.7 * PT_TO_MM / 2.0;

    for room in rooms {
        let label = format!("{}", room.id + 1);
        let (cx, cy) = room_label_position(room, page_height_mm, tile_size);

        // Radius: half the text width plus a small margin, minimum 1.5 mm.
        let r = (label.len() as f32 * char_w_mm / 2.0 + 0.4).max(1.5_f32);

        // White filled circle with black outline
        layer.set_fill_color(printpdf::Color::Rgb(Rgb::new(1.0, 1.0, 1.0, None)));
        layer.set_outline_color(printpdf::Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)));
        layer.set_outline_thickness(0.5);
        add_polygon(layer, n_gon_points(cx.0, cy.0, r, r, 24, 0.0), PaintMode::FillStroke);

        // Centred bold label — reset fill to black so use_text inherits it
        layer.set_fill_color(printpdf::Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)));
        let tx = Mm(cx.0 - label.len() as f32 * char_w_mm / 2.0);
        let ty = Mm(cy.0 - half_cap_mm);
        layer.use_text(&label, LABEL_PT, tx, ty, bold_font);
    }
}

/// Returns the visual centre of the corridor segment whose midpoint is furthest from every room
/// centre, in PDF coordinates. This keeps the label in open corridor space and avoids landing
/// inside a room that the corridor passes through.
/// Mirrors the half-tile offset that `draw_corridor_segment` applies.
/// Returns `None` if the corridor has no usable segments (all degenerate).
pub fn corridor_label_position(corridor: &Corridor, rooms: &[Room], page_height_mm: f32, tile_size: f32) -> Option<(Mm, Mm)> {
    let seg = corridor
        .segments
        .iter()
        .filter(|s| s.width > 0 && s.height > 0)
        .max_by_key(|s| {
            // Grid-space centre of this segment.
            let scx = s.x as f32 + s.width as f32 / 2.0;
            let scy = s.y as f32 + s.height as f32 / 2.0;
            // Minimum squared distance from the segment centre to the nearest edge of any room.
            // Using bounding-box distance (not centre distance) so labels near room walls score
            // lower than labels in genuinely open corridor space.
            let min_dist_sq = rooms
                .iter()
                .map(|r| {
                    let rx = r.bounds.x as f32;
                    let ry = r.bounds.y as f32;
                    let rr = rx + r.bounds.width as f32;
                    let rb = ry + r.bounds.height as f32;
                    let nx = scx.clamp(rx, rr);
                    let ny = scy.clamp(ry, rb);
                    let dx = scx - nx;
                    let dy = scy - ny;
                    dx * dx + dy * dy
                })
                .fold(f32::INFINITY, |a, b| a.min(b));
            (min_dist_sq * 100.0) as u32
        })?;

    let half = tile_size / 2.0;
    let (ox, oy) = rect_origin(seg, page_height_mm, tile_size);
    let w = tile_to_mm(seg.width, tile_size);
    let h = tile_to_mm(seg.height, tile_size);

    let (cx, cy) = if seg.height <= seg.width {
        // Horizontal: draw_corridor_segment shifts the box up by half a tile in PDF y.
        (ox.0 + w.0 / 2.0, oy.0 + half + h.0 / 2.0)
    } else {
        // Vertical: draw_corridor_segment shifts the box left by half a tile in PDF x.
        (ox.0 - half + w.0 / 2.0, oy.0 + h.0 / 2.0)
    };

    Some((Mm(cx), Mm(cy)))
}

/// Draw a door-type legend box in the bottom-right of the page.
///
/// The box has a white fill so it sits cleanly over map content. Lock-picking difficulties are
/// derived from the dungeon difficulty and shown for wooden and iron locked doors.
pub fn draw_legend(
    layer: &PdfLayerReference,
    font: &IndirectFontRef,
    bold_font: &IndirectFontRef,
    page_width_mm: f32,
    difficulty: Difficulty,
) {
    // Box geometry
    let bx = page_width_mm - 82.0;
    let by = 6.0_f32;
    let bw = 74.0_f32;
    let bh = 40.0_f32;

    // White background with thin border
    layer.set_fill_color(printpdf::Color::Rgb(Rgb::new(1.0, 1.0, 1.0, None)));
    layer.set_outline_color(printpdf::Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)));
    layer.set_outline_thickness(0.5);
    add_polygon(layer, rect_points(Mm(bx), Mm(by), Mm(bw), Mm(bh)), PaintMode::FillStroke);

    layer.use_text("Door Legend", 8.0, Mm(bx + 3.0), Mm(by + bh - 6.5), bold_font);

    // Mini symbol dimensions (horizontal door, viewed from above)
    let sym_x = bx + 4.0;
    let sym_w = 6.0_f32;
    let sym_h = 1.8_f32;
    let lbl_x = bx + 13.5;

    // Row y-centres inside the box (top to bottom)
    let row_y = [by + 30.0, by + 24.0, by + 18.0, by + 12.0, by + 7.0];

    // Helper closures ---------------------------------------------------

    // Draw a mini symbol rectangle at row_y[i], then reset fill to black so the
    // following use_text call inherits black rather than the symbol's fill colour.
    let mini = |ry: f32, fill: (f32, f32, f32), stroke: (f32, f32, f32), line_w: f32| {
        layer.set_fill_color(printpdf::Color::Rgb(Rgb::new(fill.0, fill.1, fill.2, None)));
        layer.set_outline_color(printpdf::Color::Rgb(Rgb::new(stroke.0, stroke.1, stroke.2, None)));
        layer.set_outline_thickness(line_w);
        add_polygon(
            layer,
            rect_points(Mm(sym_x), Mm(ry - sym_h / 2.0), Mm(sym_w), Mm(sym_h)),
            PaintMode::FillStroke,
        );
        layer.set_fill_color(printpdf::Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)));
    };

    // Row 0 — Wooden door
    mini(row_y[0], (1.0, 1.0, 1.0), (0.0, 0.0, 0.0), 0.5_f32);
    layer.use_text("Wooden door", 7.0, Mm(lbl_x), Mm(row_y[0] - 1.3), font);

    // Row 1 — Iron door
    mini(row_y[1], (0.45, 0.45, 0.45), (0.0, 0.0, 0.0), 0.8_f32);
    layer.use_text("Iron door (reinforced)", 7.0, Mm(lbl_x), Mm(row_y[1] - 1.3), font);

    // Row 2 — Secret door
    mini(row_y[2], (0.85, 0.85, 0.85), (0.55, 0.55, 0.55), 0.3_f32);
    layer.use_text("Secret door", 7.0, Mm(lbl_x), Mm(row_y[2] - 1.3), font);

    // Row 3 — Locked indicator (filled dot)
    let dot = 2.0_f32;
    layer.set_fill_color(printpdf::Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)));
    layer.set_outline_thickness(0.0);
    add_polygon(
        layer,
        rect_points(Mm(sym_x + sym_w / 2.0 - dot / 2.0), Mm(row_y[3] - dot / 2.0), Mm(dot), Mm(dot)),
        PaintMode::Fill,
    );
    layer.use_text("Locked door indicator", 7.0, Mm(lbl_x), Mm(row_y[3] - 1.3), font);

    // Row 4 — Lock-pick difficulty note
    let wood_diff = lock_picking_difficulty(DoorKind::Wooden, difficulty);
    let iron_diff = lock_picking_difficulty(DoorKind::Iron, difficulty);
    layer.use_text(
        format!("Pick: wooden={wood_diff}  iron={iron_diff}"),
        6.0,
        Mm(bx + 3.0),
        Mm(row_y[4] - 1.3),
        font,
    );
}

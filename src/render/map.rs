use std::f32::consts::PI;

use printpdf::path::{PaintMode, WindingOrder};
use printpdf::{Mm, PdfLayerReference, Point, Polygon, Rgb};

use crate::generator::bsp::{Rect, Room, RoomShape};
use crate::generator::corridor::Corridor;

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

pub fn draw_map(
    layer: &PdfLayerReference,
    rooms: &[Room],
    corridors: &[Corridor],
    page_height_mm: f32,
    tile_size: f32,
) {
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

    layer.set_fill_color(printpdf::Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)));
}

pub fn room_label_position(room: &Room, page_height_mm: f32, tile_size: f32) -> (Mm, Mm) {
    let cx = MAP_MARGIN_MM + (room.bounds.x as f32 + room.bounds.width as f32 / 2.0) * tile_size;
    let cy = page_height_mm
        - MAP_MARGIN_MM
        - (room.bounds.y as f32 + room.bounds.height as f32 / 2.0) * tile_size;
    (Mm(cx), Mm(cy))
}

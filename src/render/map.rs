use printpdf::{Line, Mm, PdfLayerReference, Point, Rgb};

use crate::generator::bsp::{Rect, Room};
use crate::generator::corridor::Corridor;

const MAP_MARGIN_MM: f32 = 15.0;
const MAX_TILE_SIZE_MM: f32 = 5.0;

/// Compute a tile size that fits the full dungeon grid within the page with margins.
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

fn rect_origin(rect: &Rect, page_height_mm: f32, tile_size: f32) -> (Mm, Mm) {
    let x = MAP_MARGIN_MM + rect.x as f32 * tile_size;
    let y = page_height_mm - MAP_MARGIN_MM - (rect.y as f32 + rect.height as f32) * tile_size;
    (Mm(x), Mm(y))
}

fn draw_filled_rect(layer: &PdfLayerReference, rect: &Rect, page_height_mm: f32, tile_size: f32) {
    let (ox, oy) = rect_origin(rect, page_height_mm, tile_size);
    let w = tile_to_mm(rect.width, tile_size);
    let h = tile_to_mm(rect.height, tile_size);

    let line = Line {
        points: vec![
            (Point::new(ox, oy), false),
            (Point::new(Mm(ox.0 + w.0), oy), false),
            (Point::new(Mm(ox.0 + w.0), Mm(oy.0 + h.0)), false),
            (Point::new(ox, Mm(oy.0 + h.0)), false),
        ],
        is_closed: true,
    };
    layer.add_line(line);
}

fn draw_outline_rect(layer: &PdfLayerReference, rect: &Rect, page_height_mm: f32, tile_size: f32) {
    let (ox, oy) = rect_origin(rect, page_height_mm, tile_size);
    let w = tile_to_mm(rect.width, tile_size);
    let h = tile_to_mm(rect.height, tile_size);

    let line = Line {
        points: vec![
            (Point::new(ox, oy), false),
            (Point::new(Mm(ox.0 + w.0), oy), false),
            (Point::new(Mm(ox.0 + w.0), Mm(oy.0 + h.0)), false),
            (Point::new(ox, Mm(oy.0 + h.0)), false),
        ],
        is_closed: true,
    };
    layer.add_line(line);
}

pub fn draw_map(
    layer: &PdfLayerReference,
    rooms: &[Room],
    corridors: &[Corridor],
    page_height_mm: f32,
    tile_size: f32,
) {
    layer.set_fill_color(printpdf::Color::Rgb(Rgb::new(0.75, 0.75, 0.75, None)));
    layer.set_outline_color(printpdf::Color::Rgb(Rgb::new(0.5, 0.5, 0.5, None)));
    layer.set_outline_thickness(0.5);

    for corridor in corridors {
        for segment in &corridor.segments {
            draw_filled_rect(layer, segment, page_height_mm, tile_size);
        }
    }

    layer.set_fill_color(printpdf::Color::Rgb(Rgb::new(1.0, 1.0, 1.0, None)));
    layer.set_outline_color(printpdf::Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)));
    layer.set_outline_thickness(1.0);

    for room in rooms {
        draw_outline_rect(layer, &room.bounds, page_height_mm, tile_size);
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

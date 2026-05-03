use printpdf::{Line, Mm, PdfLayerReference, Point, Rgb};

use crate::generator::bsp::{Rect, Room};
use crate::generator::corridor::Corridor;

const TILE_SIZE_MM: f32 = 3.5;
const MAP_ORIGIN_X: f32 = 15.0;
const MAP_ORIGIN_Y: f32 = 15.0;

fn tile_to_mm(tile: u32) -> Mm {
    Mm((tile as f32) * TILE_SIZE_MM)
}

fn rect_origin(rect: &Rect, page_height_mm: f32) -> (Mm, Mm) {
    let x = MAP_ORIGIN_X + (rect.x as f32) * TILE_SIZE_MM;
    let y = page_height_mm - MAP_ORIGIN_Y - ((rect.y as f32) + (rect.height as f32)) * TILE_SIZE_MM;
    (Mm(x), Mm(y))
}

fn draw_filled_rect(layer: &PdfLayerReference, rect: &Rect, page_height_mm: f32) {
    let (ox, oy) = rect_origin(rect, page_height_mm);
    let w = tile_to_mm(rect.width);
    let h = tile_to_mm(rect.height);

    let points = vec![
        (Point::new(ox, oy), false),
        (Point::new(Mm(ox.0 + w.0), oy), false),
        (Point::new(Mm(ox.0 + w.0), Mm(oy.0 + h.0)), false),
        (Point::new(ox, Mm(oy.0 + h.0)), false),
    ];

    let line = Line {
        points,
        is_closed: true,
    };
    layer.add_line(line);
}

fn draw_outline_rect(layer: &PdfLayerReference, rect: &Rect, page_height_mm: f32) {
    let (ox, oy) = rect_origin(rect, page_height_mm);
    let w = tile_to_mm(rect.width);
    let h = tile_to_mm(rect.height);

    let points = vec![
        (Point::new(ox, oy), false),
        (Point::new(Mm(ox.0 + w.0), oy), false),
        (Point::new(Mm(ox.0 + w.0), Mm(oy.0 + h.0)), false),
        (Point::new(ox, Mm(oy.0 + h.0)), false),
    ];

    let line = Line {
        points,
        is_closed: true,
    };
    layer.add_line(line);
}

pub fn draw_map(
    layer: &PdfLayerReference,
    rooms: &[Room],
    corridors: &[Corridor],
    page_height_mm: f32,
) {
    // Draw corridors first (grey fill)
    layer.set_fill_color(printpdf::Color::Rgb(Rgb::new(0.75, 0.75, 0.75, None)));
    layer.set_outline_color(printpdf::Color::Rgb(Rgb::new(0.5, 0.5, 0.5, None)));
    layer.set_outline_thickness(0.5);

    for corridor in corridors {
        for segment in &corridor.segments {
            draw_filled_rect(layer, segment, page_height_mm);
        }
    }

    // Draw rooms (white fill, black outline)
    layer.set_fill_color(printpdf::Color::Rgb(Rgb::new(1.0, 1.0, 1.0, None)));
    layer.set_outline_color(printpdf::Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)));
    layer.set_outline_thickness(1.0);

    for room in rooms {
        draw_outline_rect(layer, &room.bounds, page_height_mm);
    }

    // Draw room number labels (centre of each room)
    layer.set_fill_color(printpdf::Color::Rgb(Rgb::new(0.0, 0.0, 0.0, None)));
    // Note: text rendering is handled in pdf.rs where the font is available
}

pub fn room_label_position(room: &Room, page_height_mm: f32) -> (Mm, Mm) {
    let cx = MAP_ORIGIN_X + (room.bounds.x as f32 + room.bounds.width as f32 / 2.0) * TILE_SIZE_MM;
    let cy = page_height_mm
        - MAP_ORIGIN_Y
        - (room.bounds.y as f32 + room.bounds.height as f32 / 2.0) * TILE_SIZE_MM;
    (Mm(cx), Mm(cy))
}

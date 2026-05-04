use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use walkdir::WalkDir;

use crate::config::{Difficulty, Theme};

/// A single drawing primitive in normalised (−1..1) coordinate space.
/// The renderer scales by `min(room_w, room_h) * tile_size * 0.3` and
/// translates to the room centre — no Rust code change needed for new room types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DrawCommand {
    /// Stroke a straight line between two normalised points.
    Line { x1: f32, y1: f32, x2: f32, y2: f32 },
    /// Stroke (or fill+stroke) an axis-aligned rectangle.
    /// `x`, `y` are the bottom-left corner; `w`, `h` are the dimensions (all normalised).
    Rect { x: f32, y: f32, w: f32, h: f32, filled: bool },
    /// Stroke (or fill+stroke) a regular n-gon inscribed in a circle of normalised radius `r`.
    /// `start_angle` is in radians (0 = rightmost vertex).
    NGon { cx: f32, cy: f32, r: f32, n: u32, start_angle: f32, filled: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomContents {
    pub id: String,
    pub name: String,
    pub description: String,
    pub themes: Vec<Theme>,
    pub difficulty: Vec<Difficulty>,
    #[serde(default)]
    pub icon: Vec<DrawCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomContentsList(pub Vec<RoomContents>);

#[derive(Debug, Error)]
pub enum RoomContentsError {
    #[error("Failed to read room contents file {path}: {source}")]
    ReadError {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to parse room contents file {path}: {source}")]
    ParseError {
        path: String,
        #[source]
        source: ron::error::SpannedError,
    },
}

pub fn load_room_contents(dir: &Path) -> Result<Vec<RoomContents>> {
    let mut contents = Vec::new();

    for entry in WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "ron"))
    {
        let path = entry.path();
        let content = std::fs::read_to_string(path)
            .map_err(|e| RoomContentsError::ReadError {
                path: path.display().to_string(),
                source: e,
            })
            .with_context(|| format!("reading room contents file {}", path.display()))?;

        let RoomContentsList(mut list) = ron::from_str(&content)
            .map_err(|e| RoomContentsError::ParseError {
                path: path.display().to_string(),
                source: e,
            })
            .with_context(|| format!("parsing room contents file {}", path.display()))?;

        contents.append(&mut list);
    }

    Ok(contents)
}

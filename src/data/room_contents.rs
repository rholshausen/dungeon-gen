use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use walkdir::WalkDir;

use crate::config::{Difficulty, Theme};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomContents {
    pub id: String,
    pub name: String,
    pub description: String,
    pub themes: Vec<Theme>,
    pub difficulty: Vec<Difficulty>,
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

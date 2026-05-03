use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use walkdir::WalkDir;

use crate::config::{Difficulty, Theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AppliesTo {
    Room,
    Corridor,
    #[default]
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub name: String,
    pub trigger: String,
    /// Free-form difficulty rating in whatever notation the game system uses
    /// (e.g. `"DC 15"` for D&D, `"TN 12"` for GURPS, `"Difficulty 3"` for WFRP).
    pub difficulty_rating: Option<String>,
    pub effect: String,
    pub themes: Vec<Theme>,
    pub difficulty: Vec<Difficulty>,
    #[serde(default)]
    pub applies_to: AppliesTo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventList(pub Vec<Event>);

#[derive(Debug, Error)]
pub enum EventError {
    #[error("Failed to read event file {path}: {source}")]
    ReadError {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to parse event file {path}: {source}")]
    ParseError {
        path: String,
        #[source]
        source: ron::error::SpannedError,
    },
}

pub fn load_events(dir: &Path) -> Result<Vec<Event>> {
    let mut events = Vec::new();

    for entry in WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "ron"))
    {
        let path = entry.path();
        let content = std::fs::read_to_string(path)
            .map_err(|e| EventError::ReadError {
                path: path.display().to_string(),
                source: e,
            })
            .with_context(|| format!("reading event file {}", path.display()))?;

        let EventList(mut list) = ron::from_str(&content)
            .map_err(|e| EventError::ParseError {
                path: path.display().to_string(),
                source: e,
            })
            .with_context(|| format!("parsing event file {}", path.display()))?;

        events.append(&mut list);
    }

    Ok(events)
}

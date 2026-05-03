use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use walkdir::WalkDir;

use crate::config::{Difficulty, Theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CreatureTag {
    Small,
    Medium,
    Large,
    Humanoid,
    Beast,
    Undead,
    Common,
    Rare,
    Boss,
}

/// A single game-system stat — label and value are both free-form strings so
/// any system's notation fits without code changes (e.g. `("AC", "15")` for
/// D&D, `("DR", "4")` for GURPS, `("T", "3")` for WFRP).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stat {
    pub label: String,
    pub value: String,
}

/// An attack or ability. `stats` carries whatever the game system needs
/// (e.g. `[("Bonus", "+4"), ("Damage", "1d6+2 Slashing")]` for D&D 5e,
/// `[("Skill", "Shortsword-14"), ("Damage", "1d6+2 cut")]` for GURPS).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attack {
    pub name: String,
    pub stats: Vec<Stat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Description {
    pub appearance: String,
    pub tactics: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Creature {
    pub name: String,
    pub tags: Vec<CreatureTag>,
    pub themes: Vec<Theme>,
    pub difficulty: Vec<Difficulty>,
    pub description: Description,
    /// Game-system stats — any labels the RON file defines are rendered as-is.
    pub stats: Vec<Stat>,
    pub attacks: Vec<Attack>,
}

#[derive(Debug, Error)]
pub enum CreatureError {
    #[error("Failed to read creature file {path}: {source}")]
    ReadError {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to parse creature file {path}: {source}")]
    ParseError {
        path: String,
        #[source]
        source: ron::error::SpannedError,
    },
}

pub fn load_creatures(dir: &Path) -> Result<Vec<Creature>> {
    let mut creatures = Vec::new();

    for entry in WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "ron"))
    {
        let path = entry.path();
        let content = std::fs::read_to_string(path)
            .map_err(|e| CreatureError::ReadError {
                path: path.display().to_string(),
                source: e,
            })
            .with_context(|| format!("reading creature file {}", path.display()))?;

        let creature: Creature = ron::from_str(&content)
            .map_err(|e| CreatureError::ParseError {
                path: path.display().to_string(),
                source: e,
            })
            .with_context(|| format!("parsing creature file {}", path.display()))?;

        creatures.push(creature);
    }

    Ok(creatures)
}

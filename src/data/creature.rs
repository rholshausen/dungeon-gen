use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use walkdir::WalkDir;

use crate::config::{Difficulty, Theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DamageType {
    Slashing,
    Piercing,
    Bludgeoning,
    Fire,
    Cold,
    Poison,
    Necrotic,
    Radiant,
    Lightning,
    Thunder,
    Psychic,
    Force,
}

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

/// Tuple struct: `Dice(count, sides, modifier)` — matches RON file syntax.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dice(pub u32, pub u32, pub i32);

impl Dice {
    pub fn count(&self) -> u32 { self.0 }
    pub fn sides(&self) -> u32 { self.1 }
    pub fn modifier(&self) -> i32 { self.2 }

    pub fn average(&self) -> f32 {
        (self.0 as f32 * (self.1 as f32 + 1.0) / 2.0) + self.2 as f32
    }

    pub fn notation(&self) -> String {
        if self.2 == 0 {
            format!("{}d{}", self.0, self.1)
        } else if self.2 > 0 {
            format!("{}d{}+{}", self.0, self.1, self.2)
        } else {
            format!("{}d{}{}", self.0, self.1, self.2)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Damage(pub Dice, pub DamageType);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attack {
    pub name: String,
    pub bonus: i32,
    pub damage: Damage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Description {
    pub appearance: String,
    pub tactics: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Creature {
    pub name: String,
    pub cr: f32,
    pub hp: Dice,
    pub ac: u32,
    pub tags: Vec<CreatureTag>,
    pub themes: Vec<Theme>,
    pub difficulty: Vec<Difficulty>,
    pub description: Description,
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

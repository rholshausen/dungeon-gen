use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::cli::Cli;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
    Deadly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
pub enum Theme {
    Classic,
    Underdark,
    Ruins,
    Crypt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaperSize {
    A4,
    Letter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomCountRange {
    pub min: u32,
    pub max: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DungeonConfig {
    pub width: u32,
    pub height: u32,
    pub room_count: RoomCountRange,
    pub seed: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataConfig {
    pub creatures_dir: PathBuf,
    pub events_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub path: Option<PathBuf>,
    pub paper_size: PaperSize,
    pub include_dm_notes: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub dungeon: DungeonConfig,
    pub difficulty: Difficulty,
    pub theme: Theme,
    pub data: DataConfig,
    pub output: OutputConfig,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            dungeon: DungeonConfig {
                width: 80,
                height: 50,
                room_count: RoomCountRange { min: 6, max: 14 },
                seed: None,
            },
            difficulty: Difficulty::Medium,
            theme: Theme::Classic,
            data: DataConfig {
                creatures_dir: PathBuf::from("./data/creatures"),
                events_dir: PathBuf::from("./data/events"),
            },
            output: OutputConfig {
                path: None,
                paper_size: PaperSize::A4,
                include_dm_notes: true,
            },
        }
    }
}

impl Config {
    pub fn load(config_path: &Path) -> Result<Self> {
        let mut cfg = Config::default();

        if config_path.exists() {
            let content = std::fs::read_to_string(config_path)
                .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;
            cfg = ron::from_str(&content)
                .with_context(|| format!("Failed to parse config file: {}", config_path.display()))?;
        }

        Ok(cfg)
    }

    pub fn apply_cli_overrides(&mut self, cli: &Cli) {
        if let Some(seed) = cli.seed {
            self.dungeon.seed = Some(seed);
        }
        if let Some(difficulty) = cli.difficulty {
            self.difficulty = difficulty;
        }
        if let Some(theme) = cli.theme {
            self.theme = theme;
        }
        if let Some(ref output) = cli.output {
            self.output.path = Some(output.clone());
        }
        if let Some(width) = cli.width {
            self.dungeon.width = width;
        }
        if let Some(height) = cli.height {
            self.dungeon.height = height;
        }
    }
}

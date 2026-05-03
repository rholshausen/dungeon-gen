use std::path::PathBuf;

use clap::Parser;

use crate::config::{Difficulty, Theme};

#[derive(Parser, Debug)]
#[command(author, version, about = "Procedural dungeon generator — exports to PDF")]
pub struct Cli {
    /// Path to config.ron (default: ./config.ron)
    #[arg(long, default_value = "./config.ron")]
    pub config: PathBuf,

    /// RNG seed for reproducible dungeons
    #[arg(long)]
    pub seed: Option<u64>,

    /// Encounter difficulty
    #[arg(long, value_enum)]
    pub difficulty: Option<Difficulty>,

    /// Dungeon theme
    #[arg(long, value_enum)]
    pub theme: Option<Theme>,

    /// Output PDF path
    #[arg(long, short)]
    pub output: Option<PathBuf>,

    /// Dungeon grid width in tiles
    #[arg(long)]
    pub width: Option<u32>,

    /// Dungeon grid height in tiles
    #[arg(long)]
    pub height: Option<u32>,
}

mod cli;
mod config;
mod data;
mod generator;
mod render;
mod seed;

use anyhow::Result;
use clap::Parser;

use cli::Cli;
use config::Config;

fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut cfg = Config::load(&cli.config)?;
    cfg.apply_cli_overrides(&cli);

    let creatures = data::creature::load_creatures(&cfg.data.creatures_dir)?;
    let events = data::event::load_events(&cfg.data.events_dir)?;

    let (mut rng, seed_used) = seed::make_rng(cfg.dungeon.seed);

    let mut rooms = generator::bsp::generate(&cfg.dungeon, &mut rng);
    let corridors = generator::corridor::stitch(&rooms, &mut rng);

    generator::populator::seed(
        &mut rooms,
        &creatures,
        &events,
        cfg.difficulty,
        cfg.theme,
        &mut rng,
    );

    let creatures_used = generator::populator::unique_creatures(&rooms);

    render::pdf::render(
        &rooms,
        &corridors,
        &creatures_used,
        &cfg.output,
        seed_used,
        &cfg.output.path,
    )?;

    println!(
        "Dungeon generated: {} rooms, seed {}. PDF written to {}",
        rooms.len(),
        seed_used,
        cfg.output.path.display()
    );

    Ok(())
}

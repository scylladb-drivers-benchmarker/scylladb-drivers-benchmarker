pub mod aliasing;
pub mod benchmark;
pub mod database;
pub mod plot;
#[cfg(test)]
mod tests;
use crate::BenchmarkParams;
use crate::Database;
use crate::PlotParams;
use crate::parsing::aliasing::AliasingConfig;
use crate::parsing::benchmark::BenchmarkCommand;
use crate::parsing::database::DatabaseArgs;
use crate::parsing::plot::PlotCommand;
use clap::Parser;
use scylladb_drivers_benchmarker::utilities::DatabaseCommand;
use std::env;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[clap(name = "my-app", version, about)]
struct App {
    #[arg(short, long)]
    db_path: Option<PathBuf>,

    #[arg(short, long)]
    aliasing_config_path: Option<PathBuf>,

    #[clap(subcommand)]
    subcommand: AppSubcommands,
}

#[derive(Debug, clap::Subcommand)]
pub enum AppSubcommands {
    Run(BenchmarkCommand), // TODO zmienilbym na benchmark?

    Plot(PlotCommand),

    Database(DatabaseArgs),
}

pub struct ParsedParams {
    pub database: Database,
    pub aliasing_config: AliasingConfig,
    pub params: Subcommands,
}

pub enum Subcommands {
    Benchmark(BenchmarkParams),
    Plot(PlotParams),
    Database(DatabaseCommand),
}

impl App {
    pub fn finalize(self) -> Result<ParsedParams, Box<dyn std::error::Error>> {
        let aliasing_config = self
            .aliasing_config_path
            .or_else(|| env::var_os("SDB_CONFIG").map(Into::into))
            .map(|path| AliasingConfig::read_config(&path))
            .unwrap_or(Ok(AliasingConfig::default()))?;

        let db_path = self
            .db_path
            .or(aliasing_config.dp_path.clone())
            .map(Ok)
            .unwrap_or_else(default_db_path)?;

        let database = Database::new(&db_path)?;

        let params: Subcommands = match self.subcommand {
            AppSubcommands::Run(x) => Subcommands::Benchmark(x.finalize()?),
            AppSubcommands::Plot(x) => Subcommands::Plot(x.finalize(&aliasing_config)?),
            AppSubcommands::Database(x) => Subcommands::Database(x.finalize()?),
        };
        Ok(ParsedParams {
            database,
            aliasing_config,
            params,
        })
    }
}

#[justerror::Error(desc = "Failed to obtain default database location. Provide one.")]
pub enum DbPathError {
    NoHomeDir,
}

fn default_db_path() -> Result<std::path::PathBuf, DbPathError> {
    Ok(home::home_dir()
        .ok_or(DbPathError::NoHomeDir)?
        .join("SDB_benchmarker.db"))
}

pub fn parse_all() -> Result<ParsedParams, Box<dyn std::error::Error>> {
    App::parse().finalize()
}

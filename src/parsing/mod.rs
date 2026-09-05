mod benchmark;
mod benchmark_setup;
mod database;
mod plot;
#[cfg(test)]
mod tests;

use std::io;
use std::path::PathBuf;

use clap::Parser;
use log::{info, trace};
use plot::DriverWithCommitParsingError;
use scylladb_drivers_benchmarker::config::ConfigError;
use scylladb_drivers_benchmarker::config::api::ApiConfigError;
use scylladb_drivers_benchmarker::config::driver::DriverSpecError;
use scylladb_drivers_benchmarker::database::DatabaseError;

use crate::parsing::benchmark::BenchmarkCommand;
use crate::parsing::database::{DatabaseArgs, DbPathError, default_db_path};
use crate::parsing::plot::PlotCommand;
use crate::{BenchmarkParams, Database, DropDatabaseParams, PlotParams, PrintDatabaseParams};

#[derive(Debug, Parser)]
#[clap(name = "scylladb-drivers-benchmarker", version, about)]
pub(crate) struct App {
    #[arg(short, long)]
    db_path: Option<PathBuf>,

    #[clap(subcommand)]
    subcommand: AppSubcommands,
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum AppSubcommands {
    Run(BenchmarkCommand),
    Plot(PlotCommand),
    Database(DatabaseArgs),
}

pub(crate) struct ParsedParams {
    pub database: Database,
    pub params: Subcommands,
}

pub(crate) enum Subcommands {
    Benchmark(BenchmarkParams),
    Plot(PlotParams),
    PrintDatabase(PrintDatabaseParams),
    DropDatabase(DropDatabaseParams),
}

#[justerror::Error(desc = "Failed to parse or obtain necessary parameters")]
pub(crate) enum ParsingError {
    DatabasePathAccess(#[from] DbPathError),
    DatabaseInitialization(#[from] DatabaseError),
    SeriesError(#[from] DriverWithCommitParsingError),
    BenchmarkConfigError(#[from] ConfigError),
    DriverSpec(#[from] DriverSpecError),
    ApiConfig(#[from] ApiConfigError),
    #[error(desc = "Scenario '{name}' not found in {config_path}")]
    UnknownScenario {
        name: String,
        config_path: PathBuf,
    },
    FailedCanonicalizing(#[from] io::Error),
    #[error(desc = "Given path to store is not a directory")]
    StoreDirNotADir,
    #[error(desc = "Benchmark configuration not found")]
    NoBenchmarkConfiguration,
}

impl App {
    pub fn finalize(self) -> Result<ParsedParams, ParsingError> {
        info!("Gathering data...");
        let db_path = self.db_path.map_or_else(default_db_path, Ok)?;

        trace!("Database resolved to: {}", db_path.display());
        let database = Database::new(&db_path)?;

        let params: Subcommands = match self.subcommand {
            AppSubcommands::Run(x) => x.finalize()?,
            AppSubcommands::Plot(x) => x.finalize()?,
            AppSubcommands::Database(x) => x.finalize(),
        };

        Ok(ParsedParams { database, params })
    }
}

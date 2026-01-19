mod aliasing;
mod benchmark;
mod benchmark_setup;
mod database;
mod plot;
#[cfg(test)]
mod tests;

use std::path::PathBuf;
use std::{env, io};

use clap::Parser;
use scylladb_drivers_benchmarker::config::ConfigError;
use scylladb_drivers_benchmarker::database::DatabaseError;
use scylladb_drivers_benchmarker::measurement::MeasurementMethod;
use scylladb_drivers_benchmarker::repo_with_commits::RepoNameWithCommitsParsingError;

use crate::parsing::aliasing::{AliasingConfig, MainConfigError};
use crate::parsing::benchmark::BenchmarkCommand;
use crate::parsing::database::{DatabaseArgs, DbPathError, default_db_path};
use crate::parsing::plot::PlotCommand;
use crate::{BenchmarkParams, Database, DropDatabaseParams, PlotParams, PrintDatabaseParams};

#[derive(Debug, Parser)]
#[clap(name = "my-app", version, about)]
pub(crate) struct App {
    #[arg(short, long)]
    db_path: Option<PathBuf>,

    #[arg(short, long)]
    aliasing_config_path: Option<PathBuf>,

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
    AliasingConfig(#[from] MainConfigError),
    DatabasePathAccess(#[from] DbPathError),
    DatabaseInitialization(#[from] DatabaseError),
    FromClauser(#[from] RepoNameWithCommitsParsingError),
    BenchmarkConfigError(#[from] ConfigError),
    NoStoreDir {
        needed_by: MeasurementMethod,
    },
    FailedCanonicalizing(#[from] io::Error),
    #[error(desc = "Given path to store is not a directory")]
    StoreDirNotADir,
    #[error(desc = "Benchmark configuration not found")]
    NoBenchmarkConfiguration,
    #[error(desc = "Flame graph repository path not provided")]
    NoFlameGraphRepository,
}

impl App {
    pub fn finalize(self) -> Result<ParsedParams, ParsingError> {
        let aliasing_config = self
            .aliasing_config_path
            .or_else(|| env::var_os("SDB_CONFIG").map(Into::into))
            .map_or(Ok(AliasingConfig::default()), |path| {
                AliasingConfig::read_config(&path)
            })?;

        let db_path = self
            .db_path
            .or(aliasing_config.db_path.clone())
            .map_or_else(default_db_path, Ok)?;

        let database = Database::new(&db_path)?;

        let params: Subcommands = match self.subcommand {
            AppSubcommands::Run(x) => x.finalize(aliasing_config)?,
            AppSubcommands::Plot(x) => x.finalize(aliasing_config)?,
            AppSubcommands::Database(x) => x.finalize(),
        };

        Ok(ParsedParams { database, params })
    }
}

use crate::parsing::Subcommands;

use clap::Args;
use clap::Subcommand;

use crate::DropDatabaseParams;
use crate::PrintDatabaseParams;
use scylladb_drivers_benchmarker::database::utilities::BenchmarkFilters;
use scylladb_drivers_benchmarker::utilities::BenchmarkPoint;

#[derive(Debug, clap::Args)]
pub struct DatabaseArgs {
    #[command(subcommand)]
    pub command: DatabaseCommandINPUT,
}

#[derive(Subcommand, Debug)]
pub enum DatabaseCommandINPUT {
    Print {
        #[command(flatten)]
        filters: InputDatabaseFilters,
    },

    Drop {
        #[command(flatten)]
        filters: InputDatabaseFilters,
    },
}

#[derive(Args, Debug)]
pub struct InputDatabaseFilters {
    #[arg(long = "commit-hash", value_delimiter = ':', num_args(1..))]
    pub commit_hashes: Vec<String>,

    #[arg(long = "benchmark-name", value_delimiter = ':', num_args(1..))]
    pub benchmark_names: Vec<String>,

    #[arg(long = "benchmark-point", value_delimiter = ':', num_args(1..))]
    pub benchmark_points: Vec<BenchmarkPoint>,

    #[arg(long = "measurement-method", value_delimiter = ':', num_args(1..))]
    pub measurement_methods: Vec<String>,
}

impl From<InputDatabaseFilters> for BenchmarkFilters {
    fn from(input: InputDatabaseFilters) -> Self {
        BenchmarkFilters {
            commit_hashes: input.commit_hashes,
            benchmark_names: input.benchmark_names,
            benchmark_points: input.benchmark_points,
            measurement_methods: input.measurement_methods,
        }
    }
}

impl DatabaseArgs {
    pub fn finalize(self) -> Subcommands {
        match self.command {
            DatabaseCommandINPUT::Print { filters } => {
                Subcommands::PrintDatabase(PrintDatabaseParams {
                    filters: (filters.into()),
                })
            }
            DatabaseCommandINPUT::Drop { filters } => {
                Subcommands::DropDatabase(DropDatabaseParams {
                    filters: (filters.into()),
                })
            }
        }
    }
}

#[justerror::Error(desc = "Failed to obtain default database location. Provide one.")]
pub enum DbPathError {
    NoHomeDir,
}

pub fn default_db_path() -> Result<std::path::PathBuf, DbPathError> {
    Ok(home::home_dir()
        .ok_or(DbPathError::NoHomeDir)?
        .join("SDB_benchmarker.db"))
}

use clap::Args;
use clap::Subcommand;

use scylladb_drivers_benchmarker::database::utilities::BenchmarkFilters;
use scylladb_drivers_benchmarker::utilities::BenchmarkPoint;
use scylladb_drivers_benchmarker::utilities::DatabaseCommand;

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
    pub fn finalize(self) -> Result<DatabaseCommand, Box<dyn std::error::Error>> {
        match self.command {
            DatabaseCommandINPUT::Print { filters } => Ok(DatabaseCommand::Print {
                filters: filters.into(),
            }),
            DatabaseCommandINPUT::Drop { filters } => Ok(DatabaseCommand::Drop {
                filters: filters.into(),
            }),
        }
    }
}

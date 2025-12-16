use clap::Parser;

use scylladb_drivers_benchmarker::{
    VisKind,
    database::Database,
    utilities::{BenchmarkMode, DatabaseCommand, RepositoryWithCommits},
};
use std::path::PathBuf;

#[derive(Debug, clap::Subcommand)]
enum AppSubcommand {
    /// Plot the results of previous benchmarks from the database.
    Plot {
        #[arg(short, long)]
        visualization_kind: Option<VisKind>,

        /// The source of data for the plot
        #[arg(long, value_name = "REPOSITORY_PATH:TAG1,TAG2,...")]
        from: Vec<RepositoryWithCommits>,
    },

    /// Collect the results of benchmarks and store to the database.
    Run {
        #[arg(short, long, default_value = "./config.yml")]
        backend_config_path: PathBuf,

        #[arg(long, short = 'm', value_enum, default_value_t = BenchmarkMode::UseCached)]
        benchmark_mode: BenchmarkMode,
    },
    Database {
        #[command(subcommand)]
        command: DatabaseCommand,
    },
}

/// Benchmarker and plotter for git-based applications.
#[derive(Debug, Parser)]
#[clap(name = "my-app", version, about)]
struct App {
    #[arg(short, long)]
    db_path: Option<PathBuf>,

    #[arg(short, long)]
    #[clap(default_value = "time -f \"%e\"")]
    measurement_method: String,

    benchmark_name: String,

    #[arg(short, long, default_value = "./config.yml")]
    benchmark_config_path: PathBuf,

    #[clap(subcommand)]
    subcommand: AppSubcommand,
}

#[justerror::Error(desc = "Failed to obtain default database location. Provide one.")]
pub enum DbPathError {
    NoHomeDir,
}

fn default_db_path() -> Result<std::path::PathBuf, DbPathError> {
    Ok(home::home_dir()
        .ok_or(DbPathError::NoHomeDir)?
        .join("benchmarker.db"))
}

fn print_error<T>(err: impl std::error::Error) -> T {
    println!("{}", err);
    std::process::exit(1);
}

fn main() {
    let args = App::parse();

    let db_path = args
        .db_path
        .map(Ok)
        .unwrap_or_else(default_db_path)
        .unwrap_or_else(print_error);

    let database = Database::new(db_path).unwrap_or_else(print_error);

    match args.subcommand {
        AppSubcommand::Run {
            backend_config_path,
            benchmark_mode,
        } => scylladb_drivers_benchmarker::run_benchmarks(
            &database,
            &args.benchmark_name,
            &args.benchmark_config_path,
            args.measurement_method,
            backend_config_path.as_path(),
            benchmark_mode,
        )
        .unwrap_or_else(print_error),
        AppSubcommand::Plot {
            visualization_kind,
            from,
        } => scylladb_drivers_benchmarker::plot_benchmarks(
            &database,
            &args.benchmark_name,
            &args.benchmark_config_path,
            &args.measurement_method,
            visualization_kind,
            from,
        )
        .unwrap_or_else(print_error),
        AppSubcommand::Database { command } => {
            scylladb_drivers_benchmarker::access_database(&database, command)
                .unwrap_or_else(print_error)
        }
    }
}

#[cfg(test)]
mod test {
    use std::path::Path;

    use clap::Parser;

    use crate::{App, AppSubcommand, RepositoryWithCommits};

    #[test]
    fn basic_run() {
        let args = App::parse_from(vec!["scylladb-drivers-benchmarker", "select", "run"]);
        assert_eq!(args.measurement_method, "time -f \"%e\"");
        assert_eq!(args.benchmark_name, "select");
        assert!(matches!(args.subcommand, AppSubcommand::Run { .. }));
    }

    #[test]
    fn advanced_plot() {
        let args = App::parse_from(vec![
            "scylladb-drivers-benchmarker",
            "select",
            "plot",
            "--from=repo:branch",
            "--from",
            "repo2:commit",
        ]);
        assert_eq!(args.measurement_method, "time -f \"%e\"");
        assert_eq!(args.benchmark_name, "select");

        let AppSubcommand::Plot {
            visualization_kind,
            from,
        } = args.subcommand
        else {
            panic!("Not a plot");
        };

        assert_eq!(visualization_kind, None);
        assert_eq!(
            from,
            vec!(
                RepositoryWithCommits {
                    repo_path: Path::new("repo").to_path_buf(),
                    commits: vec!("branch".to_owned())
                },
                RepositoryWithCommits {
                    repo_path: Path::new("repo2").to_path_buf(),
                    commits: vec!("commit".to_owned())
                }
            )
        );
    }
}

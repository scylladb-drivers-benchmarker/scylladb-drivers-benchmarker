mod repo_with_commits;

use clap::Parser;

use scylladb_drivers_benchmarker::{
    OutputFormat, VisKind,
    database::Database,
    utilities::{BenchmarkMode, DatabaseCommand, RepoNameWithTags, RepoPathWithCommits},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env};
use std::{fs::File, path::Path};
use std::{io, path::PathBuf};

use crate::repo_with_commits::{ParsableRepoNameWithTags, resolve_repo_tags};

#[derive(Debug, clap::Subcommand)]
enum AppSubcommand {
    /// Plot the results of previous benchmarks from the database.
    Plot {
        benchmark_name: String,

        #[arg(short, long)]
        #[clap(default_value = "time -f \"%e\"")]
        measurement_method: String,

        #[arg(short, long, default_value = "./config.yml")]
        benchmark_config_path: PathBuf,

        #[arg(short, long)]
        visualization_kind: Option<VisKind>,

        /// The source of data for the plot
        #[arg(long, value_name = "REPOSITORY_PATH:TAG1,TAG2,...")]
        from: Vec<ParsableRepoNameWithTags>,

        #[arg(long, value_enum, default_value_t = OutputFormat::Png)]
        format: OutputFormat,

        /// Path to save the plot image
        #[arg(short, long, value_name = "FILE_PATH")]
        output: Option<PathBuf>,
    },

    /// Collect the results of benchmarks and store to the database.
    Run {
        benchmark_name: String,

        #[arg(short, long)]
        #[clap(default_value = "time -f \"%e\"")]
        measurement_method: String,

        #[arg(short = 'B', long, default_value = "./config.yml")]
        backend_config_path: PathBuf,

        #[arg(short, long, default_value = "./config.yml")]
        benchmark_config_path: PathBuf,

        #[arg(long, short = 'M', value_enum, default_value_t = BenchmarkMode::UseCached)]
        benchmark_mode: BenchmarkMode,
    },

    /// Interact with the underlying db
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
    aliasing_config_path: Option<PathBuf>,

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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
struct AliasingConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    dp_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    repo_path: HashMap<String, PathBuf>,
}

#[justerror::Error(desc = "Failed reading the main config file")]
enum MainConfigError {
    FailedOpening(#[from] io::Error),
    FailedParsing(#[from] serde_yml::Error),
}

impl AliasingConfig {
    fn read_config(path: &Path) -> Result<Self, MainConfigError> {
        let file = File::open(path)?;
        Ok(serde_yml::from_reader(file)?)
    }
}

fn main() {
    let args = App::parse();
    let aliasing_config: AliasingConfig = args
        .aliasing_config_path
        .or_else(|| env::var_os("SDB_CONFIG").map(Into::into))
        .map(|path| AliasingConfig::read_config(&path).unwrap_or_else(print_error))
        .unwrap_or_default();

    let db_path = args
        .db_path
        .or(aliasing_config.dp_path)
        .map(Ok)
        .unwrap_or_else(default_db_path)
        .unwrap_or_else(print_error);

    let database = Database::new(db_path).unwrap_or_else(print_error);

    match args.subcommand {
        AppSubcommand::Run {
            benchmark_name,
            measurement_method,
            benchmark_config_path,
            backend_config_path,
            benchmark_mode,
        } => scylladb_drivers_benchmarker::run_benchmarks(
            &database,
            &benchmark_name,
            &benchmark_config_path,
            measurement_method,
            backend_config_path.as_path(),
            benchmark_mode,
        )
        .unwrap_or_else(print_error),

        AppSubcommand::Plot {
            benchmark_name,
            measurement_method,
            benchmark_config_path,
            visualization_kind,
            from,
            format,
            output,
        } => {
            let parsed: Vec<RepoNameWithTags> = from.into_iter().map(Into::into).collect();

            let resolved = parsed
                .iter()
                .map(|repo| resolve_repo_tags(repo.clone(), &aliasing_config.repo_path))
                .collect::<Result<Vec<RepoPathWithCommits>, _>>()
                .unwrap_or_else(print_error);

            scylladb_drivers_benchmarker::plot_benchmarks(
                &database,
                &benchmark_name,
                &benchmark_config_path,
                &measurement_method,
                visualization_kind,
                parsed,
                resolved,
                format,
                output.as_deref(),
            )
            .unwrap_or_else(print_error)
        }

        AppSubcommand::Database { command } => {
            scylladb_drivers_benchmarker::access_database(&database, command)
                .unwrap_or_else(print_error)
        }
    }
}

#[cfg(test)]
mod test {
    use clap::Parser;

    use crate::{App, AppSubcommand, DatabaseCommand};
    use scylladb_drivers_benchmarker::utilities::RepoNameWithTags;

    use super::OutputFormat;

    #[test]
    fn basic_run() {
        let args = App::parse_from(vec!["scylladb-drivers-benchmarker", "run", "select"]);

        let AppSubcommand::Run {
            benchmark_name,
            measurement_method,
            ..
        } = args.subcommand
        else {
            panic!("Not a run")
        };

        assert_eq!(benchmark_name, "select");
        assert_eq!(measurement_method, "time -f \"%e\"");
    }

    #[test]
    fn advanced_plot() {
        let args = App::parse_from(vec![
            "scylladb-drivers-benchmarker",
            "plot",
            "select",
            "--from=repo:branch",
            "--from",
            "repo2:commit",
            "--format=svg",
        ]);

        let AppSubcommand::Plot {
            benchmark_name,
            measurement_method,
            benchmark_config_path: _,
            visualization_kind,
            from,
            output,
            format,
        } = args.subcommand
        else {
            panic!("Not a plot");
        };

        let from: Vec<RepoNameWithTags> = from.into_iter().map(From::from).collect();

        let from: Vec<RepoNameWithTags> = from.into_iter().map(From::from).collect();

        assert_eq!(benchmark_name, "select");
        assert_eq!(measurement_method, "time -f \"%e\"");
        assert_eq!(visualization_kind, None);

        assert_eq!(
            from,
            vec!(
                RepoNameWithTags {
                    name: "repo".to_owned(),
                    tags: vec!("branch".to_owned())
                },
                RepoNameWithTags {
                    name: "repo2".to_owned(),
                    tags: vec!("commit".to_owned())
                }
            )
        );
        assert_eq!(output, None);
        assert!(matches!(format, OutputFormat::Svg));
    }

    #[test]
    fn advanced_database() {
        let args = App::parse_from(vec![
            "scylladb-drivers-benchmarker",
            "database",
            "print",
            "--commit-hash=test:21123123:ff",
            "--benchmark-name=my:benchmark:",
            "--benchmark-point=1:2:5:3",
            "--measurement-method=m1:m2:m4",
        ]);
        let AppSubcommand::Database { command } = args.subcommand else {
            panic!("Expected Database subcommand");
        };

        let DatabaseCommand::Print { filters } = command else {
            panic!("Expected DatabaseCommand::Print");
        };

        assert_eq!(filters.commit_hashes, vec!["test", "21123123", "ff"]);
        assert_eq!(filters.benchmark_names, vec!["my", "benchmark", ""]);
        assert_eq!(filters.benchmark_points, vec![1, 2, 5, 3]);
        assert_eq!(filters.measurement_methods, vec!["m1", "m2", "m4"]);
    }
}

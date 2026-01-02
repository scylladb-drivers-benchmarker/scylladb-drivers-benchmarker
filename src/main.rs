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
        #[arg(short, long, default_value = "./config.yml")]
        backend_config_path: PathBuf,

        #[arg(long, short = 'm', value_enum, default_value_t = BenchmarkMode::UseCached)]
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
    println!("aliasing_config: {aliasing_config:?}");

    let db_path = args
        .db_path
        .or(aliasing_config.dp_path)
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
                &args.benchmark_name,
                &args.benchmark_config_path,
                &args.measurement_method,
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

    use crate::{App, AppSubcommand};
    use scylladb_drivers_benchmarker::utilities::RepoNameWithTags;

    use super::OutputFormat;

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
            "--format=svg",
        ]);
        assert_eq!(args.measurement_method, "time -f \"%e\"");
        assert_eq!(args.benchmark_name, "select");

        let AppSubcommand::Plot {
            visualization_kind,
            from,
            output,
            format,
        } = args.subcommand
        else {
            panic!("Not a plot");
        };

        let from: Vec<RepoNameWithTags> = from.into_iter().map(From::from).collect();

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
}

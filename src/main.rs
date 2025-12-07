use anyhow::Result;
use clap::Parser;
use scylladb_drivers_benchmarker::{database::Database, utilities::RepositoryWithCommits};
use std::{path::PathBuf};

#[derive(Debug, clap::Subcommand)]
enum AppSubcommand {
    Plot {
        #[arg(short, long)]
        visualization_kind: Option<String>,

        #[arg(short, long)]
        from: Vec<RepositoryWithCommits>,
    },
    Run {
        #[arg(long, default_value = "./config.yml")]
        backend_config_path: PathBuf,
    },
}

#[derive(Debug, Parser)]
#[clap(name = "my-app", version, about)]
struct App {
    #[arg(short, long)]
    db_path: Option<PathBuf>,

    #[arg(short, long)]
    #[clap(default_value = "time")]
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
    let home = home::home_dir().ok_or(DbPathError::NoHomeDir)?;
    Ok(home.join("benchmarker.db"))
}

fn main() -> Result<()> {
    let args = App::parse();

    let db_path: PathBuf = match args.db_path {
        Some(path) => path.into(),
        None => default_db_path()?,
    };

    let database = Database::new(db_path)?;

    match args.subcommand {
        AppSubcommand::Run {
            backend_config_path,
        } => scylladb_drivers_benchmarker::run_benchmarks(
            &database,
            &args.benchmark_name,
            &args.benchmark_config_path,
            args.measurement_method,
            backend_config_path.as_path(),
        )?,
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
        )?,
    };

    Ok(())
}

#[cfg(test)]
mod test {
    use std::path::Path;

    use clap::Parser;

    use crate::{App, AppSubcommand, RepositoryWithCommits};

    #[test]
    fn basic_run() {
        let args = App::parse_from(vec!["scylladb-drivers-benchmarker", "select", "run"]);
        assert_eq!(args.measurement_method, "time");
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
        assert_eq!(args.measurement_method, "time");
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

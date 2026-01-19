use std::path::PathBuf;
use std::str::FromStr;

use clap::Args;
use scylladb_drivers_benchmarker::measurement::MeasurementMethod;
use scylladb_drivers_benchmarker::repo_with_commits::{
    RepoNameWithCommitsParsingError, RepoPathWithCommits, resolve_repo_tags,
};
use scylladb_drivers_benchmarker::{PlotSettings, VisKind};

use crate::parsing::aliasing::AliasingConfig;
use crate::parsing::benchmark_setup::BenchmarkSetup;
use crate::parsing::{ParsingError, Subcommands};
use crate::{PlotKind, PlotParams, RepoNameWithTags};

#[derive(Args, Debug)]
pub struct PlotCommand {
    pub benchmark_name: String,

    #[arg(short, long, default_value = "./config.yml")]
    pub benchmark_setup: Option<BenchmarkSetup>,

    /// The source of data for the plot
    #[arg(long, value_name = "REPOSITORY_PATH:TAG1,TAG2,...")]
    pub from: Vec<ParsableRepoNameWithTags>,

    /// Path to save the plot image
    #[arg(short, long, value_name = "FILE_PATH")]
    pub output: PathBuf,

    // Type of plot to generate
    #[clap(subcommand)]
    pub plot_kind: InputPlotKind,
}

#[derive(Debug, Clone)]
pub struct ParsableRepoNameWithTags(RepoNameWithTags);

impl From<ParsableRepoNameWithTags> for RepoNameWithTags {
    fn from(value: ParsableRepoNameWithTags) -> Self {
        value.0
    }
}

impl FromStr for ParsableRepoNameWithTags {
    type Err = RepoNameWithCommitsParsingError;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let (repo_names_str, tags_str) = string
            .split_once(':')
            .ok_or(RepoNameWithCommitsParsingError::PathNotSupplied)?;

        Ok(ParsableRepoNameWithTags(RepoNameWithTags {
            name: repo_names_str.to_owned(),
            tags: tags_str.split(',').map(str::to_owned).collect(),
        }))
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum InputPlotKind {
    /// Generate a series plot
    Series {
        #[arg(short, long, default_value_t = MeasurementMethod::Time)]
        measurement_method: MeasurementMethod,

        #[arg(short, long, value_enum, default_value_t = InputVisKind::Linear)]
        visualization_kind: InputVisKind,
    },

    /// Generate a flame-graph plot
    FlameGraph {
        #[arg(short, long, value_name = "DIR")]
        artifacts_dir: Option<PathBuf>,

        #[arg(short, long, value_name = "DIR")]
        flame_repo: Option<PathBuf>,
    },

    /// Generate a perf-stat plot
    PerfStat {
        #[arg(short, long)]
        #[clap(value_delimiter=',', num_args(1..))]
        events: Vec<String>,
    },
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, clap::ValueEnum)]
pub enum InputVisKind {
    Linear,
    Log,
}

impl InputVisKind {
    pub fn finalize(self) -> VisKind {
        match self {
            InputVisKind::Linear => VisKind::Linear,
            InputVisKind::Log => VisKind::Log,
        }
    }
}

impl InputPlotKind {
    pub fn finalize(self, aliasing_config: AliasingConfig) -> Result<PlotKind, ParsingError> {
        match self {
            InputPlotKind::Series {
                measurement_method,
                visualization_kind,
            } => Ok(PlotKind::Series {
                measurement_method,
                visualization_kind: visualization_kind.finalize(),
            }),
            InputPlotKind::FlameGraph {
                artifacts_dir,
                flame_repo,
            } => Ok(PlotKind::FlameGraph {
                artifacts_dir,
                flame_repo: flame_repo
                    .or(aliasing_config.flame_path)
                    .ok_or(ParsingError::NoFlameGraphRepository)?,
            }),
            InputPlotKind::PerfStat { events } => Ok(PlotKind::PerfStat { events }),
        }
    }
}

impl PlotCommand {
    pub fn finalize(self, aliasing_config: AliasingConfig) -> Result<Subcommands, ParsingError> {
        let parsed: Vec<RepoNameWithTags> = self.from.into_iter().map(Into::into).collect();

        let resolved = parsed
            .iter()
            .map(|repo| resolve_repo_tags(repo.clone(), &aliasing_config.repo_path))
            .collect::<Result<Vec<RepoPathWithCommits>, _>>()?;

        Ok(Subcommands::Plot(PlotParams {
            benchmark_config: BenchmarkSetup::finalize(
                self.benchmark_setup,
                &self.benchmark_name,
                &aliasing_config,
            )?,
            from: parsed,
            resolved,
            plot_settings: PlotSettings::new(
                self.plot_kind.finalize(aliasing_config)?,
                self.output.to_string_lossy().to_string(),
            ),
        }))
    }
}

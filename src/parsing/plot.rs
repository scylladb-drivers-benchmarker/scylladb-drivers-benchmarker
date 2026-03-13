use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

use clap::Args;
use scylladb_drivers_benchmarker::BackendWithCommit;
use scylladb_drivers_benchmarker::commit_hash::{CommitHash, FailedToRetrieveCommitHash};
use scylladb_drivers_benchmarker::measurement::MeasurementMethod;
use scylladb_drivers_benchmarker::{PlotSettings, VisKind};

use crate::parsing::aliasing::AliasingConfig;
use crate::parsing::benchmark_setup::BenchmarkSetup;
use crate::parsing::{ParsingError, Subcommands};
use crate::{PlotKind, PlotParams};

#[justerror::Error]
pub(crate) enum BackendWithCommitParsingError {
    #[error(desc = "expected format BACKEND@REPO[:REF] \u{2014} '@' separator is missing")]
    MissingAtSign,
    #[error(desc = "backend name cannot be empty")]
    EmptyBackendName,
    #[error(desc = "repository path cannot be empty")]
    EmptyRepo,
    HashResolutionFailed(#[from] Box<FailedToRetrieveCommitHash>),
}

#[derive(Debug, Clone)]
pub(crate) struct ParsableBackendWithCommit {
    backend_name: String,
    repo: String,
    tag: String,
}

impl FromStr for ParsableBackendWithCommit {
    type Err = BackendWithCommitParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (backend_name, rest) = s
            .split_once('@')
            .ok_or(BackendWithCommitParsingError::MissingAtSign)?;
        if backend_name.is_empty() {
            return Err(BackendWithCommitParsingError::EmptyBackendName);
        }
        let (repo, tag) = match rest.split_once(':') {
            Some((repo, tag)) => {
                if repo.is_empty() {
                    return Err(BackendWithCommitParsingError::EmptyRepo);
                }
                (repo, if tag.is_empty() { "HEAD" } else { tag })
            }
            None => {
                if rest.is_empty() {
                    return Err(BackendWithCommitParsingError::EmptyRepo);
                }
                (rest, "HEAD")
            }
        };
        Ok(ParsableBackendWithCommit {
            backend_name: backend_name.to_owned(),
            repo: repo.to_owned(),
            tag: tag.to_owned(),
        })
    }
}

impl ParsableBackendWithCommit {
    fn resolve(
        self,
        name_path_map: &HashMap<String, PathBuf>,
    ) -> Result<BackendWithCommit, BackendWithCommitParsingError> {
        let repo_path: PathBuf = name_path_map
            .get(&self.repo)
            .cloned()
            .unwrap_or_else(|| PathBuf::from(&self.repo));
        let commit = CommitHash::new(&repo_path, self.tag.clone())?;
        Ok(BackendWithCommit {
            backend_name: self.backend_name,
            commit,
            tag: self.tag,
        })
    }
}

#[derive(Args, Debug)]
pub(crate) struct PlotCommand {
    pub benchmark_name: String,

    #[arg(short, long)]
    pub benchmark_setup: Option<BenchmarkSetup>,

    /// Select a backend at a specific commit to include in the plot.
    /// Format: BACKEND_NAME@REPO_OR_PATH[:REF]
    /// REF is a git tag, branch, or commit hash; defaults to HEAD if omitted.
    /// Repeat to overlay multiple backends and/or commits on the same chart.
    #[arg(long, value_name = "BACKEND@REPO[:REF]")]
    pub series: Vec<ParsableBackendWithCommit>,

    /// Path to save the plot image
    #[arg(short, long, value_name = "FILE_PATH")]
    pub output: Option<PathBuf>,

    // Type of plot to generate
    #[clap(subcommand)]
    pub plot_kind: InputPlotKind,
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum InputPlotKind {
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
        /// Specify which events should be displayed (eg. task-clock). The names of events are highly platform dependant.
        #[arg(short, long)]
        #[clap(required = true, value_delimiter=',', num_args(1..))]
        events: Vec<String>,
    },
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, clap::ValueEnum)]
pub(crate) enum InputVisKind {
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
        let series: Vec<BackendWithCommit> = self
            .series
            .into_iter()
            .map(|s| s.resolve(&aliasing_config.repo_path))
            .collect::<Result<Vec<_>, _>>()?;

        let default_output_name = match self.plot_kind {
            InputPlotKind::Series { .. } => "out.svg",
            InputPlotKind::FlameGraph { .. } => "out.html",
            InputPlotKind::PerfStat { .. } => "out.svg",
        };

        let output_file_name = self
            .output
            .map(|path| path.to_string_lossy().to_string())
            .unwrap_or_else(|| default_output_name.to_owned());
        Ok(Subcommands::Plot(PlotParams {
            benchmark_config: BenchmarkSetup::finalize(
                self.benchmark_setup,
                &self.benchmark_name,
                &aliasing_config,
            )?,
            series,
            plot_settings: PlotSettings::new(
                self.plot_kind.finalize(aliasing_config)?,
                output_file_name,
            ),
        }))
    }
}

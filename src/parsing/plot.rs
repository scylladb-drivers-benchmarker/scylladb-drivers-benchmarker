use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

use clap::Args;
use scylladb_drivers_benchmarker::BackendWithCommit;
use scylladb_drivers_benchmarker::commit_hash::{CommitHash, FailedToRetrieveCommitHash};
use scylladb_drivers_benchmarker::config::benchmark::{BenchmarkConfigList, BenchmarkData};
use scylladb_drivers_benchmarker::config::config_traits::ConfigurationList;
use scylladb_drivers_benchmarker::config::open_config;
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
    pub(crate) backend_name: String,
    pub(crate) repo: String,
    /// Git ref passed to `git rev-parse` (branch, tag, or commit hash).
    pub(crate) git_ref: String,
    /// Label shown on the plot. Defaults to `git_ref` unless `=ALIAS` was given.
    pub(crate) display_tag: String,
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
        let (repo, ref_with_alias) = match rest.split_once(':') {
            Some((repo, ref_part)) => {
                if repo.is_empty() {
                    return Err(BackendWithCommitParsingError::EmptyRepo);
                }
                (repo, if ref_part.is_empty() { "HEAD" } else { ref_part })
            }
            None => {
                if rest.is_empty() {
                    return Err(BackendWithCommitParsingError::EmptyRepo);
                }
                (rest, "HEAD")
            }
        };
        // Split off optional =ALIAS from the REF portion.
        // An explicit empty alias (REF=) means "show backend name only, no @..." suffix.
        let (git_ref, display_tag) = match ref_with_alias.split_once('=') {
            Some((r, alias)) => {
                let r = if r.is_empty() { "HEAD" } else { r };
                (r.to_owned(), alias.to_owned()) // alias may be empty string intentionally
            }
            None => (ref_with_alias.to_owned(), ref_with_alias.to_owned()),
        };
        Ok(ParsableBackendWithCommit {
            backend_name: backend_name.to_owned(),
            repo: repo.to_owned(),
            git_ref,
            display_tag,
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
        let commit = CommitHash::new(&repo_path, self.git_ref)?;
        Ok(BackendWithCommit {
            backend_name: self.backend_name,
            commit,
            tag: self.display_tag,
        })
    }
}

#[derive(Args, Debug)]
pub(crate) struct PlotCommand {
    /// Benchmark name to plot. If omitted, all benchmarks from the config are plotted in a grid.
    pub benchmark_name: Option<String>,

    #[arg(short, long)]
    pub benchmark_setup: Option<BenchmarkSetup>,

    /// Select a backend at a specific commit to include in the plot.
    /// Format: BACKEND_NAME@REPO_OR_PATH[:REF[=ALIAS]]
    /// REF is a git tag, branch, or commit hash; defaults to HEAD if omitted.
    /// ALIAS overrides the label shown on the plot (defaults to REF).
    /// Repeat to overlay multiple backends and/or commits on the same chart.
    #[arg(long, value_name = "BACKEND@REPO[:REF[=ALIAS]]")]
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
        let benchmarks: Vec<BenchmarkData> = if let Some(benchmark_name) = self.benchmark_name {
            vec![BenchmarkSetup::finalize(
                self.benchmark_setup,
                &benchmark_name,
                &aliasing_config,
            )?]
        } else {
            let config_path = match &self.benchmark_setup {
                Some(BenchmarkSetup::Path(path)) => path.clone(),
                Some(BenchmarkSetup::Points(_)) => {
                    return Err(ParsingError::NoBenchmarkConfiguration);
                }
                None => aliasing_config
                    .benchmark_config
                    .clone()
                    .ok_or(ParsingError::NoBenchmarkConfiguration)?,
            };
            let config_list: BenchmarkConfigList = open_config(&config_path)?;
            config_list.configs().map(BenchmarkData::from).collect()
        };

        Ok(Subcommands::Plot(PlotParams {
            benchmarks,
            series,
            plot_settings: PlotSettings::new(
                self.plot_kind.finalize(aliasing_config)?,
                output_file_name,
            ),
        }))
    }
}

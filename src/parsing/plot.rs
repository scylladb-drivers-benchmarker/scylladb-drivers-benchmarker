use std::path::{Path, PathBuf};
use std::str::FromStr;

use clap::Args;
use scylladb_drivers_benchmarker::DriverWithCommit;
use scylladb_drivers_benchmarker::commit_hash::{CommitHash, FailedToRetrieveCommitHash};
use scylladb_drivers_benchmarker::config::benchmark::{BenchmarkConfigList, BenchmarkData};
use scylladb_drivers_benchmarker::config::config_traits::ConfigurationList;
use scylladb_drivers_benchmarker::config::open_config;
use scylladb_drivers_benchmarker::measurement::MeasurementMethod;
use scylladb_drivers_benchmarker::{PlotSettings, Quantity, Scale, VisKind};

use crate::parsing::benchmark_setup::BenchmarkSetup;
use crate::parsing::{ParsingError, Subcommands};
use crate::{PlotKind, PlotParams};

#[justerror::Error]
pub(crate) enum DriverWithCommitParsingError {
    #[error(desc = "expected format BACKEND@REPO[:REF] \u{2014} '@' separator is missing")]
    MissingAtSign,
    #[error(desc = "backend name cannot be empty")]
    EmptyBackendName,
    #[error(desc = "repository path cannot be empty")]
    EmptyRepo,
    HashResolutionFailed(#[from] Box<FailedToRetrieveCommitHash>),
}

#[derive(Debug, Clone)]
pub(crate) struct ParsableDriverWithCommit {
    pub(crate) driver_name: String,
    pub(crate) repo: String,
    /// Git ref passed to `git rev-parse` (branch, tag, or commit hash).
    pub(crate) git_ref: String,
    /// Label shown on the plot. Defaults to `git_ref` unless `=ALIAS` was given.
    pub(crate) display_tag: String,
}

impl FromStr for ParsableDriverWithCommit {
    type Err = DriverWithCommitParsingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (driver_name, rest) = s
            .split_once('@')
            .ok_or(DriverWithCommitParsingError::MissingAtSign)?;
        if driver_name.is_empty() {
            return Err(DriverWithCommitParsingError::EmptyBackendName);
        }
        let (repo, ref_with_alias) = match rest.split_once(':') {
            Some((repo, ref_part)) => {
                if repo.is_empty() {
                    return Err(DriverWithCommitParsingError::EmptyRepo);
                }
                (repo, if ref_part.is_empty() { "HEAD" } else { ref_part })
            }
            None => {
                // No REF part - an =ALIAS may still follow the repo/commit id.
                let (repo, alias) = match rest.split_once('=') {
                    Some((repo, alias)) => (repo, Some(alias)),
                    None => (rest, None),
                };
                if repo.is_empty() {
                    return Err(DriverWithCommitParsingError::EmptyRepo);
                }
                return Ok(ParsableDriverWithCommit {
                    driver_name: driver_name.to_owned(),
                    repo: repo.to_owned(),
                    git_ref: "HEAD".to_owned(),
                    display_tag: alias.unwrap_or("HEAD").to_owned(),
                });
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
        Ok(ParsableDriverWithCommit {
            driver_name: driver_name.to_owned(),
            repo: repo.to_owned(),
            git_ref,
            display_tag,
        })
    }
}

impl ParsableDriverWithCommit {
    /// If REPO is an existing directory, the REF is resolved with git inside it.
    /// Otherwise REPO itself is taken as the literal version identity stored in
    /// the database (a commit hash, `<hash>-dirty`, or `v<version>` for
    /// published drivers).
    fn resolve(self) -> Result<DriverWithCommit, DriverWithCommitParsingError> {
        let repo_path = Path::new(&self.repo);
        let (commit, tag) = if repo_path.is_dir() {
            (CommitHash::new(repo_path, self.git_ref)?, self.display_tag)
        } else {
            // Literal identity: default the label to a shortened id
            // instead of the meaningless "HEAD".
            let tag = if self.display_tag == "HEAD" {
                self.repo.chars().take(8).collect()
            } else {
                self.display_tag
            };
            (CommitHash::new_unchecked(self.repo.clone()), tag)
        };
        Ok(DriverWithCommit {
            driver_name: self.driver_name,
            commit,
            tag,
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
    pub series: Vec<ParsableDriverWithCommit>,

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

        /// What is plotted on the y axis: the measured time, or the throughput
        /// derived from it (drawn as grouped columns, one per series).
        #[arg(short, long, value_enum, default_value_t = InputQuantity::Time)]
        quantity: InputQuantity,

        /// Scaling of the y axis.
        #[arg(short, long, value_enum, default_value_t = InputScale::Linear)]
        visualization_kind: InputScale,
    },

    /// Generate a flame-graph plot
    FlameGraph {
        #[arg(short, long, value_name = "DIR")]
        artifacts_dir: Option<PathBuf>,

        #[arg(short, long, value_name = "DIR")]
        flame_repo: PathBuf,
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
pub(crate) enum InputQuantity {
    Time,
    Throughput,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, clap::ValueEnum)]
pub(crate) enum InputScale {
    Linear,
    Log,
}

impl InputQuantity {
    pub fn finalize(self) -> Quantity {
        match self {
            InputQuantity::Time => Quantity::Time,
            InputQuantity::Throughput => Quantity::Throughput,
        }
    }
}

impl InputScale {
    pub fn finalize(self) -> Scale {
        match self {
            InputScale::Linear => Scale::Linear,
            InputScale::Log => Scale::Log,
        }
    }
}

impl InputPlotKind {
    pub fn finalize(self) -> Result<PlotKind, ParsingError> {
        match self {
            InputPlotKind::Series {
                measurement_method,
                quantity,
                visualization_kind,
            } => Ok(PlotKind::Series {
                measurement_method,
                visualization_kind: VisKind::new(quantity.finalize(), visualization_kind.finalize()),
            }),
            InputPlotKind::FlameGraph {
                artifacts_dir,
                flame_repo,
            } => Ok(PlotKind::FlameGraph {
                artifacts_dir,
                flame_repo,
            }),
            InputPlotKind::PerfStat { events } => Ok(PlotKind::PerfStat { events }),
        }
    }
}

impl PlotCommand {
    pub fn finalize(self) -> Result<Subcommands, ParsingError> {
        let series: Vec<DriverWithCommit> = self
            .series
            .into_iter()
            .map(|s| s.resolve())
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
            )?]
        } else {
            let config_path = match &self.benchmark_setup {
                Some(BenchmarkSetup::Path(path)) => path.clone(),
                Some(BenchmarkSetup::Points(_)) | None => {
                    return Err(ParsingError::NoBenchmarkConfiguration);
                }
            };
            let config_list: BenchmarkConfigList = open_config(&config_path)?;
            config_list.configs().map(BenchmarkData::from).collect()
        };

        Ok(Subcommands::Plot(PlotParams {
            benchmarks,
            series,
            plot_settings: PlotSettings::new(self.plot_kind.finalize()?, output_file_name),
        }))
    }
}

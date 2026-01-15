use crate::MeasurementMethod;
use crate::OutputFormat;
use crate::PlotKind;
use crate::PlotParams;
use crate::RepoNameWithTags;
use crate::parsing::ParsingError;
use crate::parsing::Subcommands;
use crate::parsing::aliasing::AliasingConfig;
use clap::Args;
use scylladb_drivers_benchmarker::PlotSettings;
use scylladb_drivers_benchmarker::config::find_config;
use scylladb_drivers_benchmarker::repo_with_commits::RepoNameWithCommitsParsingError;
use scylladb_drivers_benchmarker::repo_with_commits::RepoPathWithCommits;
use scylladb_drivers_benchmarker::repo_with_commits::resolve_repo_tags;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Args, Debug)]
pub struct PlotCommand {
    pub benchmark_name: String,

    #[arg(short, long, default_value_t = MeasurementMethod::Time)]
    pub measurement_method: MeasurementMethod,

    #[arg(short, long, default_value = "./config.yml")]
    pub benchmark_config_path: PathBuf,

    /// The source of data for the plot
    #[arg(long, value_name = "REPOSITORY_PATH:TAG1,TAG2,...")]
    pub from: Vec<ParsableRepoNameWithTags>,

    /// Output format of the plot
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Png)]
    pub format: OutputFormat,

    /// Path to save the plot image
    #[arg(short, long, value_name = "FILE_PATH")]
    pub output: Option<PathBuf>,

    // Type of plot to generate
    #[clap(subcommand)]
    pub plot_kind: PlotKind,
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

impl PlotCommand {
    pub fn finalize(self, aliasing_config: AliasingConfig) -> Result<Subcommands, ParsingError> {
        let parsed: Vec<RepoNameWithTags> = self.from.into_iter().map(Into::into).collect();

        let resolved = parsed
            .iter()
            .map(|repo| resolve_repo_tags(repo.clone(), &aliasing_config.repo_path))
            .collect::<Result<Vec<RepoPathWithCommits>, _>>()?;

        let plot_settings = PlotSettings::new(
            self.plot_kind,
            self.format,
            self.output
                .as_deref()
                .and_then(std::path::Path::to_str)
                .unwrap_or("plot.png")
                .to_owned(),
        );

        Ok(Subcommands::Plot(PlotParams {
            measurement_method: self.measurement_method,
            benchmark_config: find_config(&self.benchmark_name, &self.benchmark_config_path)?,
            from: parsed,
            resolved,
            plot_settings,
        }))
    }
}

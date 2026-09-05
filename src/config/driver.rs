//! The driver under test: either a local repository containing a
//! `benchmark-config.yml`, or a published package specified on the CLI as
//! `published:<api>:<package>@<version>`.

use std::path::{Path, PathBuf};

use fs_err as fs;
use log::warn;
use serde::{Deserialize, Serialize};

use crate::cmd;
use crate::commit_hash::{CommitHash, FailedToRetrieveCommitHash};

pub const DRIVER_CONFIG_FILE: &str = "benchmark-config.yml";

/// Contents of `benchmark-config.yml` at the root of a driver repository.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct DriverRepoConfig {
    /// Identity of the driver in the results database and on plots.
    pub driver_name: String,
    /// Driver API - selects `apis/<api>/` in the benchmarks repository.
    pub api: String,
    /// Package name (crate / npm package) as imported by the workloads.
    pub package: String,
    /// Relative path within the repository to the package named above
    /// (crate directory for Rust, npm package root for Node.js).
    /// Optional - defaults to the repository root.
    #[serde(default)]
    pub package_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriverSource {
    /// A local repository checkout (branch benchmarking).
    Path { repo: PathBuf },
    /// A published package version (release benchmarking).
    Published { version: String },
}

/// Fully resolved description of the driver under test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriverSpec {
    pub name: String,
    pub api: String,
    pub package: String,
    pub package_path: Option<String>,
    pub source: DriverSource,
}

#[justerror::Error(desc = "failed to resolve the driver under test")]
pub enum DriverSpecError {
    #[error(desc = "cannot read {file} in the driver repository at {repo}: {source}")]
    ConfigUnreadable {
        repo: PathBuf,
        file: &'static str,
        source: std::io::Error,
    },
    ConfigInvalid {
        repo: PathBuf,
        source: serde_yml::Error,
    },
    #[error(
        desc = "invalid published driver spec {spec:?} - expected published:<api>:<package>@<version>"
    )]
    InvalidPublishedSpec { spec: String },
    CommitResolution(#[from] Box<FailedToRetrieveCommitHash>),
}

impl DriverSpec {
    /// Reads `benchmark-config.yml` from a local driver repository.
    pub fn from_repo(repo: &Path) -> Result<Self, DriverSpecError> {
        let config_path = repo.join(DRIVER_CONFIG_FILE);
        let contents =
            fs::read(&config_path).map_err(|source| DriverSpecError::ConfigUnreadable {
                repo: repo.to_owned(),
                file: DRIVER_CONFIG_FILE,
                source,
            })?;
        let config: DriverRepoConfig =
            serde_yml::from_slice(&contents).map_err(|source| DriverSpecError::ConfigInvalid {
                repo: repo.to_owned(),
                source,
            })?;
        Ok(DriverSpec {
            name: config.driver_name,
            api: config.api,
            package: config.package,
            package_path: config.package_path,
            source: DriverSource::Path {
                repo: repo.to_owned(),
            },
        })
    }

    /// Parses a `published:<api>:<package>@<version>` CLI spec.
    pub fn from_published_spec(spec: &str) -> Result<Self, DriverSpecError> {
        let invalid = || DriverSpecError::InvalidPublishedSpec {
            spec: spec.to_owned(),
        };
        let rest = spec.strip_prefix("published:").ok_or_else(invalid)?;
        let (api, package_at_version) = rest.split_once(':').ok_or_else(invalid)?;
        let (package, version) = package_at_version.split_once('@').ok_or_else(invalid)?;
        if api.is_empty() || package.is_empty() || version.is_empty() {
            return Err(invalid());
        }
        Ok(DriverSpec {
            name: package.to_owned(),
            api: api.to_owned(),
            package: package.to_owned(),
            package_path: None,
            source: DriverSource::Published {
                version: version.to_owned(),
            },
        })
    }

    /// Identity of the driver version in the results database: the commit hash
    /// of the driver repository (with a `-dirty` suffix for an unclean working
    /// tree), or the version string for published drivers.
    pub fn commit_id(&self) -> Result<CommitHash, DriverSpecError> {
        match &self.source {
            DriverSource::Path { repo } => {
                let hash = CommitHash::new(repo, "HEAD".to_owned())?;
                let dirty = !cmd!("git", "status", "--porcelain")
                    .with_cwd(repo)
                    .process()
                    .output()
                    .map(|o| o.stdout.is_empty())
                    .unwrap_or(true);
                if dirty {
                    warn!(
                        "Driver repository {} has uncommitted changes - results will be recorded as {}-dirty",
                        repo.display(),
                        hash
                    );
                    Ok(CommitHash::new_unchecked(format!("{hash}-dirty")))
                } else {
                    Ok(hash)
                }
            }
            DriverSource::Published { version } => {
                Ok(CommitHash::new_unchecked(format!("v{version}")))
            }
        }
    }

    /// Environment variables consumed by the per-API `env_prepare.sh` scripts.
    pub fn env_vars(&self) -> Vec<(String, String)> {
        let mut envs = vec![("DRIVER_PACKAGE".to_owned(), self.package.clone())];
        match &self.source {
            DriverSource::Path { repo } => {
                envs.push(("DRIVER_SOURCE".to_owned(), "path".to_owned()));
                envs.push((
                    "DRIVER_PATH".to_owned(),
                    repo.to_string_lossy().into_owned(),
                ));
                if let Some(path) = &self.package_path {
                    envs.push(("DRIVER_PACKAGE_PATH".to_owned(), path.clone()));
                }
            }
            DriverSource::Published { version } => {
                envs.push(("DRIVER_SOURCE".to_owned(), "published".to_owned()));
                envs.push(("DRIVER_VERSION".to_owned(), version.clone()));
            }
        }
        envs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_published_spec() {
        let spec = DriverSpec::from_published_spec("published:nodejs:cassandra-driver@4.8.0")
            .unwrap();
        assert_eq!(spec.name, "cassandra-driver");
        assert_eq!(spec.api, "nodejs");
        assert_eq!(spec.package, "cassandra-driver");
        assert_eq!(
            spec.source,
            DriverSource::Published {
                version: "4.8.0".to_owned()
            }
        );
        assert_eq!(spec.commit_id().unwrap().as_str(), "v4.8.0");
    }

    #[test]
    fn parse_published_spec_invalid() {
        for s in [
            "published:nodejs:cassandra-driver",
            "published:cassandra-driver@4.8.0",
            "nodejs:cassandra-driver@4.8.0",
            "published::x@1",
        ] {
            assert!(DriverSpec::from_published_spec(s).is_err(), "{s}");
        }
    }

    #[test]
    fn parse_repo_config() {
        let config: DriverRepoConfig = serde_yml::from_str(
            "driver-name: rust-driver\napi: rust-v1\npackage: scylla\npackage-path: scylla\n",
        )
        .unwrap();
        assert_eq!(config.driver_name, "rust-driver");
        assert_eq!(config.api, "rust-v1");
        assert_eq!(config.package, "scylla");
        assert_eq!(config.package_path.as_deref(), Some("scylla"));
    }
}

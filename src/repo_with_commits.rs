use std::{collections::HashMap, path::PathBuf, str::FromStr};

use scylladb_drivers_benchmarker::{
    commit_hash::{self, CommitHash},
    utilities::{RepoNameWithTags, RepoPathWithCommits},
};

#[justerror::Error]
pub enum RepoNameWithCommitsParsingError {
    PathNotSupplied,
    HashResolutionFailed(#[from] commit_hash::errors::Error),
    Infallible(#[from] std::convert::Infallible),
}

#[derive(Debug, Clone)]
pub struct ParsableRepoNameWithTags {
    value: RepoNameWithTags,
}

impl From<ParsableRepoNameWithTags> for RepoNameWithTags {
    fn from(value: ParsableRepoNameWithTags) -> Self {
        value.value
    }
}

impl FromStr for ParsableRepoNameWithTags {
    type Err = RepoNameWithCommitsParsingError;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let (repo_names_str, tags_str) = string
            .split_once(':')
            .ok_or(RepoNameWithCommitsParsingError::PathNotSupplied)?;

        Ok(ParsableRepoNameWithTags {
            value: RepoNameWithTags {
                name: repo_names_str.to_owned(),
                tags: tags_str.split(',').map(str::to_owned).collect(),
            },
        })
    }
}

pub fn resolve_repo_tags(
    name_with_tags: RepoNameWithTags,
    name_path_map: &HashMap<String, PathBuf>,
) -> Result<RepoPathWithCommits, RepoNameWithCommitsParsingError> {
    let repo_path: PathBuf = match name_path_map.get(&name_with_tags.name) {
        Some(path) => path.clone(),
        None => name_with_tags.name.into(),
    };

    let git_hashes = name_with_tags
        .tags
        .into_iter()
        .map(|commit| CommitHash::new(&repo_path, commit))
        .collect::<Result<Vec<CommitHash>, commit_hash::errors::Error>>()?;

    Ok(RepoPathWithCommits {
        repo_path,
        git_hashes,
    })
}

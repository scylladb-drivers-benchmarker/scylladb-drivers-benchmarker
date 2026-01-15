use std::{collections::HashMap, path::PathBuf};

use crate::commit_hash::CommitHash;
use crate::commit_hash::FailedToRetrieveCommitHash;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoPathWithCommits {
    pub repo_path: PathBuf,
    pub git_hashes: Vec<CommitHash>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoNameWithTags {
    pub name: String,
    pub tags: Vec<String>,
}

#[justerror::Error]
pub enum RepoNameWithCommitsParsingError {
    PathNotSupplied,
    HashResolutionFailed(#[from] Box<FailedToRetrieveCommitHash>),
    Infallible(#[from] std::convert::Infallible),
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
        .collect::<Result<Vec<CommitHash>, Box<FailedToRetrieveCommitHash>>>()?;

    Ok(RepoPathWithCommits {
        repo_path,
        git_hashes,
    })
}

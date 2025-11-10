use anyhow::{Context, Result};
use directories::ProjectDirs;

pub fn project_dirs() -> Result<ProjectDirs> {
    directories::ProjectDirs::from("io", "ja2-stracciatella", "stracciatella-toolset")
        .context("could not determine home directory")
}

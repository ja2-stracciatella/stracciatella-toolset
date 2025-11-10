use anyhow::{Context, Result};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

use crate::dirs;

pub fn get_json_cache_dir() -> Result<PathBuf> {
    Ok(dirs::project_dirs()?
        .cache_dir()
        .join("stracciatella-json-cache"))
}

pub fn update_json_cache() -> Result<()> {
    let json_cache_dir = get_json_cache_dir().context("failed to get JSON cache dir")?;
    let json_cache_version_path = json_cache_dir.join("current-version");
    let json_cache_tmp_zip_path = json_cache_dir.join("current-archive.zip");
    let current_cache_version =
        fs::read_to_string(&json_cache_version_path).unwrap_or_else(|_| "".to_owned());
    let expected_cache_version = env!("CARGO_PKG_VERSION");
    if current_cache_version != expected_cache_version {
        log::info!(
            "Updating JSON cache from {} to {}",
            current_cache_version,
            expected_cache_version
        );

        let externalized_dir = json_cache_dir.join("externalized");

        // Create the directory if it doesn't exist
        fs::create_dir_all(&json_cache_dir).context("failed to create cache directory")?;
        if let Err(e) = fs::remove_dir_all(&externalized_dir) {
            if e.kind() != std::io::ErrorKind::NotFound {
                return Err(e).context("failed to remove existing externalized directory");
            }
        }
        fs::create_dir_all(&externalized_dir).context("failed to create externalized directory")?;

        // Download the latest JSON cache
        let release_version = format!(
            "{}.{}.{}",
            env!("CARGO_PKG_VERSION_MAJOR"),
            env!("CARGO_PKG_VERSION_MINOR"),
            0
        );
        let release_url = format!(
            "https://github.com/ja2-stracciatella/ja2-stracciatella/archive/refs/tags/v{}.zip",
            release_version
        );
        let download_context = format!(
            "failed to fetch stracciatella release from {}",
            &release_url
        );
        let mut z = download_temporary_zip_to_path(&release_url, &json_cache_tmp_zip_path)
            .context(download_context)?;
        let root_dir = z
            .root_dir(zip::read::root_dir_common_filter)
            .context("failed to get root dir")?
            .context("empty root dir")?;
        let externalized_zip_dir = root_dir
            .join("assets/externalized/")
            .to_string_lossy()
            .to_string();
        let externalized_files: Vec<_> = z
            .file_names()
            .filter_map(|entry| {
                if entry.starts_with(&externalized_zip_dir) {
                    Some(entry.to_owned())
                } else {
                    None
                }
            })
            .collect();

        for entry in &externalized_files {
            let mut src = z.by_name(entry).context("failed to get zip entry")?;
            if src.is_dir() {
                continue;
            }
            let target = externalized_dir.join(
                &entry
                    .strip_prefix(&externalized_zip_dir)
                    .context("failed to stip prefix from zip entry")?,
            );

            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).context("failed to create directory while unzipping")?;
            }

            let mut f = fs::OpenOptions::new()
                .create(true)
                .write(true)
                .open(&target)
                .context("failed to open target file")?;

            io::copy(&mut src, &mut f).context("failed to extract file")?;
        }

        // Update the cache version
        fs::write(&json_cache_version_path, expected_cache_version)?;

        // Remove the temporary zip file
        fs::remove_file(&json_cache_tmp_zip_path).context("failed to remove temporary zip file")?;
    }

    Ok(())
}

fn download_temporary_zip_to_path(url: &str, path: &Path) -> Result<zip::ZipArchive<fs::File>> {
    let mut response = reqwest::blocking::get(url).context("failed to execute request")?;
    if !response.status().is_success() {
        return Err(anyhow::anyhow!("invalid status code {}", response.status()));
    }

    {
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .context("failed to open temporary zip path")?;
        io::copy(&mut response, &mut f).context("failed to download zip")?;
    }

    let f = fs::OpenOptions::new()
        .read(true)
        .open(&path)
        .context("failed to open file after download")?;
    Ok(zip::ZipArchive::new(f).context("failed to open zip")?)
}

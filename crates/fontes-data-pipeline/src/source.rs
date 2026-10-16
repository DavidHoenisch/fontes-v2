use crate::{file_digest, source_asset};
use anyhow::{Context, Result, bail, ensure};
use fontes_data_contracts::LockedSource;
use reqwest::blocking::Client;
use serde_json::Value;
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};

pub fn client() -> Result<Client> {
    Ok(Client::builder()
        .https_only(true)
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(180))
        .user_agent("FontesDataPipeline/0.1 (https://github.com/DavidHoenisch/fontes-v2)")
        .build()?)
}

pub fn valid_repository(repository: &str) -> bool {
    let parts: Vec<_> = repository.split('/').collect();
    parts.len() == 2
        && parts.iter().all(|s| {
            !s.is_empty()
                && *s != "."
                && *s != ".."
                && s.bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c))
        })
}

fn download(
    client: &Client,
    url: &str,
    source: &LockedSource,
    cache: &Path,
    dest: &Path,
) -> Result<()> {
    let response = client.get(url).send()?.error_for_status()?;
    if let Some(length) = response.content_length() {
        ensure!(length == source.bytes, "source length changed");
    }
    let mut tmp = tempfile::NamedTempFile::new_in(cache)?;
    let count = std::io::copy(&mut response.take(source.bytes + 1), tmp.as_file_mut())?;
    ensure!(
        count == source.bytes,
        "{} source byte length changed",
        source.source_id
    );
    ensure!(
        file_digest(tmp.path())? == source.sha256,
        "{} source checksum changed; update the reviewed source lock",
        source.source_id
    );
    tmp.as_file().sync_all()?;
    tmp.persist(dest).map_err(|e| e.error)?;
    Ok(())
}

pub fn acquire(
    source: &LockedSource,
    cache: &Path,
    offline: bool,
    mirror: Option<&str>,
) -> Result<PathBuf> {
    fs::create_dir_all(cache)?;
    let path = cache.join(format!("{}.zip", source.sha256));
    if path.exists() {
        ensure!(
            path.metadata()?.len() == source.bytes && file_digest(&path)? == source.sha256,
            "corrupt source cache entry {}",
            source.source_id
        );
        return Ok(path);
    }
    ensure!(!offline, "offline cache miss for {}", source.source_id);
    let client = client()?;
    let mut last_error = None;
    for _ in 0..3 {
        match download(&client, &source.url, source, cache, &path) {
            Ok(()) => return Ok(path),
            Err(error) => last_error = Some(error),
        }
    }
    if let Some(repository) = mirror {
        ensure!(valid_repository(repository), "invalid mirror repository");
        let asset_name = source_asset(source);
        for page in 1..=5 {
            let response = client
                .get(format!(
                    "https://api.github.com/repos/{repository}/releases?per_page=100&page={page}"
                ))
                .send()?
                .error_for_status()?;
            let releases: Vec<Value> = response.json()?;
            let more = releases.len() == 100;
            for release in releases {
                let tag = release["tag_name"].as_str().unwrap_or("");
                if tag
                    .strip_prefix("data-v1-stable-")
                    .is_none_or(|s| s.parse::<u64>().is_err())
                {
                    continue;
                }
                if release["assets"]
                    .as_array()
                    .context("missing release assets")?
                    .iter()
                    .any(|asset| asset["name"] == asset_name)
                {
                    let url = format!(
                        "https://github.com/{repository}/releases/download/{tag}/{asset_name}"
                    );
                    if download(&client, &url, source, cache, &path).is_ok() {
                        return Ok(path);
                    }
                }
            }
            if !more {
                break;
            }
        }
    }
    bail!(
        "could not acquire locked {}: {}",
        source.source_id,
        last_error.context("source fetch did not run")?
    )
}

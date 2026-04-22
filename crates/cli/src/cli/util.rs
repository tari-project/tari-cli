// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use std::{fs::Metadata, io, path::PathBuf};

use dialoguer::FuzzySelect;
use tari_ootle_common_types::Network;
use tokio::fs;

pub async fn create_dir(dir: &PathBuf) -> io::Result<()> {
    fs::create_dir_all(dir).await
}

pub async fn file_exists(file: &PathBuf) -> io::Result<bool> {
    Ok(fs::try_exists(file).await? && path_metadata(file).await?.is_file())
}

pub async fn dir_exists(dir: &PathBuf) -> io::Result<bool> {
    Ok(fs::try_exists(dir).await? && path_metadata(dir).await?.is_dir())
}

pub async fn path_metadata(path: &PathBuf) -> io::Result<Metadata> {
    fs::metadata(path).await
}

pub fn cli_select<'a, T: std::fmt::Display>(prompt: &str, items: &'a [T]) -> anyhow::Result<&'a T> {
    let selection = FuzzySelect::new()
        .with_prompt(prompt)
        .highlight_matches(true)
        .items(items)
        .interact()?;

    Ok(&items[selection])
}

pub fn human_bytes(n: usize) -> String {
    human_bytes::human_bytes(n as f64)
}

pub fn get_default_metadata_server_url(network: Network) -> Option<&'static str> {
    match network {
        Network::LocalNet => Some(crate::project::DEFAULT_METADATA_SERVER_URL_LOCALNET),
        Network::Esmeralda => Some(crate::project::DEFAULT_METADATA_SERVER_URL_ESMERALDA),
        _ => None,
    }
}

// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use std::{path::PathBuf, string::ToString};

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use tari_ootle_publish_lib::walletd_client::ComponentAddressOrName;
use tokio::{fs, io::AsyncWriteExt};

pub const VALID_OVERRIDE_KEYS: &[&str] = &[
    "template_repository.url",
    "template_repository.branch",
    "template_repository.folder",
    "default_account",
    "wallet_daemon_url",
];

/// CLI configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Config {
    pub template_repository: TemplateRepository,
    pub default_account: Option<ComponentAddressOrName>,
    /// Global default wallet daemon JSON-RPC URL.
    /// Used when no tari.config.toml is found in the project tree.
    pub wallet_daemon_url: Option<url::Url>,
}

/// Repository that holds templates to generate Tari template crates.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TemplateRepository {
    pub url: String,
    pub branch: String,
    pub folder: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            template_repository: TemplateRepository {
                url: "https://github.com/tari-project/wasm-template".to_string(),
                branch: "main".to_string(),
                folder: "wasm_templates".to_string(),
            },
            default_account: None,
            wallet_daemon_url: None,
        }
    }
}

impl Config {
    pub async fn open(path: &PathBuf) -> anyhow::Result<Self> {
        let content = fs::read_to_string(path).await?;
        Ok(toml::from_str(content.as_str())?)
    }

    pub async fn write_to_file(&self, path: &PathBuf) -> anyhow::Result<()> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(path)
            .await?;
        let content = toml::to_string(self)?;
        let _ = file.write(content.as_bytes()).await?;
        file.flush().await?;
        Ok(())
    }

    pub fn is_override_key_valid(key: &str) -> bool {
        VALID_OVERRIDE_KEYS.contains(&key)
    }

    pub fn override_data(&mut self, key: &str, value: &str) -> anyhow::Result<&mut Self> {
        if !Self::is_override_key_valid(key) {
            return Err(anyhow!("Invalid key: {}", key));
        }

        match key {
            "template_repository.url" => {
                self.template_repository.url = value.to_string();
            },
            "template_repository.branch" => {
                self.template_repository.branch = value.to_string();
            },
            "template_repository.folder" => {
                self.template_repository.folder = value.to_string();
            },
            "default_account" => {
                self.default_account = Some(value.parse()?);
            },
            "wallet_daemon_url" => {
                self.wallet_daemon_url = Some(value.parse().map_err(|e| anyhow!("Invalid URL: {e}"))?);
            },
            _ => {},
        }

        Ok(self)
    }
}

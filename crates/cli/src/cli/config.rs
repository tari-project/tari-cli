// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use std::collections::HashMap;
use std::{path::PathBuf, string::ToString};

use anyhow::anyhow;
use ootle_network::Network;
use serde::{Deserialize, Serialize};
use tari_ootle_publish_lib::walletd_client::ComponentAddressOrName;
use tokio::{fs, io::AsyncWriteExt};

use crate::project::{
    DEFAULT_METADATA_SERVER_URL_ESMERALDA, DEFAULT_METADATA_SERVER_URL_LOCALNET, DEFAULT_WALLET_DAEMON_URL,
};

pub const VALID_OVERRIDE_KEYS: &[&str] = &[
    "template_repository.url",
    "template_repository.branch",
    "template_repository.folder",
    "default_account",
    "default_network",
];

const VALID_NETWORK_OVERRIDE_FIELDS: &[&str] = &["wallet-daemon-url", "metadata-server-url"];

/// CLI configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Config {
    pub template_repository: TemplateRepository,
    pub default_account: Option<ComponentAddressOrName>,
    /// Default network used when no project config or CLI flag selects one.
    pub default_network: Option<Network>,
    /// Per-network defaults (wallet daemon URL, metadata server URL).
    #[serde(default)]
    pub networks: HashMap<Network, CliNetworkSettings>,
}

/// Per-network CLI defaults used when the project config is absent or does not
/// specify a value for the selected network.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CliNetworkSettings {
    pub wallet_daemon_url: Option<url::Url>,
    pub metadata_server_url: Option<url::Url>,
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
        let wallet_url =
            || Some(url::Url::parse(DEFAULT_WALLET_DAEMON_URL).expect("default wallet daemon URL is valid"));
        let metadata_url = |s: &str| Some(url::Url::parse(s).expect("default metadata server URL is valid"));
        let mut networks = HashMap::new();
        networks.insert(
            Network::Esmeralda,
            CliNetworkSettings {
                wallet_daemon_url: wallet_url(),
                metadata_server_url: metadata_url(DEFAULT_METADATA_SERVER_URL_ESMERALDA),
            },
        );
        networks.insert(
            Network::LocalNet,
            CliNetworkSettings {
                wallet_daemon_url: wallet_url(),
                metadata_server_url: metadata_url(DEFAULT_METADATA_SERVER_URL_LOCALNET),
            },
        );
        Self {
            template_repository: TemplateRepository {
                url: "https://github.com/tari-project/wasm-template".to_string(),
                branch: "main".to_string(),
                folder: "wasm_templates".to_string(),
            },
            default_account: None,
            default_network: Some(Network::Esmeralda),
            networks,
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
        if VALID_OVERRIDE_KEYS.contains(&key) {
            return true;
        }
        // networks.<net>.<wallet-daemon-url|metadata-server-url>
        let parts: Vec<&str> = key.split('.').collect();
        if parts.len() == 3 && parts[0] == "networks" {
            return parts[1].parse::<Network>().is_ok() && VALID_NETWORK_OVERRIDE_FIELDS.contains(&parts[2]);
        }
        false
    }

    pub fn wallet_daemon_url(&self, network: Network) -> Option<&url::Url> {
        self.networks.get(&network).and_then(|n| n.wallet_daemon_url.as_ref())
    }

    pub fn metadata_server_url(&self, network: Network) -> Option<&url::Url> {
        self.networks.get(&network).and_then(|n| n.metadata_server_url.as_ref())
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
            "default_network" => {
                self.default_network = Some(value.parse().map_err(|e| anyhow!("Invalid network: {e}"))?);
            },
            _ => self.apply_network_override(key, value)?,
        }

        Ok(self)
    }

    fn apply_network_override(&mut self, key: &str, value: &str) -> anyhow::Result<()> {
        let parts: Vec<&str> = key.split('.').collect();
        // is_override_key_valid already enforced shape and Network parse.
        let network: Network = parts[1].parse().map_err(|e| anyhow!("Invalid network: {e}"))?;
        let entry = self.networks.entry(network).or_default();
        let url: url::Url = value.parse().map_err(|e| anyhow!("Invalid URL: {e}"))?;
        match parts[2] {
            "wallet-daemon-url" => entry.wallet_daemon_url = Some(url),
            "metadata-server-url" => entry.metadata_server_url = Some(url),
            other => return Err(anyhow!("Unknown network field: {other}")),
        }
        Ok(())
    }
}

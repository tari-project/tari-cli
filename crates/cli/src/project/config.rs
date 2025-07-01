// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use serde::{Deserialize, Serialize};
use tari_deploy::NetworkConfig;
use tari_wallet_daemon_client::ComponentAddressOrName;
use thiserror::Error;
use url::Url;

#[derive(Error, Debug)]
pub enum Error {
    #[error("URL parsing error: {0}")]
    Parse(#[from] url::ParseError),
}

/// Project configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectConfig {
    network: NetworkConfig,
    default_account: Option<String>,
}

impl ProjectConfig {
    pub fn network(&self) -> &NetworkConfig {
        &self.network
    }

    pub fn parsed_default_account(&self) -> anyhow::Result<Option<ComponentAddressOrName>> {
        let acc = self
            .default_account
            .as_ref()
            .map(|s| s.parse())
            .transpose()?;
        Ok(acc)
    }
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            network: NetworkConfig::new(Url::parse("http://127.0.0.1:9000").unwrap()),
            default_account: None,
        }
    }
}

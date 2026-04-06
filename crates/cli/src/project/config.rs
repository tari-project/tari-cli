// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use serde::{Deserialize, Serialize};
use tari_ootle_publish_lib::NetworkConfig;
use tari_ootle_publish_lib::walletd_client::ComponentAddressOrName;
use url::Url;

/// Project configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectConfig {
    network: NetworkConfig,
    default_account: Option<String>,
    /// Metadata server URL for publishing template metadata.
    metadata_server_url: Option<url::Url>,
}

impl ProjectConfig {
    pub fn network(&self) -> &NetworkConfig {
        &self.network
    }

    pub fn set_wallet_daemon_url(&mut self, url: Url) {
        self.network = NetworkConfig::new(url);
    }

    pub fn metadata_server_url(&self) -> Option<&url::Url> {
        self.metadata_server_url.as_ref()
    }

    pub fn parsed_default_account(&self) -> anyhow::Result<Option<ComponentAddressOrName>> {
        let acc = self.default_account.as_ref().map(|s| s.parse()).transpose()?;
        Ok(acc)
    }
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            network: NetworkConfig::new(Url::parse("http://127.0.0.1:9000/json_rpc").unwrap()),
            default_account: None,
            metadata_server_url: None,
        }
    }
}

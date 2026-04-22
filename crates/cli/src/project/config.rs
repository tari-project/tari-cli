// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tari_engine_types::published_template::PublishedTemplateAddress;
use tari_ootle_common_types::Network;
use tari_ootle_publish_lib::walletd_client::ComponentAddressOrName;
use url::Url;

pub const DEFAULT_WALLET_DAEMON_URL: &str = "http://127.0.0.1:5100/json_rpc";
pub const DEFAULT_METADATA_SERVER_URL_ESMERALDA: &str = "https://ootle-templates-esme.tari.com/";
pub const DEFAULT_METADATA_SERVER_URL_LOCALNET: &str = "http://localhost:3000";

/// Project configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ProjectConfig {
    default_network: Option<Network>,
    default_account: Option<String>,
    #[serde(default)]
    networks: HashMap<Network, ProjectNetworkSettings>,
}

/// Per-network project settings.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ProjectNetworkSettings {
    pub wallet_daemon_url: Option<Url>,
    pub metadata_server_url: Option<Url>,
    pub template_address: Option<PublishedTemplateAddress>,
}

impl ProjectConfig {
    pub fn default_network(&self) -> Option<Network> {
        self.default_network
    }

    pub fn wallet_daemon_url(&self, network: Network) -> Option<&Url> {
        self.networks.get(&network).and_then(|n| n.wallet_daemon_url.as_ref())
    }

    pub fn metadata_server_url(&self, network: Network) -> Option<&Url> {
        self.networks.get(&network).and_then(|n| n.metadata_server_url.as_ref())
    }

    pub fn template_address(&self, network: Network) -> Option<&PublishedTemplateAddress> {
        self.networks.get(&network).and_then(|n| n.template_address.as_ref())
    }

    pub fn parsed_default_account(&self) -> anyhow::Result<Option<ComponentAddressOrName>> {
        let acc = self.default_account.as_ref().map(|s| s.parse()).transpose()?;
        Ok(acc)
    }
}

impl Default for ProjectConfig {
    fn default() -> Self {
        let wallet_url = || Some(Url::parse(DEFAULT_WALLET_DAEMON_URL).expect("default wallet daemon URL is valid"));
        let metadata_url = |s: &str| Some(Url::parse(s).expect("default metadata server URL is valid"));
        let mut networks = HashMap::new();
        networks.insert(
            Network::Esmeralda,
            ProjectNetworkSettings {
                wallet_daemon_url: wallet_url(),
                metadata_server_url: metadata_url(DEFAULT_METADATA_SERVER_URL_ESMERALDA),
                template_address: None,
            },
        );
        networks.insert(
            Network::LocalNet,
            ProjectNetworkSettings {
                wallet_daemon_url: wallet_url(),
                metadata_server_url: metadata_url(DEFAULT_METADATA_SERVER_URL_LOCALNET),
                template_address: None,
            },
        );
        Self {
            default_network: Some(Network::Esmeralda),
            default_account: None,
            networks,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_project_config_roundtrips() {
        let cfg = ProjectConfig::default();
        let ser = toml::to_string(&cfg).expect("serialize default");
        let de: ProjectConfig = toml::from_str(&ser).expect("deserialize default");
        assert_eq!(de.default_network(), Some(Network::Esmeralda));
        assert_eq!(
            de.wallet_daemon_url(Network::Esmeralda).map(|u| u.as_str()),
            Some(DEFAULT_WALLET_DAEMON_URL)
        );
    }

    #[test]
    fn per_network_sections_parse() {
        let toml_str = r#"
default-network = "esmeralda"

[networks.esmeralda]
wallet-daemon-url = "http://localhost:5100/json_rpc"
template-address = "template_0000000000000000000000000000000000000000000000000000000000000000"

[networks.localnet]
wallet-daemon-url = "http://localhost:9999/json_rpc"
"#;
        let cfg: ProjectConfig = toml::from_str(toml_str).expect("parse");
        assert_eq!(cfg.default_network(), Some(Network::Esmeralda));
        assert_eq!(
            cfg.wallet_daemon_url(Network::Esmeralda).map(|u| u.as_str()),
            Some("http://localhost:5100/json_rpc")
        );
        assert_eq!(
            cfg.wallet_daemon_url(Network::LocalNet).map(|u| u.as_str()),
            Some("http://localhost:9999/json_rpc")
        );
        assert!(cfg.template_address(Network::Esmeralda).is_some());
        assert!(cfg.template_address(Network::LocalNet).is_none());
    }
}

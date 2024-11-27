// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use crate::cli::arguments;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tari_deploy::{NetworkConfig, WalletGrpcAuthConfig, WalletGrpcConfig};
use thiserror::Error;
use url::Url;

#[derive(Error, Debug)]
pub enum Error {
    #[error("URL parsing error: {0}")]
    Parse(#[from] url::ParseError),
}

/// Project configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    networks: HashMap<String, NetworkConfig>,
}

impl Config {
    pub fn find_network_config(&self, name: &str) -> Option<&NetworkConfig> {
        self.networks.get(name)
    }
}

// TODO: add other networks when available
impl Default for Config {
    fn default() -> Self {
        Self {
            networks: HashMap::from([(
                arguments::Network::Local.to_string(),
                NetworkConfig::new(
                    WalletGrpcConfig::new(
                        Url::parse("http://127.0.0.1:12003").unwrap(),
                        WalletGrpcAuthConfig::default(),
                    ),
                    Url::parse("http://127.0.0.1:12009").unwrap(),
                    Url::parse("http://127.0.0.1:8080/upload_template?register_template=false")
                        .unwrap(),
                ),
            )]),
        }
    }
}

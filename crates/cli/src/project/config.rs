// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use serde::{Deserialize, Serialize};
use tari_deploy::NetworkConfig;
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
    network: NetworkConfig,
}

impl Config {
    pub fn network(&self) -> &NetworkConfig {
        &self.network
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            network: NetworkConfig::new(Url::parse("http://127.0.0.1:9000").unwrap()),
        }
    }
}

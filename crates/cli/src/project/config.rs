use serde::{Deserialize, Serialize};
use tari_deploy::NetworkConfig;

// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause
/// Project configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub network: NetworkConfig,
}


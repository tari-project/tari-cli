// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use serde::{Deserialize, Serialize};
use tari_utilities::Hidden;
use url::Url;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct NetworkConfig {
    /// HTTP address of Tari Layer-2 wallet daemon's JRPC (JSON-RPC) endpoint.
    /// Example: http://127.0.0.1:12047
    wallet_daemon_jrpc_address: Url,
    /// API key used to authenticate with the wallet daemon. Sent as a bearer
    /// token on every JSON-RPC request. `None` sends no `Authorization` header,
    /// which only works against a daemon with authentication disabled.
    //
    // Wrapped in `Hidden` so it is zeroized from memory on drop and never
    // exposed through `Debug`. Never persisted either: `#[serde(skip)]` keeps
    // the secret out of any config file written to disk.
    #[serde(skip)]
    api_key: Option<Hidden<String>>,
}

impl NetworkConfig {
    pub fn new(wallet_daemon_jrpc_address: Url) -> Self {
        Self {
            wallet_daemon_jrpc_address,
            api_key: None,
        }
    }

    /// Sets the wallet daemon API key used for authentication.
    pub fn with_api_key(mut self, api_key: Option<Hidden<String>>) -> Self {
        self.api_key = api_key;
        self
    }

    pub fn wallet_daemon_jrpc_address(&self) -> &Url {
        &self.wallet_daemon_jrpc_address
    }

    pub fn api_key(&self) -> Option<&str> {
        self.api_key.as_ref().map(|key| key.reveal().as_str())
    }
}

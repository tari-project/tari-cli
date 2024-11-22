// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Name of the network.
    name: String,

    /// HTTP address of Tari Layer-1 wallet's gRPC endpoint.
    /// Example: http://127.0.0.1:12003
    wallet_grpc_address: Url,

    /// HTTP address of Tari Layer-2 wallet daemon's JRPC (JSON-RPC) endpoint.
    /// Example: http://127.0.0.1:12047
    wallet_daemon_jrpc_address: Url,
}

impl NetworkConfig {
    pub fn new(
        name: String,
        wallet_grpc_address: Url,
        wallet_daemon_jrpc_address: Url,
    ) -> Self {
        Self {
            name,
            wallet_grpc_address,
            wallet_daemon_jrpc_address,
        }
    }
    pub fn wallet_grpc_address(&self) -> &Url {
        &self.wallet_grpc_address
    }

    pub fn wallet_daemon_jrpc_address(&self) -> &Url {
        &self.wallet_daemon_jrpc_address
    }
}
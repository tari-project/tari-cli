// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// HTTP address of Tari Layer-1 wallet's gRPC endpoint.
    /// Example: http://127.0.0.1:12003
    wallet_grpc_address: Url,

    /// HTTP address of Tari Layer-2 wallet daemon's JRPC (JSON-RPC) endpoint.
    /// Example: http://127.0.0.1:12047
    wallet_daemon_jrpc_address: Url,

    /// HTTP endpoint to upload compiled templates
    /// Example: http://127.0.0.1:8080/upload_template
    uploader_endpoint: Url,
}

impl NetworkConfig {
    pub fn new(
        wallet_grpc_address: Url,
        wallet_daemon_jrpc_address: Url,
        uploader_endpoint: Url,
    ) -> Self {
        Self {
            wallet_grpc_address,
            wallet_daemon_jrpc_address,
            uploader_endpoint,
        }
    }
    pub fn wallet_grpc_address(&self) -> &Url {
        &self.wallet_grpc_address
    }

    pub fn wallet_daemon_jrpc_address(&self) -> &Url {
        &self.wallet_daemon_jrpc_address
    }

    pub fn uploader_endpoint(&self) -> &Url {
        &self.uploader_endpoint
    }
}
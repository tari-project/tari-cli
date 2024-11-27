// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tari_common_types::grpc_authentication::GrpcAuthentication;
use tari_utilities::SafePassword;
use thiserror::Error;
use url::Url;

/// Configuration errors.
#[derive(Error, Debug)]
pub enum Error {
    #[error("Safe password conversion error: {0}")]
    SafePasswordConversion(String),
}

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub enum WalletGrpcAuthConfig {
    #[default]
    None,
    Basic {
        username: String,
        password: String,
    },
}

impl TryFrom<&WalletGrpcAuthConfig> for GrpcAuthentication {
    type Error = Error;

    fn try_from(config: &WalletGrpcAuthConfig) -> Result<Self, Self::Error> {
        Ok(match config {
            WalletGrpcAuthConfig::None => GrpcAuthentication::None,
            WalletGrpcAuthConfig::Basic { username, password } => GrpcAuthentication::Basic {
                username: username.clone(),
                password: SafePassword::from_str(password.as_str())
                    .map_err(Error::SafePasswordConversion)?,
            },
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WalletGrpcConfig {
    /// HTTP address of gRPC endpoint.
    /// Example: http://127.0.0.1:12003
    address: Url,

    /// Authentication method to use when connecting to wallet's gRPC.
    authentication: WalletGrpcAuthConfig,
}

impl WalletGrpcConfig {
    pub fn new(address: Url, authentication: WalletGrpcAuthConfig) -> Self {
        Self {
            address,
            authentication,
        }
    }

    pub fn address(&self) -> &Url {
        &self.address
    }

    pub fn authentication(&self) -> &WalletGrpcAuthConfig {
        &self.authentication
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Configuration for Tari Layer-1 wallet's gRPC service.
    wallet_grpc_config: WalletGrpcConfig,

    /// HTTP address of Tari Layer-2 wallet daemon's JRPC (JSON-RPC) endpoint.
    /// Example: http://127.0.0.1:12047
    wallet_daemon_jrpc_address: Url,

    /// HTTP endpoint to upload compiled templates
    /// Example: http://127.0.0.1:8080/upload_template
    uploader_endpoint: Url,
}

impl NetworkConfig {
    pub fn new(
        wallet_grpc_config: WalletGrpcConfig,
        wallet_daemon_jrpc_address: Url,
        uploader_endpoint: Url,
    ) -> Self {
        Self {
            wallet_grpc_config,
            wallet_daemon_jrpc_address,
            uploader_endpoint,
        }
    }

    pub fn wallet_daemon_jrpc_address(&self) -> &Url {
        &self.wallet_daemon_jrpc_address
    }

    pub fn uploader_endpoint(&self) -> &Url {
        &self.uploader_endpoint
    }

    pub fn wallet_grpc_config(&self) -> &WalletGrpcConfig {
        &self.wallet_grpc_config
    }
}

// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use crate::{config, uploader};
use minotari_app_grpc::authentication::BasicAuthError;
use std::io;
use tari_core::transactions::tari_amount::MicroMinotari;
use tari_dan_engine::template::TemplateLoaderError;
use tari_template_lib::HashParseError;
use tari_wallet_daemon_client::error::WalletDaemonClientError;
use thiserror::Error;

/// Possible errors for [`crate::TemplateDeployer`].
#[derive(Error, Debug)]
pub enum Error {
    #[error("Tonic transport error: {0}")]
    TonicTransport(#[from] tonic::transport::Error),
    #[error("Tari gRPC basic auth error: {0}")]
    TariGrpcBasicAuth(#[from] BasicAuthError),
    #[error("Wallet daemon client error: {0}")]
    WalletDaemonClient(#[from] WalletDaemonClientError),
    #[error("gRPC error: {0}")]
    Grpc(#[from] tonic::Status),
    #[error("Invalid template: {0}")]
    InvalidTemplate(#[from] TemplateLoaderError),
    #[error("Invalid template: {0}")]
    IO(#[from] io::Error),
    #[error("Template binary uploader error: {0}")]
    Uploader(#[from] uploader::Error),
    #[error("Invalid hash error: {0}")]
    InvalidHash(#[from] HashParseError),
    #[error("Config error: {0}")]
    Config(#[from] config::Error),
    #[error("Insufficient balance in Tari L1 wallet! Current balance: {0}, Estimated Fee: {1}")]
    InsufficientBalance(MicroMinotari, MicroMinotari),
}

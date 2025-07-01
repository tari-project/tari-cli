// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use std::io;
use tari_engine::template::TemplateLoaderError;
use tari_template_lib::models::Amount;
use tari_template_lib::types::HashParseError;
use tari_wallet_daemon_client::error::WalletDaemonClientError;
use thiserror::Error;

/// Possible errors for [`crate::TemplateDeployer`].
#[derive(Error, Debug)]
pub enum Error {
    #[error("Wallet daemon client error: {0}")]
    WalletDaemonClient(#[from] WalletDaemonClientError),
    #[error("gRPC error: {0}")]
    Grpc(#[from] tonic::Status),
    #[error("Invalid template: {0}")]
    InvalidTemplate(#[from] TemplateLoaderError),
    #[error("Invalid template: {0}")]
    IO(#[from] io::Error),
    #[error("Invalid hash error: {0}")]
    InvalidHash(#[from] HashParseError),
    #[error(
        "Insufficient balance in Tari L2 wallet! Current balance: {current}, Estimated Fee: {fee}"
    )]
    InsufficientBalance { current: Amount, fee: Amount },
    #[error("Waiting for transaction timed out! Transaction ID: {0}")]
    WaitForTransactionTimeout(String),
    #[error("Invalid transaction: {0}!\n{1}")]
    InvalidTransaction(String, String),
    #[error("Missing transaction result: {0}")]
    MissingTransactionResult(String),
    #[error("Missing published template in substates!")]
    MissingPublishedTemplate,
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

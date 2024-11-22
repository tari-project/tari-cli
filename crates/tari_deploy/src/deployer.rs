// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use crate::error::Error;
use crate::NetworkConfig;
use minotari_app_grpc::authentication::ClientAuthenticationInterceptor;
use minotari_app_grpc::tari_rpc::wallet_client::WalletClient;
use minotari_app_grpc::tari_rpc::GetBalanceRequest;
use std::path::Path;
use std::str::FromStr;
use tari_common_types::grpc_authentication::GrpcAuthentication;
use tari_core::transactions::tari_amount::MicroMinotari;
use tari_dan_engine::wasm::WasmModule;
use tari_wallet_daemon_client::types::{AuthLoginAcceptRequest, AuthLoginRequest, AuthLoginResponse};
use tari_wallet_daemon_client::WalletDaemonClient;
use tokio::fs;
use tonic::codegen::InterceptedService;
use tonic::transport::{Channel, Endpoint};

pub type Result<T> = std::result::Result<T, Error>;
type TariWalletClient = WalletClient<InterceptedService<Channel, ClientAuthenticationInterceptor>>;

/// Tari template deployer.
/// You can use this struct to deploy easily Tari template project to the target network.
/// Note: This is the entry point to use this library crate.
pub struct TemplateDeployer {
    network: NetworkConfig,
}

impl TemplateDeployer {
    pub fn new(network: NetworkConfig) -> Self {
        Self { network }
    }

    pub async fn deploy(&self, wasm_template: &Path) -> Result<()> {
        self.validate_wasm_template(wasm_template).await?;
        self.check_balance_to_deploy(wasm_template).await?;

        Ok(())
    }

    pub async fn check_balance_to_deploy(&self, wasm_template: &Path) -> Result<()> {
        let wallet_balance = self.wallet_balance().await?;
        // TODO: calculate fee for the template deployment transaction   
        Ok(())
    }

    /// Validating provided wasm template on the given path.
    async fn validate_wasm_template(&self, wasm_template: &Path) -> Result<()> {
        let wasm_code = fs::read(wasm_template).await?;
        WasmModule::load_template_from_code(wasm_code.as_slice())?;
        Ok(())
    }

    /// Get available wallet balance.
    async fn wallet_balance(&self) -> Result<MicroMinotari> {
        let mut client = self.wallet_client().await?;
        let result = client.get_balance(GetBalanceRequest {}).await?.into_inner();
        Ok(MicroMinotari::from(result.available_balance))
    }

    /// Returns a new Tari wallet client.
    async fn wallet_client(&self) -> Result<TariWalletClient> {
        let endpoint = Endpoint::from_str(self.network.wallet_grpc_address().as_str())?;
        Ok(WalletClient::with_interceptor(
            endpoint.connect().await?,
            ClientAuthenticationInterceptor::create(&GrpcAuthentication::default())?,
        ))
    }

    /// Returns a new wallet daemon client.
    async fn wallet_daemon_client(&self) -> Result<WalletDaemonClient> {
        let mut client = WalletDaemonClient::connect(self.network.wallet_daemon_jrpc_address(), None)?;

        // authentication
        let AuthLoginResponse { auth_token, .. } = client
            .auth_request(AuthLoginRequest {
                permissions: vec!["Admin".to_string()],
                duration: None,
            })
            .await?;
        let auth_response = client
            .auth_accept(AuthLoginAcceptRequest {
                auth_token,
                name: "default".to_string(),
            })
            .await?;

        client.set_auth_token(auth_response.permissions_token);

        Ok(client)
    }
}

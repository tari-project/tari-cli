// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use crate::error::Error;
use crate::uploader::TemplateBinaryUploader;
use crate::NetworkConfig;
use minotari_app_grpc::authentication::ClientAuthenticationInterceptor;
use minotari_app_grpc::tari_rpc::wallet_client::WalletClient;
use minotari_app_grpc::tari_rpc::{template_type, BuildInfo, CreateTemplateRegistrationRequest, GetBalanceRequest, TemplateType, WasmInfo};
use std::path::Path;
use std::str::FromStr;
use tari_core::transactions::tari_amount::MicroMinotari;
use tari_dan_engine::template::LoadedTemplate;
use tari_dan_engine::wasm::WasmModule;
use tari_engine_types::hashing::template_hasher32;
use tari_template_lib::prelude::TemplateAddress;
use tari_template_lib::Hash;
use tari_wallet_daemon_client::types::{AuthLoginAcceptRequest, AuthLoginRequest, AuthLoginResponse};
use tari_wallet_daemon_client::WalletDaemonClient;
use tokio::fs;
use tonic::codegen::InterceptedService;
use tonic::transport::{Channel, Endpoint};
use url::Url;

pub type Result<T> = std::result::Result<T, Error>;
type TariWalletClient = WalletClient<InterceptedService<Channel, ClientAuthenticationInterceptor>>;

/// Tari template deployer.
/// You can use this struct to deploy easily Tari template project to the target network.
/// Note: This is the entry point to use this library crate.
pub struct TemplateDeployer<U>
where
    U: TemplateBinaryUploader,
{
    network: NetworkConfig,
    uploader: U,
}

impl<U> TemplateDeployer<U>
where
    U: TemplateBinaryUploader,
{
    pub fn new(network: NetworkConfig, uploader: U) -> Self {
        Self { network, uploader }
    }

    /// Deploys the given compiled template to the configured network ([`TemplateDeployer::network`]).
    pub async fn deploy(&self, wasm_template: &Path, fee_per_gram: MicroMinotari) -> Result<TemplateAddress> {
        let (loaded_template, hash) = self.validate_and_load_wasm_template(wasm_template).await?;
        self.check_balance_to_deploy(wasm_template).await?;
        let uploaded_template_url = self.uploader.upload(wasm_template).await?;
        self.register_template(&loaded_template, hash, uploaded_template_url, fee_per_gram).await
    }

    /// Check if we have enough balance or not to deploy the template.
    pub async fn check_balance_to_deploy(&self, wasm_template: &Path) -> Result<()> {
        let wallet_balance = self.wallet_balance().await?;
        // TODO: calculate fee for the template deployment transaction
        Ok(())
    }

    /// Registering a template on base layer.
    async fn register_template(&self,
                               wasm_template: &LoadedTemplate,
                               wasm_template_hash: Hash,
                               template_url: Url,
                               fee_per_gram: MicroMinotari,
    ) -> Result<TemplateAddress> {
        let template_name = wasm_template.template_name();
        let request = CreateTemplateRegistrationRequest {
            template_name: template_name.to_string(),
            template_version: 1,
            template_type: Some(TemplateType {
                template_type: Some(template_type::TemplateType::Wasm(WasmInfo { abi_version: 1 })),
            }),
            build_info: Some(BuildInfo {
                repo_url: "".to_string(),
                commit_hash: vec![],
            }),
            binary_sha: wasm_template_hash.to_vec(),
            binary_url: template_url.to_string(),
            sidechain_deployment_key: vec![],
            fee_per_gram: fee_per_gram.as_u64(),
        };
        let mut wallet_client = self.wallet_client().await?;
        let response = wallet_client.create_template_registration(request).await?.into_inner();

        Ok(Hash::try_from_vec(response.template_address)?)
    }

    /// Validating provided wasm template on the given path.
    async fn validate_and_load_wasm_template(&self, wasm_template: &Path) -> Result<(LoadedTemplate, Hash)> {
        let wasm_code = fs::read(wasm_template).await?;
        let template = WasmModule::load_template_from_code(wasm_code.as_slice())?;
        let wasm_hash: Hash = template_hasher32().chain(&wasm_code).result();
        Ok((template, wasm_hash))
    }

    /// Get available wallet balance.
    async fn wallet_balance(&self) -> Result<MicroMinotari> {
        let mut client = self.wallet_client().await?;
        let result = client.get_balance(GetBalanceRequest {}).await?.into_inner();
        Ok(MicroMinotari::from(result.available_balance))
    }

    /// Returns a new Tari wallet client.
    async fn wallet_client(&self) -> Result<TariWalletClient> {
        let endpoint = Endpoint::from_str(self.network.wallet_grpc_config().address().as_str())?;
        Ok(WalletClient::with_interceptor(
            endpoint.connect().await?,
            ClientAuthenticationInterceptor::create(&self.network.wallet_grpc_config().authentication().into())?,
        ))
    }

    /// Returns a new wallet daemon client.
    async fn wallet_daemon_client(&self) -> Result<WalletDaemonClient> {
        let mut client = WalletDaemonClient::connect(self.network.wallet_daemon_jrpc_address().clone(), None)?;

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

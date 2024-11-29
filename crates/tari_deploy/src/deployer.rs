// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use crate::error::Error;
use crate::uploader::TemplateBinaryUploader;
use crate::NetworkConfig;
use minotari_app_grpc::authentication::ClientAuthenticationInterceptor;
use minotari_app_grpc::tari_rpc::wallet_client::WalletClient;
use minotari_app_grpc::tari_rpc::{
    template_type, BuildInfo, CreateTemplateRegistrationRequest, GetBalanceRequest, TemplateType,
    WasmInfo,
};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tari_core::transactions::tari_amount::MicroMinotari;
use tari_dan_engine::template::LoadedTemplate;
use tari_dan_engine::wasm::WasmModule;
use tari_engine_types::hashing::template_hasher32;
use tari_template_lib::prelude::TemplateAddress;
use tari_template_lib::Hash;
use tari_wallet_daemon_client::types::{
    AuthLoginAcceptRequest, AuthLoginRequest, AuthLoginResponse,
};
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

#[derive(Clone)]
pub enum DeployParams {
    /// When we want to validate and upload using a simple path to wasm binary.
    Binary { path: PathBuf },
    /// We want to use an already loaded, validated and uploaded wasm template.
    Uploaded {
        template: LoadedTemplate,
        template_hash: Hash,
        url: Url,
    },
}

impl<U> TemplateDeployer<U>
where
    U: TemplateBinaryUploader,
{
    pub fn new(network: NetworkConfig, uploader: U) -> Self {
        Self { network, uploader }
    }

    /// Deploys the given compiled template to the configured network ([`TemplateDeployer::network`]).
    pub async fn deploy(
        &self,
        params: DeployParams,
        fee_per_gram: MicroMinotari,
    ) -> Result<TemplateAddress> {
        let register_request = self
            .template_registration_request(params.clone(), fee_per_gram)
            .await?;
        self.check_balance_to_deploy(params, fee_per_gram).await?;
        self.register_template(register_request).await
    }

    /// Loads, validates and uploads the template.
    /// Returns the `DeployParams` that can be passed directly to `deploy()`.
    pub async fn upload_template(&self, wasm_template: &Path) -> Result<DeployParams> {
        let (template, template_hash) = self.validate_and_load_wasm_template(wasm_template).await?;
        let url = self.uploader.upload(wasm_template).await?;
        Ok(DeployParams::Uploaded {
            template,
            template_hash,
            url,
        })
    }

    /// Get registration fee.
    /// It does not deploy anything, just gets the calculated fee for the template.
    pub async fn registration_fee(
        &self,
        params: DeployParams,
        fee_per_gram: MicroMinotari,
    ) -> Result<MicroMinotari> {
        let request = self
            .template_registration_request(params, fee_per_gram)
            .await?;
        let mut wallet_client = self.wallet_client().await?;
        let response = wallet_client
            .get_template_registration_fee(request)
            .await?
            .into_inner();

        Ok(MicroMinotari::from(response.fee))
    }

    /// Check if we have enough balance or not to deploy the template.
    pub async fn check_balance_to_deploy(
        &self,
        params: DeployParams,
        fee_per_gram: MicroMinotari,
    ) -> Result<()> {
        let request = self
            .template_registration_request(params, fee_per_gram)
            .await?;
        let wallet_balance = self.wallet_balance().await?;
        let fee = self.get_registration_fee(request).await?;
        if fee > wallet_balance {
            return Err(Error::InsufficientBalance(wallet_balance, fee));
        }
        Ok(())
    }

    async fn get_registration_fee(
        &self,
        request: CreateTemplateRegistrationRequest,
    ) -> Result<MicroMinotari> {
        let mut wallet_client = self.wallet_client().await?;
        let response = wallet_client
            .get_template_registration_fee(request)
            .await?
            .into_inner();

        Ok(MicroMinotari::from(response.fee))
    }

    /// Registering a template on base layer.
    async fn register_template(
        &self,
        request: CreateTemplateRegistrationRequest,
    ) -> Result<TemplateAddress> {
        let mut wallet_client = self.wallet_client().await?;
        let response = wallet_client
            .create_template_registration(request)
            .await?
            .into_inner();

        Ok(Hash::try_from_vec(response.template_address)?)
    }

    async fn template_registration_request(
        &self,
        params: DeployParams,
        fee_per_gram: MicroMinotari,
    ) -> Result<CreateTemplateRegistrationRequest> {
        Ok(match params {
            DeployParams::Binary { path } => {
                let (loaded_template, hash) =
                    self.validate_and_load_wasm_template(path.as_path()).await?;
                let uploaded_template_url = self.uploader.upload(path.as_path()).await?;
                let template_name = loaded_template.template_name();
                CreateTemplateRegistrationRequest {
                    template_name: template_name.to_string(),
                    template_version: 1,
                    template_type: Some(TemplateType {
                        template_type: Some(template_type::TemplateType::Wasm(WasmInfo {
                            abi_version: 1,
                        })),
                    }),
                    build_info: Some(BuildInfo {
                        repo_url: "".to_string(),
                        commit_hash: vec![],
                    }),
                    binary_sha: hash.to_vec(),
                    binary_url: uploaded_template_url.to_string(),
                    sidechain_deployment_key: vec![],
                    fee_per_gram: fee_per_gram.as_u64(),
                }
            }
            DeployParams::Uploaded {
                template,
                template_hash,
                url,
            } => {
                let template_name = template.template_name();
                CreateTemplateRegistrationRequest {
                    template_name: template_name.to_string(),
                    template_version: 1,
                    template_type: Some(TemplateType {
                        template_type: Some(template_type::TemplateType::Wasm(WasmInfo {
                            abi_version: 1,
                        })),
                    }),
                    build_info: Some(BuildInfo {
                        repo_url: "".to_string(),
                        commit_hash: vec![],
                    }),
                    binary_sha: template_hash.to_vec(),
                    binary_url: url.to_string(),
                    sidechain_deployment_key: vec![],
                    fee_per_gram: fee_per_gram.as_u64(),
                }
            }
        })
    }

    /// Validating provided wasm template on the given path.
    async fn validate_and_load_wasm_template(
        &self,
        wasm_template: &Path,
    ) -> Result<(LoadedTemplate, Hash)> {
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
            ClientAuthenticationInterceptor::create(
                &self
                    .network
                    .wallet_grpc_config()
                    .authentication()
                    .try_into()?,
            )?,
        ))
    }

    #[allow(dead_code)]
    /// Returns a new wallet daemon client.
    async fn wallet_daemon_client(&self) -> Result<WalletDaemonClient> {
        let mut client =
            WalletDaemonClient::connect(self.network.wallet_daemon_jrpc_address().clone(), None)?;

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

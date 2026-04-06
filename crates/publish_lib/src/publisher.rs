// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use crate::error::Error;
use crate::{NetworkConfig, PublisherError};
use serde::Serialize;
use std::borrow::Cow;
use std::path::PathBuf;
use std::time::Duration;
use tari_engine::template::LoadedTemplate;
use tari_engine::wasm::WasmModule;
use tari_engine_types::commit_result::TransactionResult;
use tari_engine_types::hashing::template_hasher32;
use tari_engine_types::substate::SubstateId;
use tari_ootle_common_types::optional::Optional;
use tari_ootle_template_metadata::MetadataHash;
use tari_ootle_template_metadata::TemplateMetadata;
use tari_ootle_walletd_client::permissions::JrpcPermission;
use tari_ootle_walletd_client::types::{
    AccountsGetBalancesRequest, AuthCredentials, AuthGetMethodResponse, AuthLoginRequest, AuthLoginResponse,
    AuthMethod, PublishTemplateMetadata, PublishTemplateRequest, SignTemplateMetadataRequest,
    SignTemplateMetadataResponse, TransactionWaitResultRequest, WalletGetInfoResponse,
};
use tari_ootle_walletd_client::{ComponentAddressOrName, WalletDaemonClient};
use tari_template_lib_types::Hash32;
use tari_template_lib_types::constants::TARI_TOKEN;
use tari_template_lib_types::{Amount, TemplateAddress};
use tokio::fs;

pub type Result<T> = std::result::Result<T, Error>;

/// Tari template publisher.
/// You can use this struct to easily publish a Tari template project to the target network.
/// Note: This is the entry point to use this library crate.
pub struct TemplatePublisher {
    network: NetworkConfig,
}

/// Provided template to publish.
#[derive(Clone)]
pub enum Template {
    /// Publish from a path.
    Path { path: PathBuf },
    /// Publish from a loaded binary.
    Binary { bin: Vec<u8> },
}

impl TemplatePublisher {
    pub fn new(network: NetworkConfig) -> Self {
        Self { network }
    }

    /// Publishes the given compiled template to the configured network ([`TemplatePublisher::network`]).
    pub async fn publish(
        &self,
        account: &ComponentAddressOrName,
        template: Template,
        max_fee: u64,
        metadata_hash: Option<MetadataHash>,
        wait_timeout: Option<Duration>,
    ) -> Result<TemplateAddress> {
        let publish_template_request = self
            .create_publish_template_request(account, &template, max_fee, metadata_hash.clone())
            .await?;
        self.check_balance_for_publish(account, &template, metadata_hash)
            .await?;
        self.publish_template(
            publish_template_request,
            wait_timeout.or(Some(Duration::from_secs(120))),
        )
        .await
    }

    /// Get publish fee.
    /// It does not publish anything, just gets the calculated fee for the template.
    pub async fn publish_fee(&self, account: &ComponentAddressOrName, template: &Template) -> Result<u64> {
        let mut request = self
            .create_publish_template_request(account, template, 1_000_000, None)
            .await?;
        self.get_publish_fee(&mut request).await
    }

    pub async fn get_default_account(&self) -> Result<Option<ComponentAddressOrName>> {
        let mut client = self.wallet_daemon_client().await?;
        let account = client.accounts_get_default().await.optional()?;
        let address = account.map(|a| *a.account.component_address());
        Ok(address.map(Into::into))
    }

    pub async fn get_wallet_info(&self) -> Result<WalletGetInfoResponse> {
        let mut client = self.wallet_daemon_client().await?;
        let info = client.get_wallet_info().await?;
        Ok(info)
    }

    /// Signs template metadata using the wallet daemon's key management.
    pub async fn sign_template_metadata(
        &self,
        request: SignTemplateMetadataRequest,
    ) -> Result<SignTemplateMetadataResponse> {
        let mut client = self.wallet_daemon_client().await?;
        let response = client.sign_template_metadata(request).await?;
        Ok(response)
    }

    /// Higher-level helper: sign metadata for a template using the default account key.
    ///
    /// Returns a [`SignedMetadataPayload`] with all fields needed to POST to the community server.
    pub async fn sign_metadata_for_publish(
        &self,
        key_index: u64,
        template_address: TemplateAddress,
        metadata: TemplateMetadata,
    ) -> Result<SignedMetadataPayload> {
        use tari_ootle_walletd_client::types::SignTemplateMetadataRequest;

        let key_id =
            tari_ootle_wallet_sdk::models::KeyId::derived(tari_ootle_wallet_sdk::models::KeyBranch::Account, key_index);

        let response = self
            .sign_template_metadata(SignTemplateMetadataRequest {
                key_id,
                template_address,
                metadata,
            })
            .await?;

        Ok(SignedMetadataPayload {
            metadata_cbor: response.metadata_cbor,
            public_nonce: response.public_nonce,
            signature: response.signature,
            public_key: response.public_key,
            metadata_hash: response.metadata_hash,
        })
    }

    /// Get publish fee based on a [`PublishTemplateRequest`].
    async fn get_publish_fee(&self, request: &mut PublishTemplateRequest) -> Result<u64> {
        let mut client = self.wallet_daemon_client().await?;
        request.dry_run = true;
        let response = client.publish_template(request).await?;
        let fee = response.dry_run_fee.ok_or_else(|| {
            PublisherError::InvalidResponse("Wallet daemon returned an empty dry run fee".to_string())
        })?;
        Ok(fee)
    }

    /// Check if we have enough balance or not to publish the template.
    pub async fn check_balance_for_publish(
        &self,
        account: &ComponentAddressOrName,
        template: &Template,
        metadata_hash: Option<MetadataHash>,
    ) -> Result<CheckBalanceResult> {
        let mut request = self
            .create_publish_template_request(account, template, 1_000_000, metadata_hash)
            .await?;
        let bin_size = request.binary.len();
        let max_fee = self.get_publish_fee(&mut request).await?;
        let wallet_balance = self.wallet_xtr_balance(account).await?;
        if wallet_balance < max_fee {
            return Err(Error::InsufficientBalance {
                current: wallet_balance,
                fee: max_fee,
            });
        }
        Ok(CheckBalanceResult {
            max_fee,
            binary_size: bin_size,
        })
    }

    /// Publishing a template on Ootle (Layer-2).
    async fn publish_template(
        &self,
        request: PublishTemplateRequest,
        tx_finalize_timeout: Option<Duration>,
    ) -> Result<TemplateAddress> {
        let mut client = self.wallet_daemon_client().await?;
        let response = client.publish_template(request).await?;

        let tx_resp = client
            .wait_transaction_result(TransactionWaitResultRequest {
                transaction_id: response.transaction_id,
                timeout_secs: tx_finalize_timeout.map(|duration| duration.as_secs()),
            })
            .await?;

        if tx_resp.timed_out {
            return Err(Error::WaitForTransactionTimeout(response.transaction_id.to_string()));
        }

        let finalize_result = tx_resp
            .result
            .ok_or(Error::MissingTransactionResult(response.transaction_id.to_string()))?;
        if !matches!(finalize_result.result, TransactionResult::Accept(_)) {
            let error_status = match finalize_result.result {
                TransactionResult::AcceptFeeRejectRest(_, reason) | TransactionResult::Reject(reason) => {
                    format!("⚠️ Status: {}\n⚠️ Reason: {}", tx_resp.status, reason)
                },
                TransactionResult::Accept(_) => String::new(), // does not happen here
            };
            return Err(Error::InvalidTransaction(
                response.transaction_id.to_string(),
                error_status,
            ));
        }

        // look for the new UP template substate
        let mut result = None;
        if let TransactionResult::Accept(diff) = finalize_result.result {
            for (substate_id, _) in diff.up_iter() {
                if let SubstateId::Template(addr) = substate_id {
                    result = Some(addr.as_hash());
                    break;
                }
            }
        }

        result.ok_or(Error::MissingPublishedTemplate)
    }

    async fn create_publish_template_request(
        &self,
        account: &ComponentAddressOrName,
        template: &Template,
        max_fee: u64,
        metadata_hash: Option<MetadataHash>,
    ) -> Result<PublishTemplateRequest> {
        let (binary, _, _) = self.validate_and_load_wasm_template(template).await?;
        Ok(PublishTemplateRequest {
            binary: binary.into_owned(),
            fee_account: Some(account.clone()),
            max_fee,
            metadata: metadata_hash.map(PublishTemplateMetadata::Hash),
            detect_inputs: true,
            dry_run: false,
        })
    }

    /// Validating provided wasm template on the given path.
    async fn validate_and_load_wasm_template<'a>(
        &self,
        params: &'a Template,
    ) -> Result<(Cow<'a, Vec<u8>>, LoadedTemplate, Hash32)> {
        let wasm_code = match params {
            Template::Path { path } => {
                let bin = fs::read(path).await?;
                Cow::Owned(bin)
            },
            Template::Binary { bin } => Cow::Borrowed(bin),
        };
        let template = WasmModule::load_template_from_code(wasm_code.as_slice())?;
        let wasm_hash: Hash32 = template_hasher32().chain(&wasm_code).result();
        Ok((wasm_code, template, wasm_hash))
    }

    /// Get available wallet TARI_TOKEN balance.
    async fn wallet_xtr_balance(&self, account: &ComponentAddressOrName) -> Result<Amount> {
        let mut client = self.wallet_daemon_client().await?;
        let balances_response = client
            .get_account_balances(AccountsGetBalancesRequest {
                account: Some(account.clone()),
                refresh: false,
            })
            .await?;
        let balance = balances_response
            .balances
            .iter()
            .find(|b| b.resource_address == TARI_TOKEN)
            .map(|b| b.balance)
            .unwrap_or_default();

        Ok(balance)
    }

    /// Returns a new wallet daemon client.
    async fn wallet_daemon_client(&self) -> Result<WalletDaemonClient> {
        let mut client = WalletDaemonClient::connect(self.network.wallet_daemon_jrpc_address().clone(), None)?;

        let AuthGetMethodResponse { method } = client.get_auth_method().await?;
        let credentials = match method {
            AuthMethod::None => AuthCredentials::None,
            AuthMethod::Webauthn => {
                return Err(Error::NotSupportedError(
                    "Webauthn is not currently supported".to_string(),
                ));
            },
        };
        // authentication
        let AuthLoginResponse { token } = client
            .auth_request(AuthLoginRequest {
                permissions: vec![JrpcPermission::Admin],
                credentials,
            })
            .await?;

        client.set_auth_token(token);

        Ok(client)
    }
}

pub struct CheckBalanceResult {
    pub max_fee: u64,
    pub binary_size: usize,
}

/// All fields needed to POST signed metadata to the community server.
///
/// Serializes to JSON matching the community server's expected format:
/// `{ "metadata_cbor": "<hex>", "public_nonce": "<hex>", "signature": "<hex>" }`
#[derive(Debug, Clone, Serialize)]
pub struct SignedMetadataPayload {
    #[serde(with = "ootle_serde::hex")]
    pub metadata_cbor: Vec<u8>,
    pub public_nonce: tari_template_lib_types::crypto::RistrettoPublicKeyBytes,
    pub signature: tari_template_lib_types::crypto::Scalar32Bytes,
    pub public_key: tari_template_lib_types::crypto::RistrettoPublicKeyBytes,
    pub metadata_hash: MetadataHash,
}

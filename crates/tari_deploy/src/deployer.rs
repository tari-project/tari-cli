// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use crate::error::Error;
use crate::{DeployerError, NetworkConfig};
use std::borrow::Cow;
use std::path::PathBuf;
use std::time::Duration;
use tari_engine::template::LoadedTemplate;
use tari_engine::wasm::WasmModule;
use tari_engine_types::commit_result::TransactionResult;
use tari_engine_types::hashing::template_hasher32;
use tari_engine_types::substate::SubstateId;
use tari_ootle_common_types::optional::Optional;
use tari_template_lib::constants::XTR;
use tari_template_lib::types::Hash;
use tari_template_lib::types::{Amount, TemplateAddress};
use tari_wallet_daemon_client::types::{
    AccountsGetBalancesRequest, AuthLoginAcceptRequest, AuthLoginRequest, AuthLoginResponse, PublishTemplateRequest,
    TransactionWaitResultRequest, WalletGetInfoResponse,
};
use tari_wallet_daemon_client::{ComponentAddressOrName, WalletDaemonClient};
use tokio::fs;

pub type Result<T> = std::result::Result<T, Error>;
pub const TOKEN_SYMBOL: &str = "XTR";

/// Tari template deployer.
/// You can use this struct to deploy easily Tari template project to the target network.
/// Note: This is the entry point to use this library crate.
pub struct TemplateDeployer {
    network: NetworkConfig,
}

/// Provided template to deploy.
#[derive(Clone)]
pub enum Template {
    /// Deploy from a path.
    Path { path: PathBuf },
    /// Deploy from a loaded binary.
    Binary { bin: Vec<u8> },
}

impl TemplateDeployer {
    pub fn new(network: NetworkConfig) -> Self {
        Self { network }
    }

    /// Deploys the given compiled template to the configured network ([`TemplateDeployer::network`]).
    pub async fn deploy(
        &self,
        account: &ComponentAddressOrName,
        template: Template,
        max_fee: u64,
        wait_timeout: Option<Duration>,
    ) -> Result<TemplateAddress> {
        let publish_template_request = self
            .create_publish_template_request(account, &template, max_fee)
            .await?;
        self.check_balance_to_deploy(account, &template).await?;
        self.publish_template(
            publish_template_request,
            wait_timeout.or(Some(Duration::from_secs(120))),
        )
        .await
    }

    /// Get publish fee.
    /// It does not deploy anything, just gets the calculated fee for the template.
    pub async fn publish_fee(&self, account: &ComponentAddressOrName, template: &Template) -> Result<u64> {
        let mut request = self
            .create_publish_template_request(account, template, 1_000_000)
            .await?;
        self.get_publish_fee(&mut request).await
    }

    pub async fn get_default_account(&self) -> Result<Option<ComponentAddressOrName>> {
        let mut client = self.wallet_daemon_client().await?;
        let account = client.accounts_get_default().await.optional()?;
        let address = account.map(|a| {
            *a.account
                .component_address()
        });
        Ok(address.map(Into::into))
    }

    pub async fn get_wallet_info(&self) -> Result<WalletGetInfoResponse> {
        let mut client = self.wallet_daemon_client().await?;
        let info = client.get_wallet_info().await?;
        Ok(info)
    }

    /// Get publish fee based on a [`PublishTemplateRequest`].
    async fn get_publish_fee(&self, request: &mut PublishTemplateRequest) -> Result<u64> {
        let mut client = self.wallet_daemon_client().await?;
        request.dry_run = true;
        let response = client.publish_template(request).await?;
        let fee = response
            .dry_run_fee
            .ok_or_else(|| DeployerError::InvalidResponse("Wallet daemon returned an empty dry run fee".to_string()))?;
        Ok(fee)
    }

    /// Check if we have enough balance or not to deploy the template.
    pub async fn check_balance_to_deploy(
        &self,
        account: &ComponentAddressOrName,
        template: &Template,
    ) -> Result<CheckBalanceResult> {
        let mut request = self
            .create_publish_template_request(account, template, 1_000_000)
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
    ) -> Result<PublishTemplateRequest> {
        let (binary, _, _) = self.validate_and_load_wasm_template(template).await?;
        Ok(PublishTemplateRequest {
            binary: binary.into_owned(),
            fee_account: Some(account.clone()),
            max_fee,
            detect_inputs: true,
            dry_run: false,
        })
    }

    /// Validating provided wasm template on the given path.
    async fn validate_and_load_wasm_template<'a>(
        &self,
        params: &'a Template,
    ) -> Result<(Cow<'a, Vec<u8>>, LoadedTemplate, Hash)> {
        let wasm_code = match params {
            Template::Path { path } => {
                let bin = fs::read(path).await?;
                Cow::Owned(bin)
            },
            Template::Binary { bin } => Cow::Borrowed(bin),
        };
        let template = WasmModule::load_template_from_code(wasm_code.as_slice())?;
        let wasm_hash: Hash = template_hasher32().chain(&wasm_code).result();
        Ok((wasm_code, template, wasm_hash))
    }

    /// Get available wallet XTR balance.
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
            .find(|b| b.resource_address == XTR)
            .map(|b| b.balance)
            .unwrap_or_default();

        Ok(balance)
    }

    /// Returns a new wallet daemon client.
    async fn wallet_daemon_client(&self) -> Result<WalletDaemonClient> {
        let mut client = WalletDaemonClient::connect(self.network.wallet_daemon_jrpc_address().clone(), None)?;

        // authentication
        let AuthLoginResponse { auth_token, .. } = client
            .auth_request(AuthLoginRequest {
                permissions: vec!["Admin".to_string()],
                duration: None,
                webauthn_finish_auth_request: None,
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

pub struct CheckBalanceResult {
    pub max_fee: u64,
    pub binary_size: usize,
}

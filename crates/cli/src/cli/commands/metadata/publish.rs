// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, anyhow};
use clap::Parser;
use tari_engine_types::published_template::PublishedTemplateAddress;
use tari_ootle_publish_lib::publisher::{SignedMetadataPayload, TemplatePublisher};
use tari_ootle_template_metadata::TemplateMetadata;
use url::Url;

use crate::cli::commands::publish::load_project_config;
use crate::cli::config::Config;

const DEFAULT_METADATA_SERVER: &str = "http://localhost:3000";
const METADATA_CBOR_FILENAME: &str = "template_metadata.cbor";

/// Default retry settings: 6 attempts, 10s initial backoff (10, 20, 40, 80, 160s ≈ ~5 min total).
const DEFAULT_MAX_RETRIES: u32 = 6;
const DEFAULT_INITIAL_BACKOFF_SECS: u64 = 10;

#[derive(Clone, Parser, Debug)]
pub struct PublishMetadataArgs {
    /// Path to the template crate directory.
    /// Defaults to the current directory.
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Template address to publish metadata for (e.g. template_bce07f... or raw hex).
    #[arg(long, short = 't')]
    pub template_address: PublishedTemplateAddress,

    /// Metadata server URL. Overrides the value in tari.config.toml and global CLI config.
    #[arg(long)]
    pub metadata_server_url: Option<Url>,

    /// Maximum number of retry attempts for 404 (template not yet synced).
    #[arg(long, default_value_t = DEFAULT_MAX_RETRIES)]
    pub max_retries: u32,

    /// Use author-signed metadata submission.
    /// Signs via the wallet daemon and allows updating metadata without
    /// republishing the template on-chain.
    #[arg(long)]
    pub signed: bool,

    /// Key index for the author signing key (default: 0).
    /// Used with --signed to identify which derived account key to sign with.
    #[arg(long, default_value_t = 0)]
    pub key_index: u64,

    /// Wallet daemon JSON-RPC URL.
    /// Overrides the value in tari.config.toml and global CLI config.
    /// Required with --signed.
    #[arg(long)]
    pub wallet_daemon_url: Option<url::Url>,
}

pub async fn handle(config: Config, args: PublishMetadataArgs) -> anyhow::Result<()> {
    let cbor_path = find_metadata_cbor(&args.path).await?;
    let cbor_bytes = std::fs::read(&cbor_path).context("reading metadata CBOR file")?;

    let url_override = args.wallet_daemon_url.as_ref().or(config.wallet_daemon_url.as_ref());
    let project_config = load_project_config(&args.path, url_override).await?;

    // Resolve metadata server URL: CLI flag > project config > global config > default
    let default_url: Url = DEFAULT_METADATA_SERVER.parse().unwrap();
    let metadata_server_url = args
        .metadata_server_url
        .as_ref()
        .or(project_config.metadata_server_url())
        .or(config.metadata_server_url.as_ref())
        .unwrap_or(&default_url);

    let metadata =
        TemplateMetadata::from_cbor(&cbor_bytes).context("metadata CBOR is invalid — cannot publish corrupt data")?;
    println!(
        "📄 Publishing metadata for {} v{} to {}",
        metadata.name, metadata.version, metadata_server_url
    );

    let addr = args.template_address;

    if args.signed {
        let publisher = TemplatePublisher::new(project_config.network().clone());

        let payload = publisher
            .sign_metadata_for_publish(args.key_index, addr.as_template_address(), metadata)
            .await
            .context("signing metadata via wallet daemon")?;

        println!("🔑 Signed as author: {}", payload.public_key);

        publish_metadata_signed(metadata_server_url, &addr, &payload, args.max_retries).await
    } else {
        publish_metadata_to_server(metadata_server_url, &addr, &cbor_bytes, args.max_retries).await
    }
}

/// Flow 1: Hash-verified metadata publish (POST raw CBOR).
pub async fn publish_metadata_to_server(
    server_url: &Url,
    template_address: &PublishedTemplateAddress,
    cbor_bytes: &[u8],
    max_retries: u32,
) -> anyhow::Result<()> {
    let addr = template_address.as_template_address();
    let url = server_url
        .join(&format!("/api/templates/{addr}/metadata"))
        .context("building metadata endpoint URL")?;

    let client = reqwest::Client::new();
    let mut backoff = Duration::from_secs(DEFAULT_INITIAL_BACKOFF_SECS);

    for attempt in 0..=max_retries {
        let resp = client
            .post(url.clone())
            .header("Content-Type", "application/cbor")
            .body(cbor_bytes.to_vec())
            .send()
            .await
            .with_context(|| format!("POST {url}"))?;

        let status = resp.status();
        if status.is_success() {
            println!("✅ Metadata published successfully");
            return Ok(());
        }

        let body = resp.text().await.unwrap_or_default();

        if status == reqwest::StatusCode::NOT_FOUND && attempt < max_retries {
            println!(
                "⏳ Template not yet synced by server (attempt {}/{}), retrying in {}s...",
                attempt + 1,
                max_retries + 1,
                backoff.as_secs()
            );
            tokio::time::sleep(backoff).await;
            backoff *= 2;
            continue;
        }

        return Err(anyhow!("Metadata server returned {status}: {body}"));
    }

    unreachable!()
}

/// Flow 2: Author-signed metadata publish (POST JSON with Schnorr signature from walletd).
pub async fn publish_metadata_signed(
    server_url: &Url,
    template_address: &PublishedTemplateAddress,
    payload: &SignedMetadataPayload,
    max_retries: u32,
) -> anyhow::Result<()> {
    let addr = template_address.as_template_address();
    let url = server_url
        .join(&format!("/api/templates/{addr}/metadata/signed"))
        .context("building signed metadata endpoint URL")?;

    let client = reqwest::Client::new();
    let mut backoff = Duration::from_secs(DEFAULT_INITIAL_BACKOFF_SECS);

    for attempt in 0..=max_retries {
        let resp = client
            .post(url.clone())
            .header("Content-Type", "application/json")
            .json(payload)
            .send()
            .await
            .with_context(|| format!("POST {url}"))?;

        let status = resp.status();
        if status.is_success() {
            println!("✅ Signed metadata published successfully");
            return Ok(());
        }

        let resp_body = resp.text().await.unwrap_or_default();

        if status == reqwest::StatusCode::NOT_FOUND && attempt < max_retries {
            println!(
                "⏳ Template not yet synced by server (attempt {}/{}), retrying in {}s...",
                attempt + 1,
                max_retries + 1,
                backoff.as_secs()
            );
            tokio::time::sleep(backoff).await;
            backoff *= 2;
            continue;
        }

        return Err(anyhow!("Metadata server returned {status}: {resp_body}"));
    }

    unreachable!()
}

async fn find_metadata_cbor(crate_dir: &Path) -> anyhow::Result<PathBuf> {
    let target_dir = crate::cli::commands::publish::find_target_dir(crate_dir).await?;
    let build_dir = target_dir.join("wasm32-unknown-unknown").join("release").join("build");

    if !build_dir.exists() {
        return Err(anyhow!(
            "Build output directory not found at {}. Run `tari build` first.",
            build_dir.display()
        ));
    }

    for entry in std::fs::read_dir(&build_dir).context("reading build directory")? {
        let entry = entry?;
        let out_file = entry.path().join("out").join(METADATA_CBOR_FILENAME);
        if out_file.exists() {
            return Ok(out_file);
        }
    }

    Err(anyhow!(
        "No {METADATA_CBOR_FILENAME} found in build output. \
         Make sure the template uses tari_ootle_template_build in build.rs \
         and has been built with `tari build`."
    ))
}

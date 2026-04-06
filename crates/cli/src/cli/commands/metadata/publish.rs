// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, anyhow};
use blake2::{Blake2b512, Digest};
use clap::Parser;
use tari_crypto::keys::{PublicKey, SecretKey};
use tari_crypto::ristretto::{RistrettoPublicKey, RistrettoSchnorr, RistrettoSecretKey};
use tari_crypto::tari_utilities::ByteArray;
use tari_ootle_template_metadata::TemplateMetadata;
use url::Url;

const DEFAULT_METADATA_SERVER: &str = "http://localhost:3000";
const METADATA_CBOR_FILENAME: &str = "template_metadata.cbor";
const SIGNED_METADATA_DOMAIN: &[u8] = b"com.tari.ootle.community.SignedMetadataUpdate";

/// Default retry settings: 6 attempts, 10s initial backoff (10, 20, 40, 80, 160s ≈ ~5 min total).
const DEFAULT_MAX_RETRIES: u32 = 6;
const DEFAULT_INITIAL_BACKOFF_SECS: u64 = 10;

#[derive(Clone, Parser, Debug)]
pub struct PublishMetadataArgs {
    /// Path to the template crate directory.
    /// Defaults to the current directory.
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Template address to publish metadata for.
    #[arg(long, short = 't')]
    pub template_address: String,

    /// Metadata server URL.
    #[arg(long, default_value = DEFAULT_METADATA_SERVER)]
    pub metadata_server_url: Url,

    /// Maximum number of retry attempts for 404 (template not yet synced).
    #[arg(long, default_value_t = DEFAULT_MAX_RETRIES)]
    pub max_retries: u32,

    /// Use author-signed metadata submission (Flow 2).
    /// Allows updating metadata without republishing the template on-chain.
    #[arg(long)]
    pub signed: bool,

    /// Author secret key (hex). Required with --signed.
    /// If not provided, you will be prompted.
    #[arg(long)]
    pub secret_key: Option<String>,
}

pub async fn handle(args: PublishMetadataArgs) -> anyhow::Result<()> {
    let cbor_path = find_metadata_cbor(&args.path).await?;
    let cbor_bytes = std::fs::read(&cbor_path).context("reading metadata CBOR file")?;

    // Verify it decodes before sending
    let metadata =
        TemplateMetadata::from_cbor(&cbor_bytes).context("metadata CBOR is invalid — cannot publish corrupt data")?;
    println!(
        "📄 Publishing metadata for {} v{} to {}",
        metadata.name, metadata.version, args.metadata_server_url
    );

    if args.signed {
        publish_metadata_signed(
            &args.metadata_server_url,
            &args.template_address,
            &cbor_bytes,
            args.secret_key.as_deref(),
            args.max_retries,
        )
        .await
    } else {
        publish_metadata_to_server(
            &args.metadata_server_url,
            &args.template_address,
            &cbor_bytes,
            args.max_retries,
        )
        .await
    }
}

/// Flow 1: Hash-verified metadata publish (POST raw CBOR).
pub async fn publish_metadata_to_server(
    server_url: &Url,
    template_address: &str,
    cbor_bytes: &[u8],
    max_retries: u32,
) -> anyhow::Result<()> {
    let url = server_url
        .join(&format!("/api/templates/{template_address}/metadata"))
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

        // 404 means the template hasn't been synced by the server yet — retry with backoff
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

/// Flow 2: Author-signed metadata publish (POST JSON with Schnorr signature).
pub async fn publish_metadata_signed(
    server_url: &Url,
    template_address: &str,
    cbor_bytes: &[u8],
    secret_key_hex: Option<&str>,
    max_retries: u32,
) -> anyhow::Result<()> {
    let secret_key_hex = match secret_key_hex {
        Some(hex) => hex.to_string(),
        None => {
            let input: String = dialoguer::Password::new()
                .with_prompt("Author secret key (hex)")
                .interact()?;
            input.trim().to_string()
        },
    };

    let secret_key_bytes = hex::decode(&secret_key_hex).context("invalid hex for secret key")?;
    let secret_key =
        RistrettoSecretKey::from_canonical_bytes(&secret_key_bytes).map_err(|e| anyhow!("invalid secret key: {e}"))?;
    let public_key = RistrettoPublicKey::from_secret_key(&secret_key);

    println!("🔑 Signing as author: {}", hex::encode(public_key.as_bytes()));

    // Generate nonce
    let nonce = RistrettoSecretKey::random(&mut rand::thread_rng());
    let public_nonce = RistrettoPublicKey::from_secret_key(&nonce);

    // Construct challenge: Blake2b-512(domain || nonce || pk || addr || cbor)
    let addr_bytes = hex::decode(template_address).context("invalid hex for template address")?;
    let challenge = Blake2b512::new()
        .chain_update(SIGNED_METADATA_DOMAIN)
        .chain_update(public_nonce.as_bytes())
        .chain_update(public_key.as_bytes())
        .chain_update(&addr_bytes)
        .chain_update(cbor_bytes)
        .finalize();

    let signature = RistrettoSchnorr::sign_raw_uniform(&secret_key, nonce, &challenge)
        .map_err(|e| anyhow!("signing failed: {e}"))?;

    let body = serde_json::json!({
        "metadata_cbor": hex::encode(cbor_bytes),
        "public_nonce": hex::encode(signature.get_public_nonce().as_bytes()),
        "signature": hex::encode(signature.get_signature().as_bytes()),
    });

    let url = server_url
        .join(&format!("/api/templates/{template_address}/metadata/signed"))
        .context("building signed metadata endpoint URL")?;

    let client = reqwest::Client::new();
    let mut backoff = Duration::from_secs(DEFAULT_INITIAL_BACKOFF_SECS);

    for attempt in 0..=max_retries {
        let resp = client
            .post(url.clone())
            .header("Content-Type", "application/json")
            .json(&body)
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

// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use std::path::{Path, PathBuf};

use anyhow::{Context, anyhow};
use clap::Parser;
use tari_ootle_template_metadata::TemplateMetadata;
use url::Url;

const DEFAULT_METADATA_SERVER: &str = "http://localhost:3000";
const METADATA_CBOR_FILENAME: &str = "template_metadata.cbor";

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

    publish_metadata_to_server(&args.metadata_server_url, &args.template_address, &cbor_bytes).await
}

pub async fn publish_metadata_to_server(
    server_url: &Url,
    template_address: &str,
    cbor_bytes: &[u8],
) -> anyhow::Result<()> {
    let url = server_url
        .join(&format!("/api/templates/{template_address}/metadata"))
        .context("building metadata endpoint URL")?;

    let client = reqwest::Client::new();
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
        Ok(())
    } else {
        let body = resp.text().await.unwrap_or_default();
        Err(anyhow!("Metadata server returned {status}: {body}"))
    }
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

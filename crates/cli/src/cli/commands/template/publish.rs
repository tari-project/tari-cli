// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use std::io::BufReader;
use std::path::{Path, PathBuf};

use anyhow::{Context, anyhow};
use clap::Parser;
use dialoguer::Confirm;
use tari_ootle_publish_lib::publisher::{CheckBalanceResult, Template, TemplatePublisher};
use tari_ootle_publish_lib::walletd_client::ComponentAddressOrName;
use tari_ootle_template_metadata::TemplateMetadata;

use crate::cli::commands::metadata::publish::publish_metadata_to_server;
use crate::cli::commands::publish::{build_template, load_project_config};
use crate::cli::config::Config;
use crate::cli::util;
use crate::loading;

const MAX_WASM_SIZE: usize = 5 * 1000 * 1000; // 5 MB
const METADATA_CBOR_FILENAME: &str = "template_metadata.cbor";

#[derive(Clone, Parser, Debug)]
pub struct TemplatePublishArgs {
    /// Path to the template crate directory.
    /// Defaults to the current directory.
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Account to be used for publishing fees.
    #[arg(short = 'a', long)]
    pub account: Option<ComponentAddressOrName>,

    /// (Optional) Custom network name.
    #[arg(short = 'c', long)]
    pub custom_network: Option<String>,

    /// Confirm template publishing without prompting.
    #[arg(short = 'y', long, default_value_t = false)]
    pub yes: bool,

    /// (Optional) Maximum fee in microtari.
    #[arg(short = 'f', long)]
    pub max_fee: Option<u64>,

    /// (Optional) Path to a pre-compiled WASM binary.
    #[arg(long, alias = "bin")]
    pub binary: Option<PathBuf>,

    /// Wallet daemon JSON-RPC URL.
    /// Overrides the value in tari.config.toml and global CLI config.
    #[arg(long)]
    pub wallet_daemon_url: Option<url::Url>,

    /// After publishing, automatically submit metadata to a metadata server.
    #[arg(long, default_value_t = false)]
    pub publish_metadata: bool,

    /// Metadata server URL (used with --publish-metadata).
    #[arg(long, default_value = "http://localhost:3000")]
    pub metadata_server_url: url::Url,
}

pub async fn handle(config: Config, mut args: TemplatePublishArgs) -> anyhow::Result<()> {
    let crate_dir = &args.path;

    let url_override = args.wallet_daemon_url.as_ref().or(config.wallet_daemon_url.as_ref());
    let project_config = load_project_config(crate_dir, url_override).await?;

    // Build or use provided binary
    let template_bin = match args.binary.take() {
        Some(bin_path) => {
            println!("📦 Using provided WASM binary at {}", bin_path.display());
            bin_path
        },
        None => build_template(crate_dir).await?,
    };

    // Find and read metadata CBOR from build output
    let metadata_hash = match find_metadata_cbor(crate_dir).await {
        Ok(cbor_path) => {
            println!("📄 Found metadata at {}", cbor_path.display());
            let file = std::fs::File::open(&cbor_path).context("opening metadata CBOR file")?;
            let reader = BufReader::new(file);
            let metadata = TemplateMetadata::read_cbor_from(reader).context("decoding metadata CBOR")?;
            let hash = metadata.hash().context("computing metadata hash")?;
            println!("🔑 Metadata hash: {hash}");
            println!("   Name:        {}", metadata.name);
            println!("   Version:     {}", metadata.version);
            if !metadata.description.is_empty() {
                println!("   Description: {}", metadata.description);
            }
            if let Some(ref category) = metadata.category {
                println!("   Category:    {category}");
            }
            if !metadata.tags.is_empty() {
                println!("   Tags:        {}", metadata.tags.join(", "));
            }
            if let Some(ref license) = metadata.license {
                println!("   License:     {license}");
            }
            Some(hash)
        },
        Err(e) => {
            println!("⚠️  No metadata found ({e}), publishing without metadata hash");
            None
        },
    };

    // Connect to wallet daemon
    let publisher = TemplatePublisher::new(project_config.network().clone());
    let info = publisher.get_wallet_info().await.with_context(|| {
        anyhow!(
            "Failed to connect to the wallet at {}",
            project_config.network().wallet_daemon_jrpc_address(),
        )
    })?;
    println!(
        "🔗 Connected to wallet version {} (network: {})",
        info.version, info.network
    );

    let account = resolve_account(&args, &config, &publisher, &project_config).await?;
    let template = Template::Path { path: template_bin };

    let CheckBalanceResult { max_fee, binary_size } = publisher
        .check_balance_for_publish(&account, &template, metadata_hash.clone())
        .await?;

    if binary_size > MAX_WASM_SIZE {
        println!("⚠️ WASM binary size exceeded: {}", util::human_bytes(binary_size));
    } else {
        println!("✅ WASM size: {}", util::human_bytes(binary_size));
    }

    if !args.yes {
        let confirmation = Confirm::new()
            .with_prompt(format!(
                "⚠️ Publishing this template costs {max_fee} (estimated), are you sure to continue?",
            ))
            .interact()?;
        if !confirmation {
            return Err(anyhow!("💥 Publishing aborted!"));
        }
    }

    let template_address = loading!(
        "Publishing template. This may take while...",
        publisher
            .publish(&account, template, max_fee, metadata_hash.clone(), None)
            .await
    )?;

    println!("⭐ Your new template's address: {template_address}");

    if args.publish_metadata && metadata_hash.is_some() {
        let cbor_path = find_metadata_cbor(crate_dir).await?;
        let cbor_bytes = std::fs::read(&cbor_path).context("reading metadata CBOR for server publish")?;

        println!("⏳ Waiting 20 seconds for on-chain confirmation before publishing metadata...");
        tokio::time::sleep(std::time::Duration::from_secs(20)).await;

        match publish_metadata_to_server(&args.metadata_server_url, &template_address.to_string(), &cbor_bytes).await {
            Ok(()) => {},
            Err(e) => {
                println!("⚠️  Failed to publish metadata to server: {e}");
                println!(
                    "   You can retry with: tari metadata publish --template-address {template_address} --metadata-server-url {}",
                    args.metadata_server_url
                );
            },
        }
    } else if args.publish_metadata && metadata_hash.is_none() {
        println!("⚠️  --publish-metadata was set but no metadata was found, skipping");
    }

    Ok(())
}

async fn find_metadata_cbor(crate_dir: &Path) -> anyhow::Result<PathBuf> {
    let target_dir = crate::cli::commands::publish::find_target_dir(crate_dir).await?;
    let build_dir = target_dir.join("wasm32-unknown-unknown").join("release").join("build");

    if !build_dir.exists() {
        return Err(anyhow!("build output directory not found at {}", build_dir.display()));
    }

    let mut found = Vec::new();
    for entry in std::fs::read_dir(&build_dir).context("reading build directory")? {
        let entry = entry?;
        let out_file = entry.path().join("out").join(METADATA_CBOR_FILENAME);
        if out_file.exists() {
            found.push(out_file);
        }
    }

    match found.len() {
        0 => Err(anyhow!("no {METADATA_CBOR_FILENAME} in build output")),
        1 => Ok(found.into_iter().next().unwrap()),
        _ => Err(anyhow!(
            "Multiple metadata files found. Specify the CBOR file path to `tari template inspect` instead."
        )),
    }
}

async fn resolve_account(
    args: &TemplatePublishArgs,
    config: &Config,
    publisher: &TemplatePublisher,
    project_config: &crate::project::ProjectConfig,
) -> anyhow::Result<ComponentAddressOrName> {
    let account = args
        .account
        .as_ref()
        .cloned()
        .or_else(|| {
            project_config
                .parsed_default_account()
                .expect("Malformed default account")
        })
        .or(config.default_account.clone());

    match account {
        Some(account) => {
            println!("🔍 Using account: {account}");
            Ok(account)
        },
        None => {
            let account = publisher.get_default_account().await?;
            match account {
                Some(account) => {
                    println!("❓ No Account specified. Using default account: {account}");
                    Ok(account)
                },
                None => Err(anyhow!("No account found! Please create an account first.")),
            }
        },
    }
}

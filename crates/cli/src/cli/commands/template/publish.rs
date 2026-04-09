// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use std::io::BufReader;
use std::path::PathBuf;

use anyhow::{Context, anyhow};
use clap::Parser;
use dialoguer::Confirm;
use tari_engine_types::published_template::PublishedTemplateAddress;
use tari_ootle_publish_lib::publisher::{CheckBalanceResult, Template, TemplatePublisher};
use tari_ootle_publish_lib::walletd_client::ComponentAddressOrName;
use tari_ootle_template_metadata::TemplateMetadata;

use crate::cli::commands::metadata::publish::publish_metadata_to_server;
use crate::cli::commands::publish::{build_template, find_metadata_cbor, load_project_config};
use crate::cli::config::Config;
use crate::cli::util;
use crate::loading;

const MAX_WASM_SIZE: usize = 5 * 1000 * 1000; // 5 MB

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
    /// Overrides the value in tari.config.toml and global CLI config.
    #[arg(long)]
    pub metadata_server_url: Option<url::Url>,
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

    let published_addr = PublishedTemplateAddress::from_template_address(template_address);
    println!("⭐ Your new template's address: {published_addr}");

    // Save template address to project config
    let config_path = crate::cli::commands::config::resolve_config_path()?;
    if config_path.exists() {
        let content = tokio::fs::read_to_string(&config_path)
            .await
            .context("reading config")?;
        let mut doc = content.parse::<toml_edit::DocumentMut>().context("parsing config")?;
        doc.insert("template-address", toml_edit::value(published_addr.to_string()));
        tokio::fs::write(&config_path, doc.to_string())
            .await
            .context("writing config")?;
        println!("📝 Saved template address to {}", config_path.display());
    }

    if args.publish_metadata && metadata_hash.is_some() {
        let cbor_path = find_metadata_cbor(crate_dir).await?;
        let cbor_bytes = std::fs::read(&cbor_path).context("reading metadata CBOR for server publish")?;

        let default_url: url::Url = "http://localhost:3000".parse().unwrap();
        let metadata_server_url = args
            .metadata_server_url
            .as_ref()
            .or(project_config.metadata_server_url())
            .or(config.metadata_server_url.as_ref())
            .unwrap_or(&default_url);

        println!("📡 Publishing metadata to {metadata_server_url}...");
        match publish_metadata_to_server(metadata_server_url, &published_addr, &cbor_bytes, 6).await {
            Ok(()) => {},
            Err(e) => {
                println!("⚠️  Failed to publish metadata to server: {e}");
                println!(
                    "   You can retry with: tari metadata publish --template-address {published_addr} --metadata-server-url {metadata_server_url}",
                );
            },
        }
    } else if args.publish_metadata && metadata_hash.is_none() {
        println!("⚠️  --publish-metadata was set but no metadata was found, skipping");
    }

    Ok(())
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

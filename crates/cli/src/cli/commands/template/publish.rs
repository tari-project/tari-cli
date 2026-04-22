// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use std::io::BufReader;
use std::path::PathBuf;

use anyhow::{Context, anyhow};
use clap::Parser;
use dialoguer::Confirm;
use tari_engine_types::published_template::PublishedTemplateAddress;
use tari_ootle_common_types::Network;
use tari_ootle_publish_lib::NetworkConfig;
use tari_ootle_publish_lib::publisher::{CheckBalanceResult, Template, TemplatePublisher};
use tari_ootle_publish_lib::walletd_client::ComponentAddressOrName;
use tari_ootle_template_metadata::TemplateMetadata;

use crate::cli::commands::metadata::publish::publish_metadata_to_server;
use crate::cli::commands::publish::{
    build_template, find_metadata_cbor, load_project_config, resolve_active_network, resolve_wallet_daemon_url,
};
use crate::cli::config::Config;
use crate::cli::util;
use crate::cli::util::get_default_metadata_server_url;
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

pub async fn handle(
    config: Config,
    network_override: Option<Network>,
    mut args: TemplatePublishArgs,
) -> anyhow::Result<()> {
    let crate_dir = &args.path;

    let project_config = load_project_config(crate_dir).await?;
    let network = resolve_active_network(network_override, &project_config, &config);
    let wallet_daemon_url =
        resolve_wallet_daemon_url(args.wallet_daemon_url.as_ref(), &project_config, &config, network);
    println!("🌐 Network: {network}");

    // Warn if template address already exists in config (republishing)
    if let Some(existing_addr) = project_config.template_address(network) {
        println!("⚠️  A template has already been published from this project: {existing_addr}");
        println!("   If the template binary is unchanged, the transaction will fail.");
        println!("   If changed, a new template address will be generated.");
        let proceed = Confirm::new()
            .with_prompt("Continue with publish?")
            .default(false)
            .interact()?;
        if !proceed {
            return Err(anyhow!("Publishing aborted"));
        }
    }

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
            if let Some(ref commit_hash) = metadata.commit_hash {
                println!("   Commit hash: {commit_hash}");
            }
            if let Some(ref supersedes) = metadata.supersedes {
                println!("   Supersedes:  {supersedes}");
            }
            Some(hash)
        },
        Err(e) => {
            println!("⚠️  No metadata found ({e}), publishing without metadata hash");
            None
        },
    };

    // Connect to wallet daemon
    let publisher = TemplatePublisher::new(NetworkConfig::new(wallet_daemon_url.clone()));
    let info = publisher
        .get_wallet_info()
        .await
        .with_context(|| anyhow!("Failed to connect to the wallet at {}", wallet_daemon_url))?;
    println!(
        "🔗 Connected to wallet version {} (wallet network: {})",
        info.version, info.network
    );

    if info.network_byte != network.as_byte() {
        return Err(anyhow!(
            "Wallet daemon is on network '{}' but the CLI is configured for '{network}'. \
             Use --network <name> to switch, or point --wallet-daemon-url at a daemon for the right network.",
            info.network
        ));
    }

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

    // Save template address to project config under [networks.<network>]
    let config_path = crate::cli::commands::config::resolve_config_path()?;
    if config_path.exists() {
        let content = tokio::fs::read_to_string(&config_path)
            .await
            .context("reading config")?;
        let mut doc = content.parse::<toml_edit::DocumentMut>().context("parsing config")?;
        crate::cli::commands::config::set_dotted_key(
            &mut doc,
            &format!("networks.{}.template-address", network.as_key_str()),
            &published_addr.to_string(),
        )?;
        tokio::fs::write(&config_path, doc.to_string())
            .await
            .context("writing config")?;
        println!("📝 Saved template address to {}", config_path.display());
    } else {
        println!(
            "ℹ️  Config file not found at {}. Run `tari config init` to create one.",
            config_path.display()
        );
    }

    let should_publish_metadata = if args.publish_metadata {
        metadata_hash.is_some()
    } else if metadata_hash.is_some() {
        Confirm::new()
            .with_prompt("Publish metadata to community server?")
            .default(false)
            .interact()?
    } else {
        false
    };

    if should_publish_metadata {
        let cbor_path = find_metadata_cbor(crate_dir).await?;
        let cbor_bytes = std::fs::read(&cbor_path).context("reading metadata CBOR for server publish")?;

        let resolved_default = get_default_metadata_server_url(network)
            .map(|s| s.parse::<url::Url>().expect("parse default metadata server url"));
        let metadata_server_url = args
            .metadata_server_url
            .as_ref()
            .or(project_config.metadata_server_url(network))
            .or(config.metadata_server_url(network))
            .or(resolved_default.as_ref())
            .ok_or_else(|| {
                anyhow!(
                    "No metadata server URL configured and no default known for network '{network}'. \
                     Pass --metadata-server-url or set it in tari.config.toml."
                )
            })?;

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

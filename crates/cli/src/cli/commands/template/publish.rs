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

use crate::cli::commands::publish::{build_template, load_project_config};
use crate::cli::config::Config;
use crate::cli::util;
use crate::loading;

const MAX_WASM_SIZE: usize = 5 * 1000 * 1000; // 5 MB
const METADATA_CBOR_FILENAME: &str = "template_metadata.cbor";

#[derive(Clone, Parser, Debug)]
pub struct TemplatePublishArgs {
    /// Template project to publish.
    #[arg()]
    pub template: Option<String>,

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

    /// Project folder containing tari.config.toml.
    #[arg(long, value_name = "PATH", default_value = crate::cli::command::default_output_dir().into_os_string())]
    pub project_folder: PathBuf,

    /// (Optional) Path to a pre-compiled WASM binary.
    #[arg(long, alias = "bin")]
    pub binary: Option<PathBuf>,
}

pub async fn handle(config: Config, mut args: TemplatePublishArgs) -> anyhow::Result<()> {
    if args.binary.is_none() && args.template.is_none() {
        return Err(anyhow!(
            "Either a template name or a binary path must be provided for publishing!"
        ));
    }

    let project_config = load_project_config(&args.project_folder).await?;

    // Build or use provided binary
    let publish_args = to_publish_args(&args);
    let template_bin = match args.binary.take() {
        Some(bin_path) => {
            println!("📦 Using provided WASM binary at {}", bin_path.display());
            bin_path
        },
        None => build_template(&publish_args).await?,
    };

    // Find and read metadata CBOR from build output
    let metadata_hash = match find_metadata_cbor(&args.project_folder, args.template.as_deref()) {
        Ok(cbor_path) => {
            println!("📄 Found metadata at {}", cbor_path.display());
            let file = std::fs::File::open(&cbor_path).context("opening metadata CBOR file")?;
            let reader = BufReader::new(file);
            let metadata = TemplateMetadata::read_cbor_from(reader).context("decoding metadata CBOR")?;
            let hash = metadata.hash().context("computing metadata hash")?;
            println!("🔑 Metadata hash: {hash}");
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

    let CheckBalanceResult { max_fee, binary_size } =
        publisher.check_balance_for_publish(&account, &template, metadata_hash.clone()).await?;

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
            .publish(&account, template, max_fee, metadata_hash, None)
            .await
    )?;

    println!("⭐ Your new template's address: {template_address}");

    Ok(())
}

fn find_metadata_cbor(project_folder: &Path, template_name: Option<&str>) -> anyhow::Result<PathBuf> {
    let build_dir = project_folder
        .join("target")
        .join("wasm32-unknown-unknown")
        .join("release")
        .join("build");

    if !build_dir.exists() {
        return Err(anyhow!("build output directory not found"));
    }

    let mut found = Vec::new();
    for entry in std::fs::read_dir(&build_dir).context("reading build directory")? {
        let entry = entry?;
        let dir_name = entry.file_name().to_string_lossy().to_string();
        let out_file = entry.path().join("out").join(METADATA_CBOR_FILENAME);
        if out_file.exists() {
            found.push((dir_name, out_file));
        }
    }

    match found.len() {
        0 => Err(anyhow!("no {METADATA_CBOR_FILENAME} in build output")),
        1 => Ok(found.into_iter().next().unwrap().1),
        _ => {
            // Try to match by template name
            if let Some(name) = template_name {
                let name_normalized = name.replace('-', "_");
                if let Some((_, path)) = found.iter().find(|(dir, _)| dir.starts_with(&name_normalized)) {
                    return Ok(path.clone());
                }
                Err(anyhow!(
                    "Multiple metadata files found, but none matched template '{name}'. \
                     Specify the path to the CBOR file explicitly."
                ))
            } else {
                Err(anyhow!(
                    "Multiple metadata files found. Specify the template name or \
                     the path to the CBOR file explicitly."
                ))
            }
        },
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

/// Convert TemplatePublishArgs to the existing PublishArgs for reuse of build_template.
fn to_publish_args(args: &TemplatePublishArgs) -> crate::cli::commands::publish::PublishArgs {
    crate::cli::commands::publish::PublishArgs {
        template: args.template.clone(),
        account: args.account.clone(),
        custom_network: args.custom_network.clone(),
        yes: args.yes,
        max_fee: args.max_fee,
        project_folder: args.project_folder.clone(),
        binary: args.binary.clone(),
    }
}

// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use crate::cli::config::Config;
use crate::cli::util;
use crate::{loading, project};
use anyhow::{anyhow, Context};
use cargo_toml::Manifest;
use clap::Parser;
use dialoguer::Confirm;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tari_deploy::deployer::{CheckBalanceResult, Template, TemplateDeployer, TOKEN_SYMBOL};
use tari_wallet_daemon_client::ComponentAddressOrName;
use tokio::fs;
use tokio::process::Command;

const MAX_WASM_SIZE: usize = 5 * 1000 * 1000; // 5 MB

#[derive(Clone, Parser, Debug)]
pub struct DeployArgs {
    /// Template project to deploy
    #[arg()]
    pub template: String,

    /// Account to be used for deployment fees.
    #[arg(short = 'a', long)]
    pub account: Option<ComponentAddressOrName>,

    /// (Optional) Custom network name.
    /// Custom network name set in project config.
    /// It must be set when network is set to custom!
    #[arg(short = 'c', long)]
    pub custom_network: Option<String>,

    /// Confirm template deployment.
    /// If false, it will be asked.
    #[arg(short = 'y', long, default_value_t = false)]
    pub yes: bool,

    /// (Optional) Maximum fee
    /// Maximum fee applied to the deployment.
    ///
    /// Automatically adjusted to estimated fee if not set.
    #[arg(short = 'f', long)]
    pub max_fee: Option<u64>,

    /// Project folder where we have the project configuration file (tari.config.toml).
    #[arg(long, value_name = "PATH", default_value = crate::cli::command::default_target_dir().into_os_string()
    )]
    pub project_folder: PathBuf,
}

pub async fn handle(config: Config, args: &DeployArgs) -> anyhow::Result<()> {
    // load network config from project config file
    let project_config = load_project_config(&args.project_folder).await?;

    // lookup project name and dir
    let mut crate_dir = None;
    let mut crate_name = String::new();
    let workspace_cargo_toml = Manifest::from_path(args.project_folder.join("Cargo.toml"))?;
    let crates = workspace_cargo_toml
        .workspace
        .ok_or(anyhow!("Project is not a Cargo workspace project!"))?
        .members;
    for project in crates {
        let cargo_toml =
            Manifest::from_path(args.project_folder.join(project.clone()).join("Cargo.toml"))?;
        let curr_crate_name = cargo_toml
            .package
            .ok_or(anyhow!("No package details set!"))?
            .name;
        if curr_crate_name.eq_ignore_ascii_case(&args.template) {
            crate_dir = Some(args.project_folder.join(project));
            crate_name = curr_crate_name;
        }
    }
    let Some(crate_dir) = crate_dir else {
        return Err(anyhow!("Project \"{}\" not found!", args.template));
    };

    // build
    let template_bin = loading!(
        format!("Building WASM template project **{}**", crate_name),
        build_project(&crate_dir, &crate_name).await
    )?;

    // template deployer
    let deployer = TemplateDeployer::new(project_config.network().clone());
    let info = deployer.get_wallet_info().await.with_context(|| {
        anyhow!(
            "Failed to connect to the wallet at {}",
            project_config.network().wallet_daemon_jrpc_address(),
        )
    })?;

    println!(
        "🔗 Connected to wallet version {} (network: {})",
        info.version, info.network
    );

    let account = args
        .account
        .as_ref()
        .cloned()
        .or_else(|| {
            project_config
                .parsed_default_account()
                .expect("Malformed default account")
        })
        .or(config.default_account);
    let account = match account {
        Some(account) => {
            println!("🔍 Using account: {account}");
            account
        }
        None => {
            let account = deployer.get_default_account().await?;
            let Some(account) = account else {
                return Err(anyhow!("No account found! Please create an account first."));
            };
            println!("❓ No Account specified. Using default account: {account}");
            account
        }
    };
    let template = Template::Path { path: template_bin };

    // check balance and get max fee
    let CheckBalanceResult {
        max_fee,
        binary_size,
    } = deployer
        .check_balance_to_deploy(&account, &template)
        .await?;

    if binary_size > MAX_WASM_SIZE {
        println!(
            "⚠️ WASM binary size exceeded: {}",
            util::human_bytes(binary_size)
        );
    } else {
        println!("✅ WASM size: {}", util::human_bytes(binary_size));
    }

    if !args.yes {
        let confirmation = Confirm::new()
            .with_prompt(format!(
                "⚠️ Deploying this template costs {max_fee} {TOKEN_SYMBOL} (estimated), are you sure to continue?",
            ))
            .interact()?;
        if !confirmation {
            return Err(anyhow!("💥 Deployment aborted!"));
        }
    }

    // deploy
    let template_address = loading!(
        format!("Deploying template **{crate_name}**. This may take while..."),
        deployer.deploy(&account, template, max_fee, None).await
    )?;

    println!("⭐ Your new template's address: {template_address}");

    Ok(())
}

async fn build_project(dir: &Path, name: &str) -> anyhow::Result<PathBuf> {
    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .arg("--target=wasm32-unknown-unknown")
        .arg("--release")
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let process = cmd.spawn()?;

    let output = process.wait_with_output().await?;

    if !output.status.success() {
        return Err(anyhow!(
            "Failed to build project: {dir:?}\nBuild Output:\n\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let output_bin = dir
        .join("target")
        .join("wasm32-unknown-unknown")
        .join("release")
        .join(format!("{name}.wasm"));
    if !util::file_exists(&output_bin).await? {
        return Err(anyhow!(
            "Binary is not present after build at {:?}\n\nBuild Output:\n{}",
            output_bin,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(output_bin)
}

async fn load_project_config(project_folder: &Path) -> anyhow::Result<project::ProjectConfig> {
    let config_file = project_folder.join(project::CONFIG_FILE_NAME);
    Ok(toml::from_str(
        fs::read_to_string(&config_file)
            .await
            .map_err(|error| {
                anyhow!("Failed to load project config file (at {config_file:?}): {error:?}")
            })?
            .as_str(),
    )?)
}

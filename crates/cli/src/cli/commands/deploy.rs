// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use crate::cli::arguments::Network;
use crate::cli::util;
use crate::{loading, project};
use anyhow::anyhow;
use cargo_toml::Manifest;
use clap::Parser;
use dialoguer::{Confirm, Input};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::str::FromStr;
use tari_core::transactions::tari_amount::MicroMinotari;
use tari_deploy::deployer::{Template, TemplateDeployer, TOKEN_SYMBOL};
use tari_deploy::NetworkConfig;
use tari_wallet_daemon_client::ComponentAddressOrName;
use tokio::fs;
use tokio::process::Command;

#[derive(Clone, Parser, Debug)]
pub struct DeployArgs {
    /// Template project to deploy
    #[arg()]
    pub template: String,

    /// Tari Ootle network
    #[clap(value_enum, default_value_t=Network::Local)]
    #[arg(short = 'n', long)]
    pub network: Network,

    /// Account to be used for deployment fees.
    #[arg(short = 'a', long)]
    pub account: String,

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
    #[arg(long, value_name = "PATH", default_value = crate::cli::arguments::default_target_dir().into_os_string()
    )]
    pub project_folder: PathBuf,
}

pub async fn handle(args: &DeployArgs) -> anyhow::Result<()> {
    // load network config from project config file
    let project_config = load_project_config(&args.project_folder).await?;
    let network_config = if args.network == Network::Custom {
        match &args.custom_network {
            Some(custom_network) => network_config(&project_config, custom_network),
            None => Err(anyhow!("No custom network name provided!")),
        }
    } else {
        network_config(&project_config, args.network.to_string().as_str())
    }?;

    // lookup project name and dir
    let mut project_dir = None;
    let mut project_name = String::new();
    let workspace_cargo_toml = Manifest::from_path(args.project_folder.join("Cargo.toml"))?;
    let projects = workspace_cargo_toml
        .workspace
        .ok_or(anyhow!("Project is not a Cargo workspace project!"))?
        .members;
    for project in projects {
        let cargo_toml =
            Manifest::from_path(args.project_folder.join(project.clone()).join("Cargo.toml"))?;
        let curr_project_name = cargo_toml
            .package
            .ok_or(anyhow!("No package details set!"))?
            .name;
        if curr_project_name.to_lowercase() == args.template.to_lowercase() {
            project_dir = Some(args.project_folder.join(project));
            project_name = curr_project_name;
        }
    }
    if project_dir.is_none() {
        return Err(anyhow!("Project \"{}\" not found!", args.template));
    }
    let project_dir = project_dir.unwrap();

    // build
    let template_bin = loading!(
        format!("Building WASM template project \"{}\"", project_name),
        build_project(&project_dir, project_name.clone()).await
    )?;

    // init template deployer
    let deployer = TemplateDeployer::new(network_config);

    // confirmation
    if !args.yes {
        let confirmation = Confirm::new()
            .with_prompt(format!("❓Deploying a template costs some {TOKEN_SYMBOL}, are you sure to continue?"))
            .interact()?;
        if !confirmation {
            return Err(anyhow!("💥 Deployment aborted!"));
        }
    }

    // TODO: instead of prompting max fee, just get publish fee and set it as max fee

    // max fee
    let max_fee = match args.max_fee {
        Some(value) => value,
        None => u64::from_str(
            Input::new()
                .with_prompt("Maximum fee")
                .validate_with(|input: &String| -> Result<(), &str> {
                    match MicroMinotari::from_str(input) {
                        Ok(_) => Ok(()),
                        Err(_) => Err("Maximum fee must be a positive integer!"),
                    }
                })
                .default("200000".to_string())
                .interact()?
                .as_str(),
        )?,
    };

    let account = ComponentAddressOrName::from_str(args.account.as_str())?;
    let template = Template::Path { path: template_bin };

    // check balance
    deployer
        .check_balance_to_deploy(&account, &template, max_fee)
        .await?;

    if !args.yes {
        let fee = deployer
            .publish_fee(&account, &template, max_fee)
            .await?;
        let confirmation = Confirm::new()
            .with_prompt(format!(
                "❓Deploying this template costs {} {TOKEN_SYMBOL} (estimated), are you sure to continue?",
                fee
            ))
            .interact()?;
        if !confirmation {
            return Err(anyhow!("💥 Deployment aborted!"));
        }
    }

    // deploy
    let template_address = loading!(
        format!(
            "Deploying project \"{}\" to {} network",
            project_name, args.network
        ),
        deployer.deploy(&account, template, max_fee).await
    )?;

    println!("⭐ Your new template's address: {}", template_address);

    Ok(())
}

async fn build_project(dir: &Path, name: String) -> anyhow::Result<PathBuf> {
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
        .join(format!("{}.wasm", name));
    if !util::file_exists(&output_bin).await? {
        return Err(anyhow!(
            "Binary is not present after build at {:?}\n\nBuild Output:\n{}",
            output_bin,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(output_bin)
}

fn network_config(
    project_config: &project::Config,
    network: &str,
) -> anyhow::Result<NetworkConfig> {
    match project_config.find_network_config(network) {
        Some(found_network) => Ok(found_network.clone()),
        None => Err(anyhow!("Network not found in project config: {}", network)),
    }
}

async fn load_project_config(project_folder: &Path) -> anyhow::Result<project::Config> {
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

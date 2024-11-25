// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use crate::cli::arguments::Network;
use crate::cli::util;
use crate::{loading, project};
use anyhow::anyhow;
use cargo_toml::Manifest;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tari_deploy::deployer::TemplateDeployer;
use tari_deploy::uploader::LocalSwarmUploader;
use tari_deploy::NetworkConfig;
use tokio::fs;
use tokio::process::Command;

pub async fn handle(
    template: &str,
    network: Network,
    custom_network: Option<&String>,
    project_folder: &Path,
) -> anyhow::Result<()> {
    // load network config from project config file
    let project_config = load_project_config(project_folder).await?;
    let network_config = if network == Network::Custom {
        match custom_network {
            Some(custom_network) => {
                network_config(&project_config, custom_network)
            }
            None => { Err(anyhow!("No custom network name provided!")) }
        }
    } else {
        network_config(&project_config, network.to_string().as_str())
    }?;

    // lookup project name and dir
    let mut project_dir = None;
    let mut project_name = String::new();
    let workspace_cargo_toml = Manifest::from_path(project_folder.join("Cargo.toml"))?;
    let projects = workspace_cargo_toml.workspace.ok_or(anyhow!("Project is not a Cargo workspace project!"))?.members;
    for project in projects {
        let cargo_toml = Manifest::from_path(project_folder.join(project.clone()).join("Cargo.toml"))?;
        let curr_project_name = cargo_toml.package.ok_or(anyhow!("No package details set!"))?.name;
        if curr_project_name.to_lowercase() == template.to_lowercase() {
            project_dir = Some(project_folder.join(project));
            project_name = curr_project_name;
        }
    }
    if project_dir.is_none() {
        return Err(anyhow!("Project \"{template}\" not found!"));
    }
    let project_dir = project_dir.unwrap();

    // build
    let template_bin = loading!(format!("Building WASM template project \"{}\"", project_name), build_project(&project_dir, project_name.clone()).await)?;

    // deploy
    // TODO: implement and use uploaders for the remaining networks
    let uploader = match network {
        Network::MainNet |
        Network::TestNet |
        Network::Custom |
        Network::Local => LocalSwarmUploader::new(network_config.uploader_endpoint().clone()),
    };
    loading!(
        format!("Deploying project \"{}\" to {} network", project_name, network),
        TemplateDeployer::new(network_config, uploader).deploy(&template_bin).await
    )?;

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
        return Err(anyhow!("Failed to build project: {dir:?}\nBuild Output:\n\n{}", String::from_utf8_lossy(&output.stderr)));
    }

    let output_bin = dir
        .join("target")
        .join("wasm32-unknown-unknown")
        .join("release")
        .join(format!("{}.wasm", name));
    if !util::file_exists(&output_bin).await? {
        return Err(anyhow!("Binary is not present after build at {:?}\n\nBuild Output:\n{}", output_bin, String::from_utf8_lossy(&output.stderr)));
    }

    Ok(output_bin)
}

fn network_config(project_config: &project::Config, network: &str) -> anyhow::Result<NetworkConfig> {
    match project_config.find_network_config(network) {
        Some(found_network) => Ok(found_network.clone()),
        None => Err(anyhow!("Network not found in project config: {}", network))
    }
}

async fn load_project_config(project_folder: &Path) -> anyhow::Result<project::Config> {
    let config_file = project_folder.join(project::CONFIG_FILE_NAME);
    Ok(
        toml::from_str(fs::read_to_string(&config_file).await
            .map_err(|error| anyhow!("Failed to load project config file (at {config_file:?}): {error:?}"))?
            .as_str())?
    )
}
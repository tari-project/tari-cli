// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use crate::cli::commands::template::publish::TemplatePublishArgs;
use crate::cli::config::Config;
use crate::cli::util;
use crate::{loading, project};
use anyhow::{Context, anyhow};
use cargo_toml::Manifest;
use clap::Parser;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tari_ootle_publish_lib::walletd_client::ComponentAddressOrName;
use tokio::fs;
use tokio::process::Command;

#[derive(Clone, Parser, Debug)]
pub struct PublishArgs {
    /// Path to the template crate directory.
    /// Defaults to the current directory.
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Account to be used for publishing fees.
    #[arg(short = 'a', long)]
    pub account: Option<ComponentAddressOrName>,

    /// (Optional) Custom network name.
    /// Custom network name set in project config.
    /// It must be set when network is set to custom!
    #[arg(short = 'c', long)]
    pub custom_network: Option<String>,

    /// Confirm template publishing.
    /// If false, it will be asked.
    #[arg(short = 'y', long, default_value_t = false)]
    pub yes: bool,

    /// (Optional) Maximum fee
    /// Maximum fee applied to publishing.
    ///
    /// Automatically adjusted to estimated fee if not set.
    #[arg(short = 'f', long)]
    pub max_fee: Option<u64>,

    /// (Optional) Path to the compiled WASM binary.
    /// If not set, the project will be built before publishing.
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

pub async fn build_template(crate_dir: &Path) -> anyhow::Result<PathBuf> {
    let cargo_path = crate_dir.join("Cargo.toml");
    if !cargo_path.exists() {
        return Err(anyhow!("No Cargo.toml found at {}", cargo_path.display()));
    }

    let manifest = Manifest::from_path(&cargo_path)?;
    let crate_name = manifest
        .package
        .ok_or_else(|| anyhow!("No [package] section in {}", cargo_path.display()))?
        .name;

    let template_bin = loading!(
        format!("Building WASM template project **{}**", crate_name),
        build_project(crate_dir, &crate_name).await
    )?;

    Ok(template_bin)
}

/// `tari publish` delegates to `tari template publish` — they behave identically.
pub async fn handle(config: Config, args: PublishArgs) -> anyhow::Result<()> {
    let template_args = TemplatePublishArgs {
        path: args.path,
        account: args.account,
        custom_network: args.custom_network,
        yes: args.yes,
        max_fee: args.max_fee,
        binary: args.binary,
        wallet_daemon_url: args.wallet_daemon_url,
        publish_metadata: args.publish_metadata,
        metadata_server_url: args.metadata_server_url,
    };
    crate::cli::commands::template::publish::handle(config, template_args).await
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

    // Find the target directory (may be in a parent workspace)
    let target_dir = find_target_dir(dir).await?;
    let wasm_name = name.replace('-', "_");
    let output_bin = target_dir
        .join("wasm32-unknown-unknown")
        .join("release")
        .join(format!("{wasm_name}.wasm"));

    if !util::file_exists(&output_bin).await? {
        return Err(anyhow!(
            "Binary is not present after build at {:?}\n\nBuild Output:\n{}",
            output_bin,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(output_bin)
}

pub async fn find_target_dir(dir: &Path) -> anyhow::Result<PathBuf> {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version=1", "--no-deps"])
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await?;

    if !output.status.success() {
        return Err(anyhow!(
            "Failed to get cargo metadata: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout).context("parsing cargo metadata")?;

    metadata["target_directory"]
        .as_str()
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("cargo metadata missing target_directory"))
}

const METADATA_CBOR_FILENAME: &str = "template_metadata.cbor";

/// Find the most recently generated metadata CBOR file in the build output.
pub async fn find_metadata_cbor(project_dir: &Path) -> anyhow::Result<PathBuf> {
    let target_dir = find_target_dir(project_dir).await?;
    let build_dir = target_dir.join("wasm32-unknown-unknown").join("release").join("build");

    if !build_dir.exists() {
        return Err(anyhow!(
            "Build output directory not found at {}. Run `tari build` first.",
            build_dir.display()
        ));
    }

    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    for entry in std::fs::read_dir(&build_dir).context("reading build directory")? {
        let entry = entry?;
        let out_file = entry.path().join("out").join(METADATA_CBOR_FILENAME);
        if out_file.exists() {
            let modified = std::fs::metadata(&out_file)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            if newest.as_ref().map_or(true, |(_, t)| modified > *t) {
                newest = Some((out_file, modified));
            }
        }
    }

    newest.map(|(path, _)| path).ok_or_else(|| {
        anyhow!(
            "No {METADATA_CBOR_FILENAME} found in build output. \
             Make sure the template uses tari_ootle_template_build in build.rs \
             and has been built with `tari build`."
        )
    })
}

pub async fn load_project_config(
    project_folder: &Path,
    wallet_daemon_url_override: Option<&url::Url>,
) -> anyhow::Result<project::ProjectConfig> {
    // Search current dir and parents for tari.config.toml
    let mut config = None;
    let mut search_dir = project_folder.to_path_buf();
    loop {
        let config_file = search_dir.join(project::CONFIG_FILE_NAME);
        if config_file.exists() {
            config = Some(
                toml::from_str::<project::ProjectConfig>(
                    fs::read_to_string(&config_file)
                        .await
                        .map_err(|error| {
                            anyhow!(
                                "Failed to load project config file (at {}): {}",
                                config_file.display(),
                                error
                            )
                        })?
                        .as_str(),
                )
                .context("parsing config toml")?,
            );
            break;
        }
        if !search_dir.pop() {
            break;
        }
    }

    let mut config = config.unwrap_or_default();

    // CLI flag overrides everything
    if let Some(url) = wallet_daemon_url_override {
        config.set_wallet_daemon_url(url.clone());
    }

    Ok(config)
}

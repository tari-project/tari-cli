// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use anyhow::Context;
use clap::Parser;
use tokio::fs;

use crate::cli::commands::template::init_metadata::{self, InitMetadataArgs};
use crate::project::{CONFIG_FILE_NAME, ProjectConfig};

#[derive(Clone, Parser, Debug)]
pub struct InitArgs {
    #[clap(flatten)]
    pub metadata_args: InitMetadataArgs,
}

pub async fn handle(args: InitArgs) -> anyhow::Result<()> {
    // Init project config (tari.config.toml) in the target crate directory
    let config_path = args.metadata_args.path.join(CONFIG_FILE_NAME);
    if config_path.exists() {
        println!("ℹ️  {} already exists at {}", CONFIG_FILE_NAME, config_path.display());
    } else {
        let default = toml::to_string_pretty(&ProjectConfig::default())?;
        fs::write(&config_path, &default).await.context("writing config file")?;
        println!("✅ Created {} at {}", CONFIG_FILE_NAME, config_path.display());
    }

    // Init template build.rs and metadata
    init_metadata::handle(args.metadata_args).await?;

    Ok(())
}

// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use clap::Parser;

use crate::cli::commands::config;
use crate::cli::commands::template::init_metadata::{self, InitMetadataArgs};

#[derive(Clone, Parser, Debug)]
pub struct InitArgs {
    #[clap(flatten)]
    pub metadata_args: InitMetadataArgs,
}

pub async fn handle(args: InitArgs) -> anyhow::Result<()> {
    // Init project config (tari.config.toml)
    config::handle(config::ConfigCommand::Init).await?;

    // Init template build.rs and metadata
    init_metadata::handle(args.metadata_args).await?;

    Ok(())
}

// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use std::path::PathBuf;

use clap::Parser;

use crate::cli::commands::publish::build_template;
use crate::cli::util;

#[derive(Clone, Parser, Debug)]
pub struct BuildArgs {
    /// Path to the template crate directory.
    /// Defaults to the current directory.
    #[arg(default_value = ".")]
    pub path: PathBuf,
}

pub async fn handle(args: BuildArgs) -> anyhow::Result<()> {
    let wasm_path = build_template(&args.path).await?;
    let size = tokio::fs::metadata(&wasm_path).await?.len() as usize;

    println!("✅ WASM binary: {} ({})", wasm_path.display(), util::human_bytes(size));

    Ok(())
}

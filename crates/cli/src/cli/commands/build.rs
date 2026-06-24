// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use std::path::PathBuf;

use clap::Parser;

use crate::cli::commands::publish::{build_template, find_metadata_cbor};
use crate::cli::util;

#[derive(Clone, Parser, Debug)]
pub struct BuildArgs {
    /// Path to the template crate directory.
    /// Defaults to the current directory.
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Skip the size-optimizing release profile overrides passed to `cargo build`.
    /// By default the template is compiled with size optimizations.
    #[arg(long, default_value_t = false)]
    pub no_cargo_opts: bool,
}

pub async fn handle(args: BuildArgs) -> anyhow::Result<()> {
    let wasm_path = build_template(&args.path, !args.no_cargo_opts).await?;
    let size = tokio::fs::metadata(&wasm_path).await?.len() as usize;

    println!("✅ WASM binary: {} ({})", wasm_path.display(), util::human_bytes(size));

    match find_metadata_cbor(&args.path).await {
        Ok(path) => println!("📄 Metadata:    {}", path.display()),
        Err(e) => println!("📄 Metadata:    none ({e})"),
    }

    Ok(())
}

// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use std::path::{Path, PathBuf};

use clap::Parser;

use crate::cli::commands::publish::{build_template, find_target_dir};
use crate::cli::util;

const METADATA_CBOR_FILENAME: &str = "template_metadata.cbor";

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

    match find_metadata_cbor(&args.path).await {
        Ok(path) => println!("📄 Metadata:    {}", path.display()),
        Err(_) => println!("📄 Metadata:    none"),
    }

    Ok(())
}

async fn find_metadata_cbor(project_dir: &Path) -> anyhow::Result<PathBuf> {
    let target_dir = find_target_dir(project_dir).await?;
    let build_dir = target_dir.join("wasm32-unknown-unknown").join("release").join("build");

    if !build_dir.exists() {
        anyhow::bail!("build directory not found");
    }

    for entry in std::fs::read_dir(&build_dir)? {
        let entry = entry?;
        let out_dir = entry.path().join("out").join(METADATA_CBOR_FILENAME);
        if out_dir.exists() {
            return Ok(out_dir);
        }
    }

    anyhow::bail!("no metadata CBOR found")
}

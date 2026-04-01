// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use std::io::BufReader;
use std::path::{Path, PathBuf};

use anyhow::{Context, anyhow};
use clap::Parser;
use tari_ootle_template_metadata::TemplateMetadata;

const METADATA_CBOR_FILENAME: &str = "template_metadata.cbor";

#[derive(Clone, Parser, Debug)]
pub struct InspectMetadataArgs {
    /// Path to the metadata CBOR file.
    /// If not provided, searches the build output directory.
    #[arg()]
    pub path: Option<PathBuf>,

    /// Project directory to search for build output (used when path is not provided).
    #[arg(long, default_value = ".")]
    pub project_dir: PathBuf,

    /// Output as JSON instead of human-readable format.
    #[arg(long)]
    pub json: bool,
}

pub async fn handle(args: InspectMetadataArgs) -> anyhow::Result<()> {
    let cbor_path = match args.path {
        Some(p) => p,
        None => find_metadata_cbor(&args.project_dir)?,
    };

    if !cbor_path.exists() {
        return Err(anyhow!("Metadata file not found at {}", cbor_path.display()));
    }

    println!("📄 Reading metadata from {}", cbor_path.display());

    let file = std::fs::File::open(&cbor_path).context("opening metadata CBOR file")?;
    let reader = BufReader::new(file);
    let metadata = TemplateMetadata::read_cbor_from(reader).context("decoding metadata CBOR")?;
    let hash = metadata.hash().context("computing metadata hash")?;

    if args.json {
        let json = metadata.to_json().context("serializing to JSON")?;
        println!("{json}");
        eprintln!("\nMetadata hash: {hash}");
    } else {
        print_metadata_table(&metadata, &hash);
    }

    Ok(())
}

fn find_metadata_cbor(project_dir: &Path) -> anyhow::Result<PathBuf> {
    let build_dir = project_dir
        .join("target")
        .join("wasm32-unknown-unknown")
        .join("release")
        .join("build");

    if !build_dir.exists() {
        return Err(anyhow!(
            "Build output directory not found at {}. Run `cargo build --target wasm32-unknown-unknown --release` first.",
            build_dir.display()
        ));
    }

    // Search build output directories for the metadata CBOR file
    for entry in std::fs::read_dir(&build_dir).context("reading build directory")? {
        let entry = entry?;
        let out_dir = entry.path().join("out").join(METADATA_CBOR_FILENAME);
        if out_dir.exists() {
            return Ok(out_dir);
        }
    }

    Err(anyhow!(
        "No {METADATA_CBOR_FILENAME} found in build output. \
         Make sure the template uses tari_ootle_template_build in build.rs \
         and has been built with `cargo build --target wasm32-unknown-unknown --release`."
    ))
}

fn print_metadata_table(metadata: &TemplateMetadata, hash: &tari_ootle_template_metadata::MetadataHash) {
    println!();
    println!("  Name:           {}", metadata.name);
    println!("  Version:        {}", metadata.version);
    println!("  Schema version: {}", metadata.schema_version);
    if !metadata.description.is_empty() {
        println!("  Description:    {}", metadata.description);
    }
    if let Some(ref category) = metadata.category {
        println!("  Category:       {category}");
    }
    if !metadata.tags.is_empty() {
        println!("  Tags:           {}", metadata.tags.join(", "));
    }
    if let Some(ref license) = metadata.license {
        println!("  License:        {license}");
    }
    if let Some(ref repo) = metadata.repository {
        println!("  Repository:     {repo}");
    }
    if let Some(ref docs) = metadata.documentation {
        println!("  Documentation:  {docs}");
    }
    if let Some(ref homepage) = metadata.homepage {
        println!("  Homepage:       {homepage}");
    }
    if !metadata.extra.is_empty() {
        println!("  Extra:");
        for (key, value) in &metadata.extra {
            println!("    {key}: {value}");
        }
    }
    println!();
    println!("  Metadata hash:  {hash}");
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_metadata_table() {
        let metadata = TemplateMetadata::new("test-template".to_string(), "1.0.0".to_string());
        let hash = metadata.hash().unwrap();
        // Just ensure it doesn't panic
        print_metadata_table(&metadata, &hash);
    }
}

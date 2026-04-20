// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use std::path::PathBuf;

use anyhow::{Context, anyhow};
use clap::Parser;
use dialoguer::Confirm;
use tari_ootle_template_metadata::TemplateMetadata;

use crate::cli::commands::publish::{build_template, find_metadata_cbor};

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
        None => find_metadata_cbor(&args.project_dir).await?,
    };

    if !cbor_path.exists() {
        return Err(anyhow!("Metadata file not found at {}", cbor_path.display()));
    }

    println!("📄 Reading metadata from {}", cbor_path.display());

    let mut cbor_bytes = std::fs::read(&cbor_path).context("reading metadata CBOR file")?;
    let mut metadata = TemplateMetadata::from_cbor(&cbor_bytes).context("decoding metadata CBOR")?;

    // Check if built metadata matches Cargo.toml
    let cargo_toml_path = args.project_dir.join("Cargo.toml");
    if cargo_toml_path.exists() {
        match tari_ootle_template_metadata::from_cargo_toml(&cargo_toml_path) {
            Ok(current) if current != metadata => {
                println!("⚠️  Built metadata does not match Cargo.toml (metadata may be stale)");
                let rebuild = Confirm::new()
                    .with_prompt("Rebuild to update metadata?")
                    .default(true)
                    .interact()?;
                if rebuild {
                    build_template(&args.project_dir).await?;
                    let new_cbor_path = find_metadata_cbor(&args.project_dir).await?;
                    cbor_bytes = std::fs::read(&new_cbor_path).context("reading rebuilt metadata CBOR")?;
                    metadata = TemplateMetadata::from_cbor(&cbor_bytes).context("rebuilt metadata CBOR is invalid")?;
                    println!("✅ Metadata rebuilt");
                }
            },
            Ok(_) => {},
            Err(e) => {
                println!("⚠️  Could not read Cargo.toml metadata for freshness check: {e}");
            },
        }
    }

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
    if let Some(ref commit_hash) = metadata.commit_hash {
        println!("  Commit hash:    {commit_hash}");
    }
    if let Some(ref docs) = metadata.documentation {
        println!("  Documentation:  {docs}");
    }
    if let Some(ref homepage) = metadata.homepage {
        println!("  Homepage:       {homepage}");
    }
    if let Some(ref logo_url) = metadata.logo_url {
        println!("  Logo URL:       {logo_url}");
    }
    if let Some(ref supersedes) = metadata.supersedes {
        println!("  Supersedes:     {supersedes}");
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

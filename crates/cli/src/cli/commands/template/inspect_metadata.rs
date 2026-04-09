// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use std::io::BufReader;
use std::path::PathBuf;

use anyhow::{Context, anyhow};
use clap::Parser;
use tari_ootle_template_metadata::TemplateMetadata;

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
        None => crate::cli::commands::publish::find_metadata_cbor(&args.project_dir).await?,
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
    if let Some(ref logo_url) = metadata.logo_url {
        println!("  Logo URL:       {logo_url}");
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

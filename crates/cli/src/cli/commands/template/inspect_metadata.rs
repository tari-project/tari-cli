// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use std::path::PathBuf;

use anyhow::{Context, anyhow};
use clap::Parser;
use dialoguer::Confirm;
use tari_ootle_template_metadata::{FunctionDoc, TemplateMetadata};

use crate::cli::commands::publish::{build_template, decode_metadata_cbor, find_metadata_cbor};

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

    /// Show full metadata, including extracted rustdoc comments for each public template function.
    #[arg(long, short = 'f')]
    pub full: bool,
}

pub async fn handle(args: InspectMetadataArgs) -> anyhow::Result<()> {
    let cbor_path = match args.path {
        Some(p) => p,
        None => find_metadata_cbor(&args.project_dir).await?,
    };

    if !cbor_path.exists() {
        return Err(anyhow!("Metadata file not found at {}", cbor_path.display()));
    }

    eprintln!("📄 Reading metadata from {}", cbor_path.display());

    let mut cbor_bytes = std::fs::read(&cbor_path).context("reading metadata CBOR file")?;
    let mut metadata = decode_metadata_cbor(&cbor_bytes)?;

    // Check if built metadata matches Cargo.toml. `functions` is extracted from rustdoc at
    // build time and is never present in `from_cargo_toml` output, so exclude it from the
    // comparison or every template with documented functions would appear stale.
    let cargo_toml_path = args.project_dir.join("Cargo.toml");
    if cargo_toml_path.exists() {
        let built_without_functions = TemplateMetadata {
            functions: Vec::new(),
            ..metadata.clone()
        };
        match tari_ootle_template_metadata::from_cargo_toml(&cargo_toml_path) {
            Ok(current) if current != built_without_functions => {
                eprintln!("⚠️  Built metadata does not match Cargo.toml (metadata may be stale)");
                // Skip the interactive prompt in JSON mode so stdout stays pipeable.
                let rebuild = !args.json
                    && Confirm::new()
                        .with_prompt("Rebuild to update metadata?")
                        .default(true)
                        .interact()?;
                if rebuild {
                    build_template(&args.project_dir).await?;
                    let new_cbor_path = find_metadata_cbor(&args.project_dir).await?;
                    cbor_bytes = std::fs::read(&new_cbor_path).context("reading rebuilt metadata CBOR")?;
                    metadata = decode_metadata_cbor(&cbor_bytes)?;
                    eprintln!("✅ Metadata rebuilt");
                }
            },
            Ok(_) => {},
            Err(e) => {
                eprintln!("⚠️  Could not read Cargo.toml metadata for freshness check: {e}");
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
        if args.full {
            print_function_docs(&metadata.functions);
        }
    }

    Ok(())
}

fn print_function_docs(functions: &[FunctionDoc]) {
    use termimad::MadSkin;
    use termimad::crossterm::style::Color;

    let mut skin = MadSkin::default();
    skin.bold.set_fg(Color::Magenta);
    skin.italic.set_fg(Color::DarkGrey);

    let total = functions.len();
    let documented: Vec<&FunctionDoc> = functions.iter().filter(|f| !f.doc.is_empty()).collect();
    let undocumented: Vec<&FunctionDoc> = functions.iter().filter(|f| f.doc.is_empty()).collect();

    println!();
    skin.print_inline(&format!(
        "  **Functions** ({}/{} documented)\n",
        documented.len(),
        total
    ));

    if total == 0 {
        println!("    *(none)*");
        println!();
        return;
    }

    if !documented.is_empty() {
        println!();
        for func in &documented {
            skin.print_inline(&format!("    **fn {}**\n", func.name));
            for line in func.doc.lines() {
                println!("      {line}");
            }
            println!();
        }
    }

    if !undocumented.is_empty() {
        skin.print_inline("    *Undocumented:*\n");
        let names: Vec<String> = undocumented.iter().map(|f| f.name.clone()).collect();
        for line in wrap_names(&names, 4, 76) {
            println!("      {line}");
        }
        println!();
    }
}

fn wrap_names(names: &[String], indent: usize, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for name in names {
        let candidate = if current.is_empty() {
            name.clone()
        } else {
            format!("{current}, {name}")
        };
        if candidate.len() + indent > width && !current.is_empty() {
            lines.push(format!("{current},"));
            current = name.clone();
        } else {
            current = candidate;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
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

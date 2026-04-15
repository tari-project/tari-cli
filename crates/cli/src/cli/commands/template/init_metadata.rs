// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use std::path::{Path, PathBuf};

use anyhow::{Context, anyhow};
use clap::Parser;
use dialoguer::Input;
use tokio::fs;

const BUILD_DEP_KEY: &str = "tari_ootle_template_build";
const BUILD_DEP_VERSION: &str = "0.5";
const BUILD_RS_CONTENT: &str = r#"fn main() {
    tari_ootle_template_build::TemplateMetadataBuilder::new()
        .build()
        .expect("Failed to build template metadata");
}
"#;

const TARI_TEMPLATE_METADATA_KEY: &str = "tari-template";

#[derive(Clone, Parser, Debug)]
pub struct InitMetadataArgs {
    /// Path to the template crate directory (containing Cargo.toml).
    /// Defaults to the current directory.
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Template description (written to [package].description).
    #[arg(long)]
    pub description: Option<String>,

    /// Comma-separated tags (e.g. "token,fungible,defi").
    #[arg(long, value_delimiter = ',')]
    pub tags: Vec<String>,

    /// Template category (e.g. "token", "nft", "defi").
    #[arg(long)]
    pub category: Option<String>,

    /// Documentation URL.
    #[arg(long)]
    pub documentation: Option<String>,

    /// Homepage URL.
    #[arg(long)]
    pub homepage: Option<String>,

    /// Logo URL (e.g. a link to the template's icon or logo image).
    #[arg(long)]
    pub logo_url: Option<String>,

    /// Template address of a previous version that this template supersedes (64-char hex).
    #[arg(long)]
    pub supersedes: Option<String>,

    /// Skip interactive prompts (use only provided CLI args).
    #[arg(long, short = 'y')]
    pub non_interactive: bool,
}

pub async fn handle(args: InitMetadataArgs) -> anyhow::Result<()> {
    let crate_dir = &args.path;
    let cargo_toml_path = crate_dir.join("Cargo.toml");
    if !cargo_toml_path.exists() {
        return Err(anyhow!("No Cargo.toml found at {}", cargo_toml_path.display()));
    }

    let cargo_toml_content = fs::read_to_string(&cargo_toml_path)
        .await
        .context("reading Cargo.toml")?;

    let metadata = resolve_metadata(&args)?;
    let updated = add_build_dependency(&cargo_toml_content)?;
    let updated = add_template_metadata(&updated, &metadata)?;

    fs::write(&cargo_toml_path, &updated)
        .await
        .context("writing Cargo.toml")?;
    println!("✅ Cargo.toml updated");

    let build_rs_path = crate_dir.join("build.rs");
    update_build_rs(&build_rs_path).await?;

    println!("🎉 Metadata generation configured. Run `cargo build` to generate metadata.");
    Ok(())
}

struct TemplateMetadataInput {
    description: Option<String>,
    tags: Vec<String>,
    category: Option<String>,
    documentation: Option<String>,
    homepage: Option<String>,
    logo_url: Option<String>,
    supersedes: Option<String>,
}

fn resolve_metadata(args: &InitMetadataArgs) -> anyhow::Result<TemplateMetadataInput> {
    // Check if [package].description already exists
    let cargo_toml_path = args.path.join("Cargo.toml");
    let has_description = if cargo_toml_path.exists() {
        let content = std::fs::read_to_string(&cargo_toml_path)?;
        let doc = content.parse::<toml_edit::DocumentMut>()?;
        doc.get("package")
            .and_then(|p| p.get("description"))
            .and_then(|d| d.as_str())
            .is_some_and(|s| !s.is_empty())
    } else {
        false
    };

    if args.non_interactive {
        return Ok(TemplateMetadataInput {
            description: args.description.clone(),
            tags: args.tags.clone(),
            category: args.category.clone(),
            documentation: args.documentation.clone(),
            homepage: args.homepage.clone(),
            logo_url: args.logo_url.clone(),
            supersedes: args.supersedes.clone(),
        });
    }

    // Prompt for description if not already in [package]
    let description = if has_description {
        None
    } else {
        let desc: String = Input::new()
            .with_prompt("Description")
            .default(args.description.clone().unwrap_or_default())
            .allow_empty(true)
            .interact_text()?;
        if desc.is_empty() { None } else { Some(desc) }
    };

    // Interactive prompts, using CLI args as defaults
    let tags_default = args.tags.join(", ");
    let tags_input: String = Input::new()
        .with_prompt("Tags (comma-separated, e.g. token,fungible,defi)")
        .default(tags_default)
        .allow_empty(true)
        .interact_text()?;
    let tags: Vec<String> = tags_input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let category: String = Input::new()
        .with_prompt("Category (e.g. token, nft, defi)")
        .default(args.category.clone().unwrap_or_default())
        .allow_empty(true)
        .interact_text()?;
    let category = if category.is_empty() { None } else { Some(category) };

    let documentation: String = Input::new()
        .with_prompt("Documentation URL")
        .default(args.documentation.clone().unwrap_or_default())
        .allow_empty(true)
        .interact_text()?;
    let documentation = if documentation.is_empty() {
        None
    } else {
        Some(documentation)
    };

    let homepage: String = Input::new()
        .with_prompt("Homepage URL")
        .default(args.homepage.clone().unwrap_or_default())
        .allow_empty(true)
        .interact_text()?;
    let homepage = if homepage.is_empty() { None } else { Some(homepage) };

    let logo_url: String = Input::new()
        .with_prompt("Logo URL")
        .default(args.logo_url.clone().unwrap_or_default())
        .allow_empty(true)
        .interact_text()?;
    let logo_url = if logo_url.is_empty() { None } else { Some(logo_url) };

    let supersedes: String = Input::new()
        .with_prompt("Supersedes template address (64-char hex, leave empty to skip)")
        .default(args.supersedes.clone().unwrap_or_default())
        .allow_empty(true)
        .interact_text()?;
    let supersedes = if supersedes.is_empty() { None } else { Some(supersedes) };

    Ok(TemplateMetadataInput {
        description,
        tags,
        category,
        documentation,
        homepage,
        logo_url,
        supersedes,
    })
}

fn add_build_dependency(cargo_toml_content: &str) -> anyhow::Result<String> {
    let mut doc = cargo_toml_content
        .parse::<toml_edit::DocumentMut>()
        .context("parsing Cargo.toml")?;

    let build_deps = doc
        .entry("build-dependencies")
        .or_insert_with(|| toml_edit::Item::Table(toml_edit::Table::new()));
    let build_deps = build_deps
        .as_table_mut()
        .ok_or_else(|| anyhow!("[build-dependencies] is not a table"))?;

    if build_deps.contains_key(BUILD_DEP_KEY) {
        println!("ℹ️  {BUILD_DEP_KEY} already in [build-dependencies], skipping");
    } else {
        build_deps.insert(BUILD_DEP_KEY, toml_edit::value(BUILD_DEP_VERSION));
    }

    Ok(doc.to_string())
}

fn add_template_metadata(cargo_toml_content: &str, metadata: &TemplateMetadataInput) -> anyhow::Result<String> {
    let mut doc = cargo_toml_content
        .parse::<toml_edit::DocumentMut>()
        .context("parsing Cargo.toml")?;

    let package = doc
        .get_mut("package")
        .and_then(|p| p.as_table_mut())
        .ok_or_else(|| anyhow!("missing [package] section"))?;

    // Write description to [package].description
    if let Some(ref description) = metadata.description {
        package.insert("description", toml_edit::value(description.as_str()));
    }

    // Navigate to [package.metadata.tari-template]
    let pkg_metadata = package
        .entry("metadata")
        .or_insert_with(|| toml_edit::Item::Table(toml_edit::Table::new()))
        .as_table_mut()
        .ok_or_else(|| anyhow!("[package.metadata] is not a table"))?;
    pkg_metadata.set_dotted(true);

    let tari_template = pkg_metadata
        .entry(TARI_TEMPLATE_METADATA_KEY)
        .or_insert_with(|| toml_edit::Item::Table(toml_edit::Table::new()))
        .as_table_mut()
        .ok_or_else(|| anyhow!("[package.metadata.tari-template] is not a table"))?;

    if !metadata.tags.is_empty() {
        let mut arr = toml_edit::Array::new();
        for tag in &metadata.tags {
            arr.push(tag.as_str());
        }
        tari_template.insert("tags", toml_edit::value(arr));
    }

    if let Some(ref category) = metadata.category {
        tari_template.insert("category", toml_edit::value(category.as_str()));
    }

    if let Some(ref documentation) = metadata.documentation {
        tari_template.insert("documentation", toml_edit::value(documentation.as_str()));
    }

    if let Some(ref homepage) = metadata.homepage {
        tari_template.insert("homepage", toml_edit::value(homepage.as_str()));
    }

    if let Some(ref logo_url) = metadata.logo_url {
        tari_template.insert("logo_url", toml_edit::value(logo_url.as_str()));
    }

    if let Some(ref supersedes) = metadata.supersedes {
        tari_template.insert("supersedes", toml_edit::value(supersedes.as_str()));
    }

    Ok(doc.to_string())
}

/// Auto-initialise metadata for a freshly-scaffolded crate.
///
/// Adds the build dependency, creates `build.rs`, and writes a
/// `[package.metadata.tari-template]` section (empty — the user can fill it in
/// later with `tari template init` or by editing Cargo.toml directly).
pub async fn auto_init(crate_dir: &Path) -> anyhow::Result<()> {
    let cargo_toml_path = crate_dir.join("Cargo.toml");
    if !cargo_toml_path.exists() {
        return Err(anyhow!("No Cargo.toml found at {}", cargo_toml_path.display()));
    }

    let content = fs::read_to_string(&cargo_toml_path)
        .await
        .context("reading Cargo.toml")?;

    let empty_metadata = TemplateMetadataInput {
        description: None,
        tags: vec![],
        category: None,
        documentation: None,
        homepage: None,
        logo_url: None,
        supersedes: None,
    };

    let updated = add_build_dependency(&content)?;
    let updated = add_template_metadata(&updated, &empty_metadata)?;
    fs::write(&cargo_toml_path, &updated)
        .await
        .context("writing Cargo.toml")?;

    update_build_rs(&crate_dir.join("build.rs")).await?;
    Ok(())
}

async fn update_build_rs(build_rs_path: &Path) -> anyhow::Result<()> {
    if !build_rs_path.exists() {
        fs::write(build_rs_path, BUILD_RS_CONTENT)
            .await
            .context("creating build.rs")?;
        println!("✅ Created build.rs with TemplateMetadataBuilder");
        return Ok(());
    }

    let existing = fs::read_to_string(build_rs_path).await.context("reading build.rs")?;

    if existing.contains("TemplateMetadataBuilder") {
        println!("ℹ️  build.rs already contains TemplateMetadataBuilder, skipping");
        return Ok(());
    }

    // Existing build.rs without our content — don't modify it, let the user handle it
    println!(
        "⚠️  build.rs already exists at {} but does not contain TemplateMetadataBuilder.",
        build_rs_path.display()
    );
    println!("   Please add the following to your build.rs main function:");
    println!();
    println!("   tari_ootle_template_build::TemplateMetadataBuilder::new()");
    println!("       .build()");
    println!("       .expect(\"Failed to build template metadata\");");
    println!();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_build_dependency() {
        let input = r#"[package]
name = "my-template"
version = "0.1.0"

[dependencies]
foo = "1.0"
"#;
        let result = add_build_dependency(input).unwrap();
        assert!(result.contains("[build-dependencies]"));
        assert!(result.contains("tari_ootle_template_build"));
    }

    #[test]
    fn idempotent_build_dependency() {
        let input = r#"[package]
name = "my-template"
version = "0.1.0"

[build-dependencies]
tari_ootle_template_build = "0.3"
"#;
        let result = add_build_dependency(input).unwrap();
        assert_eq!(result.matches("tari_ootle_template_build").count(), 1);
    }

    #[test]
    fn adds_template_metadata_section() {
        let input = r#"[package]
name = "my-template"
version = "0.1.0"
"#;
        let metadata = TemplateMetadataInput {
            description: None,
            tags: vec!["token".to_string(), "defi".to_string()],
            category: Some("token".to_string()),
            documentation: None,
            homepage: Some("https://example.com".to_string()),
            logo_url: None,
            supersedes: None,
        };
        let result = add_template_metadata(input, &metadata).unwrap();
        assert!(result.contains("[package.metadata.tari-template]"));
        assert!(result.contains("token"));
        assert!(result.contains("defi"));
        assert!(result.contains("category"));
        assert!(result.contains("https://example.com"));
    }

    #[test]
    fn idempotent_metadata_overwrites_values() {
        let input = r#"[package]
name = "my-template"
version = "0.1.0"

[package.metadata.tari-template]
tags = ["old"]
category = "old-category"
"#;
        let metadata = TemplateMetadataInput {
            description: None,
            tags: vec!["new".to_string()],
            category: Some("new-category".to_string()),
            documentation: None,
            homepage: None,
            logo_url: None,
            supersedes: None,
        };
        let result = add_template_metadata(input, &metadata).unwrap();
        assert!(result.contains("new-category"));
        assert!(!result.contains("old-category"));
    }

    #[test]
    fn empty_metadata_leaves_section_minimal() {
        let input = r#"[package]
name = "my-template"
version = "0.1.0"
"#;
        let metadata = TemplateMetadataInput {
            description: None,
            tags: vec![],
            category: None,
            documentation: None,
            homepage: None,
            logo_url: None,
            supersedes: None,
        };
        let result = add_template_metadata(input, &metadata).unwrap();
        // Should still create the section even if empty
        assert!(result.contains("[package.metadata.tari-template]"));
    }
}

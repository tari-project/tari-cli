// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use std::path::PathBuf;

use anyhow::Context;
use dialoguer::{Confirm, Input};

use crate::cli::commands::config::{ConfigCommand, resolve_config_path};
use crate::cli::commands::template::init_metadata;
use crate::project::CONFIG_FILE_NAME;

pub async fn handle() -> anyhow::Result<()> {
    println!();
    println!("🚀 Welcome to the Tari CLI setup wizard");
    println!();

    let cwd = std::env::current_dir()?;

    // Step 1: Template crate
    let crate_dir = step_template_crate(&cwd).await?;

    // Step 2: Project config (tari.config.toml)
    step_project_config(&crate_dir).await?;

    // Step 3: Template metadata
    step_metadata(&crate_dir).await?;

    // Summary
    println!();
    println!("🎉 You're all set! Here are some next steps:");
    println!();
    println!("   Build your template:");
    println!("     cargo build --target wasm32-unknown-unknown --release");
    println!();
    println!("   Inspect generated metadata:");
    println!("     tari template inspect");
    println!();
    println!("   Publish to the network:");
    println!("     tari template publish");
    println!();

    Ok(())
}

async fn step_template_crate(cwd: &PathBuf) -> anyhow::Result<PathBuf> {
    let cargo_toml = cwd.join("Cargo.toml");
    if cargo_toml.exists() {
        let manifest = cargo_toml::Manifest::from_path(&cargo_toml)?;
        if let Some(pkg) = &manifest.package {
            println!("✅ Found template crate: {}", pkg.name);
            return Ok(cwd.clone());
        }
    }

    println!("📦 No template crate found in the current directory.");

    let should_create = Confirm::new()
        .with_prompt("Create a new template crate here?")
        .default(true)
        .interact()?;

    if !should_create {
        println!("ℹ️  Skipping crate creation. Run `tari create <name>` when you're ready.");
        return Ok(cwd.clone());
    }

    let name: String = Input::new().with_prompt("Template crate name").interact_text()?;

    let name = convert_case::Casing::to_case(&name, convert_case::Case::Snake);

    // We need to do the full create flow: refresh repo, select template, generate
    let config = crate::cli::config::Config::default();
    let base_dir = crate::cli::command::default_base_dir();
    crate::cli::util::create_dir(&base_dir).await?;

    // Minimal repo refresh inline
    let repo_dir = refresh_template_repo(&base_dir, &config.template_repository).await?;

    let args = crate::cli::commands::create::CreateArgs {
        name: name.clone(),
        template: None,
        output: cwd.clone(),
        skip_init: false,
        skip_metadata: true, // We'll handle metadata in step 3
        verbose: false,
    };

    crate::cli::commands::create::handle(config, repo_dir, args).await?;

    Ok(cwd.join(&name))
}

async fn step_project_config(_crate_dir: &PathBuf) -> anyhow::Result<()> {
    let config_path = resolve_config_path()?;

    if config_path.exists() {
        println!("✅ Config found at {}", config_path.display());
        return Ok(());
    }

    println!("⚙️  No {} found.", CONFIG_FILE_NAME);

    let should_create = Confirm::new()
        .with_prompt("Create project configuration?")
        .default(true)
        .interact()?;

    if !should_create {
        println!("ℹ️  Skipping. Run `tari config init` when you're ready.");
        return Ok(());
    }

    // Create default config
    let default = toml::to_string_pretty(&crate::project::ProjectConfig::default())?;
    tokio::fs::write(&config_path, &default)
        .await
        .context("writing config file")?;
    println!("✅ Created {}", config_path.display());

    // Ask for wallet daemon URL
    let url: String = Input::new()
        .with_prompt("Wallet daemon JSON-RPC URL")
        .default("http://127.0.0.1:9000/json_rpc".to_string())
        .interact_text()?;

    if url != "http://127.0.0.1:9000/json_rpc" {
        crate::cli::commands::config::handle(ConfigCommand::Set {
            key: "network.wallet-daemon-jrpc-address".to_string(),
            value: url,
        })
        .await?;
    }

    Ok(())
}

async fn step_metadata(crate_dir: &PathBuf) -> anyhow::Result<()> {
    let cargo_toml = crate_dir.join("Cargo.toml");
    if !cargo_toml.exists() {
        return Ok(());
    }

    // Check build.rs setup
    let has_build_rs = {
        let build_rs = crate_dir.join("build.rs");
        if build_rs.exists() {
            let content = tokio::fs::read_to_string(&build_rs).await?;
            content.contains("TemplateMetadataBuilder")
        } else {
            false
        }
    };

    // Check if metadata fields are populated (not just an empty section)
    let has_metadata_fields = {
        let content = tokio::fs::read_to_string(&cargo_toml).await?;
        if let Ok(doc) = content.parse::<toml_edit::DocumentMut>() {
            doc.get("package")
                .and_then(|p| p.get("metadata"))
                .and_then(|m| m.get("tari-template"))
                .and_then(|t| t.as_table())
                .is_some_and(|t| !t.is_empty())
        } else {
            false
        }
    };

    if has_build_rs && has_metadata_fields {
        println!("✅ Template metadata already configured");
        return Ok(());
    }

    if has_build_rs && !has_metadata_fields {
        println!("📄 Build script configured but metadata fields are empty.");
    } else {
        println!("📄 Template metadata not yet configured.");
    }

    let should_init = Confirm::new()
        .with_prompt("Set up template metadata now?")
        .default(true)
        .interact()?;

    if !should_init {
        println!("ℹ️  Skipping. Run `tari template init` when you're ready.");
        return Ok(());
    }

    let args = init_metadata::InitMetadataArgs {
        path: crate_dir.clone(),
        tags: vec![],
        category: None,
        documentation: None,
        homepage: None,
        non_interactive: false,
    };
    init_metadata::handle(args).await?;

    Ok(())
}

async fn refresh_template_repo(
    base_dir: &PathBuf,
    template_repo: &crate::cli::config::TemplateRepository,
) -> anyhow::Result<PathBuf> {
    use crate::cli::util;
    use crate::git::repository::GitRepository;

    let repos_dir = base_dir.join("template_repositories");
    util::create_dir(&repos_dir).await?;

    let repo_url_parts: Vec<&str> = template_repo.url.split("/").collect();
    let repo_name = repo_url_parts.last().context("Failed to get repo name from URL")?;
    let repo_user = repo_url_parts
        .get(repo_url_parts.len() - 2)
        .context("Failed to get repo owner from URL")?;
    let repo_path = repos_dir.join(repo_user).join(repo_name);
    let mut repo = GitRepository::new(repo_path.clone());

    if util::dir_exists(&repo_path).await? {
        repo.load()?;
        let current_branch = repo.current_branch_name()?;
        if current_branch != template_repo.branch {
            repo.pull_changes(Some(template_repo.branch.clone()))?;
        } else {
            repo.pull_changes(None)?;
        }
    } else {
        repo.clone_and_checkout(template_repo.url.as_str(), template_repo.branch.as_str())?;
    }

    Ok(repo.local_folder().join(&template_repo.folder))
}

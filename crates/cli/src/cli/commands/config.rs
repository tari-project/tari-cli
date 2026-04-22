// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use std::path::PathBuf;

use anyhow::{Context, anyhow};
use clap::Subcommand;
use tokio::fs;

use crate::project::CONFIG_FILE_NAME;

#[derive(Clone, Subcommand)]
pub enum ConfigCommand {
    /// Initialise a tari.config.toml in the project root.
    Init,
    /// Set a configuration value.
    Set {
        /// Configuration key (e.g. wallet-daemon-jrpc-address, default_account).
        key: String,
        /// Value to set.
        value: String,
    },
    /// Get a configuration value.
    Get {
        /// Configuration key.
        key: String,
    },
    /// Show the full configuration file.
    Show,
}

pub async fn handle(command: ConfigCommand) -> anyhow::Result<()> {
    match command {
        ConfigCommand::Init => handle_init().await,
        ConfigCommand::Set { key, value } => handle_set(&key, &value).await,
        ConfigCommand::Get { key } => handle_get(&key).await,
        ConfigCommand::Show => handle_show().await,
    }
}

async fn handle_init() -> anyhow::Result<()> {
    let config_path = resolve_config_path()?;
    if config_path.exists() {
        println!("ℹ️  {} already exists at {}", CONFIG_FILE_NAME, config_path.display());
        return Ok(());
    }

    let default = toml::to_string_pretty(&crate::project::ProjectConfig::default())?;
    fs::write(&config_path, &default).await.context("writing config file")?;
    println!("✅ Created {} at {}", CONFIG_FILE_NAME, config_path.display());
    Ok(())
}

async fn handle_set(key: &str, value: &str) -> anyhow::Result<()> {
    let config_path = resolve_config_path()?;
    if !config_path.exists() {
        // Auto-create with defaults
        let default = toml::to_string_pretty(&crate::project::ProjectConfig::default())?;
        fs::write(&config_path, &default)
            .await
            .context("creating config file")?;
        println!("✅ Created {} at {}", CONFIG_FILE_NAME, config_path.display());
    }

    let content = fs::read_to_string(&config_path).await.context("reading config")?;
    let mut doc = content.parse::<toml_edit::DocumentMut>().context("parsing config")?;

    set_dotted_key(&mut doc, key, value)?;

    fs::write(&config_path, doc.to_string())
        .await
        .context("writing config")?;
    println!("✅ Set {key} = {value}");
    Ok(())
}

async fn handle_get(key: &str) -> anyhow::Result<()> {
    let config_path = find_existing_config()?;
    let content = fs::read_to_string(&config_path).await.context("reading config")?;
    let doc = content.parse::<toml_edit::DocumentMut>().context("parsing config")?;

    let value = get_dotted_key(&doc, key)?;
    println!("{value}");
    Ok(())
}

async fn handle_show() -> anyhow::Result<()> {
    let config_path = find_existing_config()?;
    let content = fs::read_to_string(&config_path).await.context("reading config")?;
    println!("# {}\n", config_path.display());
    print!("{content}");
    Ok(())
}

/// Resolve where the config file should live:
/// crate root (Cargo.toml) if in one, then git repo root, otherwise CWD.
pub fn resolve_config_path() -> anyhow::Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    let root = if cwd.join("Cargo.toml").exists() {
        cwd
    } else {
        find_repo_root().unwrap_or(cwd)
    };
    Ok(root.join(CONFIG_FILE_NAME))
}

/// Find an existing config file by walking up from CWD.
fn find_existing_config() -> anyhow::Result<PathBuf> {
    let mut dir = std::env::current_dir()?;
    loop {
        let candidate = dir.join(CONFIG_FILE_NAME);
        if candidate.exists() {
            return Ok(candidate);
        }
        if !dir.pop() {
            break;
        }
    }
    Err(anyhow!(
        "No {} found. Run `tari config init` to create one.",
        CONFIG_FILE_NAME
    ))
}

pub fn find_repo_root() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        if dir.join(".git").exists() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Set an arbitrary dotted-path key in a TOML document.
///
/// Intermediate tables (anything except the leaf-holding table) are marked implicit so nested
/// structures render as `[a.b]` instead of `[a]\n[a.b]`.
pub fn set_dotted_key(doc: &mut toml_edit::DocumentMut, key: &str, value: &str) -> anyhow::Result<()> {
    let parts: Vec<&str> = key.split('.').collect();
    if parts.is_empty() || parts.iter().any(|p| p.is_empty()) {
        return Err(anyhow!("Empty or malformed key: '{key}'"));
    }

    if parts.len() == 1 {
        doc.insert(parts[0], toml_edit::value(value));
        return Ok(());
    }

    // Walk/create intermediate tables, then insert the leaf.
    let (leaf, head) = parts.split_last().expect("non-empty parts");
    let root = doc
        .entry(head[0])
        .or_insert_with(|| toml_edit::Item::Table(toml_edit::Table::new()))
        .as_table_mut()
        .ok_or_else(|| anyhow!("'{}' is not a table", head[0]))?;

    // Any table we traverse *through* (i.e. not the table holding the leaf) only contains
    // sub-tables, so render it implicit.
    if head.len() > 1 {
        root.set_implicit(true);
    }

    let last_idx = head.len().saturating_sub(2); // index in head[1..] of the leaf-holding table
    let mut table = root;
    for (i, part) in head[1..].iter().enumerate() {
        let entry = table
            .entry(part)
            .or_insert_with(|| toml_edit::Item::Table(toml_edit::Table::new()));
        table = entry.as_table_mut().ok_or_else(|| anyhow!("'{part}' is not a table"))?;
        if i < last_idx {
            table.set_implicit(true);
        }
    }
    table.insert(leaf, toml_edit::value(value));
    Ok(())
}

fn get_dotted_key(doc: &toml_edit::DocumentMut, key: &str) -> anyhow::Result<String> {
    let parts: Vec<&str> = key.split('.').collect();
    if parts.is_empty() {
        return Err(anyhow!("Empty key"));
    }

    let mut item: Option<&toml_edit::Item> = doc.get(parts[0]);
    for part in &parts[1..] {
        item = item.and_then(|i| i.get(part));
    }

    match item {
        Some(toml_edit::Item::Value(v)) => {
            let s = v.to_string();
            Ok(s.trim().trim_matches('"').to_string())
        },
        Some(toml_edit::Item::Table(t)) => Ok(t.to_string()),
        Some(other) => Ok(other.to_string()),
        None => Err(anyhow!("Key '{key}' not found")),
    }
}

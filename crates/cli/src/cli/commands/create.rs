// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use std::path::PathBuf;

use anyhow::anyhow;
use cargo_generate::{GenerateArgs as CargoGenerateArgs, TemplatePath};
use clap::Parser;
use dialoguer::Input;
use thiserror::Error;

use crate::cli::commands::template::init_metadata;
use crate::{
    cli::{command::project_name_parser, config::Config, util},
    git::repository::GitRepository,
    loading,
    templates::Collector,
};

#[derive(Clone, Parser, Debug)]
pub struct CreateArgs {
    /// Name of the new template crate.
    /// If not provided, you will be prompted to enter one.
    #[arg(value_parser = project_name_parser)]
    pub name: Option<String>,

    /// (Optional) Template to use (e.g. "fungible", "meme_coin").
    /// You will be prompted to select a template if not set.
    #[arg(short = 't', long)]
    pub template: Option<String>,

    /// Directory where the new crate will be created.
    #[arg(long, short = 'o', value_name = "PATH", default_value = crate::cli::command::default_output_dir().into_os_string())]
    pub output: PathBuf,

    /// Skip git init.
    #[arg(long, default_value_t = false)]
    pub skip_init: bool,

    /// Skip automatic template metadata initialisation.
    /// By default, new templates are set up with build.rs and
    /// [package.metadata.tari-template] for metadata generation.
    /// Use `tari template init` later to configure metadata interactively.
    #[arg(long, default_value_t = false)]
    pub skip_metadata: bool,

    /// Enables more verbose output.
    #[arg(long, short = 'v')]
    pub verbose: bool,
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum CreateHandlerError {
    #[error("Template not found by name: {0}. Possible values: {1:?}")]
    TemplateNotFound(String, Vec<String>),
}

/// Resolves a template by case-insensitive ID first, then falls back to case-insensitive display name.
pub fn resolve_template<'a>(
    templates: &'a [crate::templates::Template],
    query: &str,
) -> Result<&'a crate::templates::Template, CreateHandlerError> {
    let trimmed = query.trim();
    if !trimmed.is_empty() {
        if let Some(by_id) = templates.iter().rfind(|t| t.id().eq_ignore_ascii_case(trimmed)) {
            return Ok(by_id);
        }
        if let Some(by_name) = templates.iter().rfind(|t| t.name().eq_ignore_ascii_case(trimmed)) {
            return Ok(by_name);
        }
    }
    Err(CreateHandlerError::TemplateNotFound(
        trimmed.to_string(),
        templates.iter().map(|t| t.id().to_string()).collect(),
    ))
}

pub async fn handle(config: Config, template_repo_dir: PathBuf, mut args: CreateArgs) -> anyhow::Result<()> {
    let name = match args.name.take() {
        Some(name) => name,
        None => {
            let raw: String = Input::new().with_prompt("Template crate name").interact_text()?;
            project_name_parser(&raw).map_err(|e| anyhow!(e))?
        },
    };

    let templates = loading!(
        "Collecting available templates",
        Collector::new(template_repo_dir.join(&config.template_repository.folder))
            .collect()
            .await
    )?;

    let template = match &args.template {
        Some(template_id) => resolve_template(&templates, template_id)?,
        None => util::cli_select("🔎 Select a template", templates.as_slice())?,
    };

    let template_path = template
        .path()
        .to_str()
        .ok_or(anyhow!("Invalid template path!"))?
        .to_string();

    let generate_args = CargoGenerateArgs {
        name: Some(name.clone()),
        destination: Some(args.output.clone()),
        template_path: TemplatePath {
            path: Some(template_path),
            ..TemplatePath::default()
        },
        verbose: args.verbose,
        ..CargoGenerateArgs::default()
    };
    loading!("Generating template crate", cargo_generate::generate(generate_args))?;

    let crate_dir = args.output.join(&name);

    // initialise template metadata (build.rs + Cargo.toml metadata section)
    if !args.skip_metadata {
        loading!(
            format!("Initialising template metadata for **{}**", name),
            init_metadata::auto_init(&crate_dir).await
        )?;
    }

    if !args.skip_init
        && let Err(error) = GitRepository::new(crate_dir).init()
        && args.verbose
    {
        println!("ℹ️ Git repository already initialized: {error}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::templates::Template;

    fn sample_templates() -> Vec<Template> {
        vec![
            Template::new(
                PathBuf::from("/tmp/fungible"),
                "fungible".to_string(),
                "Fungible Token".to_string(),
                "A fungible token template".to_string(),
                HashMap::new(),
            ),
            Template::new(
                PathBuf::from("/tmp/meme_coin"),
                "meme_coin".to_string(),
                "fungible".to_string(),
                "Template whose display name collides with fungible id".to_string(),
                HashMap::new(),
            ),
        ]
    }

    #[test]
    fn resolve_template_prioritizes_id_over_name_and_trims_whitespace() {
        let templates = sample_templates();
        let matched = resolve_template(&templates, "  FUNGIBLE ").expect("should resolve by id");
        assert_eq!(matched.id(), "fungible");
        assert_eq!(matched.path(), &PathBuf::from("/tmp/fungible"));

        let by_name = resolve_template(&templates, "Fungible Token").expect("should resolve by display name");
        assert_eq!(by_name.id(), "fungible");
    }

    #[test]
    fn resolve_template_rejects_empty_or_unknown_names() {
        let templates = sample_templates();
        assert_eq!(
            resolve_template(&templates, "   "),
            Err(CreateHandlerError::TemplateNotFound(
                String::new(),
                vec!["fungible".to_string(), "meme_coin".to_string()]
            ))
        );
        assert_eq!(
            resolve_template(&templates, "nonexistent"),
            Err(CreateHandlerError::TemplateNotFound(
                "nonexistent".to_string(),
                vec!["fungible".to_string(), "meme_coin".to_string()]
            ))
        );
    }
}

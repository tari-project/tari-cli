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

#[derive(Error, Debug)]
pub enum CreateHandlerError {
    #[error("Template not found by name: {0}. Possible values: {1:?}")]
    TemplateNotFound(String, Vec<String>),
}

pub async fn handle(config: Config, template_repo_dir: PathBuf, mut args: CreateArgs) -> anyhow::Result<()> {
    let name = match args.name.take() {
        Some(name) => name,
        None => {
            let raw: String = Input::new()
                .with_prompt("Template crate name")
                .interact_text()?;
            project_name_parser(&raw).map_err(|e| anyhow!(e))?
        }
    };

    let templates = loading!(
        "Collecting available templates",
        Collector::new(template_repo_dir.join(&config.template_repository.folder))
            .collect()
            .await
    )?;

    let template = match &args.template {
        Some(template_id) => templates
            .iter()
            .rfind(|t| t.id().eq_ignore_ascii_case(template_id) || t.name().eq_ignore_ascii_case(template_id))
            .ok_or_else(|| {
                CreateHandlerError::TemplateNotFound(
                    template_id.to_string(),
                    templates.iter().map(|t| t.id().to_string()).collect(),
                )
            })?,
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

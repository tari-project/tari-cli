// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use std::path::PathBuf;

use crate::cli::commands::add;
use crate::cli::commands::add::AddArgs;
use crate::git::find_git_root;
use crate::{
    cli::{config::Config, util},
    git::repository::GitRepository,
    loading, md_println, project,
    templates::Collector,
};
use anyhow::anyhow;
use cargo_generate::{GenerateArgs as CargoGenerateArgs, TemplatePath};
use clap::Parser;
use thiserror::Error;
use tokio::fs;

const PROJECT_TEMPLATE_EXTRA_TEMPLATES_FIELD_NAME: &str = "templates_dir";
const PROJECT_TEMPLATE_EXTRA_INIT_WASM_TEMPLATES_FIELD_NAME: &str = "wasm_templates";

#[derive(Clone, Parser, Debug)]
pub struct CreateArgs {
    /// Name of the project
    #[arg(value_parser = crate::cli::command::project_name_parser)]
    pub name: String,

    /// (Optional) Selected project template (ID).
    /// It will be prompted if not set.
    #[arg(short = 't', long)]
    pub template: Option<String>,

    /// Directory where the new generated project will be output.
    #[arg(long, short = 'o', value_name = "PATH", default_value = crate::cli::command::default_output_dir().into_os_string())]
    pub output: PathBuf,

    /// Skip git init.
    #[arg(long, default_value = "false")]
    pub skip_init: bool,

    /// Enables more verbose output.
    #[arg(long, short = 'v')]
    pub verbose: bool,
}

#[derive(Error, Debug)]
pub enum CreateHandlerError {
    #[error("Template not found by name: {0}. Possible values: {1:?}")]
    TemplateNotFound(String, Vec<String>),
}

/// Handle `create` command.
/// It creates a new Tari template development project.
pub async fn handle(
    config: Config,
    project_template_repo: GitRepository,
    wasm_template_repo: GitRepository,
    args: CreateArgs,
) -> anyhow::Result<()> {
    // is the output a git repository?
    let project_root = if let Some(root) = find_git_root(&args.output) {
        if args.verbose {
            md_println!("ℹ️ Output directory `{}` is a git repository.", root.display());
        }
        md_println!(
            "⚠️ Creating a new project `{}` in the git repository `{}`. You may want to use `tari add` instead.",
            args.name,
            root.display()
        );

        root
    } else {
        if args.verbose {
            md_println!(
                "ℹ️ Output directory `{}` is not a git repository.",
                args.output.display()
            );
        }
        let path = args.output.join(args.name.as_str());
        fs::create_dir_all(&path).await?;
        path
    };

    // selecting project template
    let templates = loading!(
        "Collecting available project templates",
        Collector::new(
            project_template_repo
                .local_folder()
                .join(config.project_template_repository.folder.clone()),
        )
        .collect()
        .await
    )?;

    let template = match &args.template {
        Some(template_id) => templates
            .iter()
            .rfind(|template| template.id().eq_ignore_ascii_case(template_id))
            .ok_or(CreateHandlerError::TemplateNotFound(
                template_id.to_string(),
                templates.iter().map(|template| template.id().to_string()).collect(),
            ))?,
        None => util::cli_select("🔎 Select project template", templates.as_slice())?,
    };

    let template_path = template
        .path()
        .to_str()
        .ok_or(anyhow!("template path must be a utf-8 string!"))?
        .to_string();

    // generate new project
    let generate_args = CargoGenerateArgs {
        name: Some(args.name.to_string()),
        destination: Some(project_root.clone()),
        template_path: TemplatePath {
            path: Some(template_path),
            ..TemplatePath::default()
        },
        init: true,
        ..CargoGenerateArgs::default()
    };
    loading!("Generate new project", cargo_generate::generate(generate_args))?;

    // create templates dir if set
    if let Some(templates_dir) = template.extra().get(PROJECT_TEMPLATE_EXTRA_TEMPLATES_FIELD_NAME) {
        util::create_dir(&project_root.join(templates_dir)).await?;
    }

    // init project config file (remove if exists already somehow)
    let project_config_file = project_root.join(project::CONFIG_FILE_NAME);
    if util::file_exists(&project_config_file).await? {
        fs::remove_file(&project_config_file).await?;
    }
    fs::write(
        &project_config_file,
        toml::to_string(&project::ProjectConfig::default())?,
    )
    .await?;

    // init wasm templates if set
    if let Some(wasm_templates) = template
        .extra()
        .get(PROJECT_TEMPLATE_EXTRA_INIT_WASM_TEMPLATES_FIELD_NAME)
    {
        let wasm_template_names = wasm_templates.split(',').map(|s| s.trim());
        for wasm_template_name in wasm_template_names {
            md_println!("\n⚙️ Generating WASM project: **{}**", wasm_template_name);
            add::handle(
                config.clone(),
                wasm_template_repo.local_folder().clone(),
                AddArgs {
                    name: wasm_template_name.to_string(),
                    template: Some(wasm_template_name.to_string()),
                    output: project_root.clone(),
                    verbose: args.verbose,
                },
            )
            .await?;
        }
    }

    if !args.skip_init {
        // git init - cargo generate should do it already, but just in case
        let mut new_repo = GitRepository::new(project_root);
        new_repo.init()?;
    }

    Ok(())
}

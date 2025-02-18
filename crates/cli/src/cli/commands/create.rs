// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use std::path::PathBuf;

use crate::cli::commands::new;
use crate::cli::commands::new::NewArgs;
use crate::{
    cli::{config::Config, util},
    git::repository::GitRepository,
    loading, md_println, project,
    templates::Collector,
};
use anyhow::anyhow;
use cargo_generate::{GenerateArgs, TemplatePath};
use clap::Parser;
use thiserror::Error;
use tokio::fs;

const PROJECT_TEMPLATE_EXTRA_TEMPLATES_FIELD_NAME: &str = "templates_dir";
const PROJECT_TEMPLATE_EXTRA_INIT_WASM_TEMPLATES_FIELD_NAME: &str = "wasm_templates";

#[derive(Clone, Parser, Debug)]
pub struct CreateArgs {
    /// Name of the project
    #[arg(value_parser = crate::cli::arguments::project_name_parser)]
    pub name: String,

    /// (Optional) Selected project template (ID).
    /// It will be prompted if not set.
    #[arg(short = 't', long)]
    pub template: Option<String>,

    /// Target folder where the new project will be generated
    #[arg(long, value_name = "PATH", default_value = crate::cli::arguments::default_target_dir().into_os_string()
    )]
    pub target: PathBuf,
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
    args: &CreateArgs,
) -> anyhow::Result<()> {
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
            .filter(|template| template.id().to_lowercase() == template_id.to_lowercase())
            .last()
            .ok_or(CreateHandlerError::TemplateNotFound(
                template_id.to_string(),
                templates
                    .iter()
                    .map(|template| template.id().to_string())
                    .collect(),
            ))?,
        None => &util::cli_select("🔎 Select project template", templates.as_slice())?,
    };

    let template_path = template
        .path()
        .to_str()
        .ok_or(anyhow!("Invalid template path!"))?
        .to_string();

    // generate new project
    let generate_args = GenerateArgs {
        name: Some(args.name.to_string()),
        destination: Some(args.target.clone()),
        template_path: TemplatePath {
            path: Some(template_path),
            ..TemplatePath::default()
        },
        ..GenerateArgs::default()
    };
    loading!(
        "Generate new project",
        cargo_generate::generate(generate_args)
    )?;

    let final_path = args.target.join(args.name.as_str());

    // create templates dir if set
    if let Some(templates_dir) = template
        .extra()
        .get(PROJECT_TEMPLATE_EXTRA_TEMPLATES_FIELD_NAME)
    {
        util::create_dir(&final_path.join(templates_dir)).await?;
    }

    // init project config file (remove if exists already somehow)
    let project_config_file = final_path.join(project::CONFIG_FILE_NAME);
    if util::file_exists(&project_config_file).await? {
        fs::remove_file(&project_config_file).await?;
    }
    fs::write(
        &project_config_file,
        toml::to_string(&project::Config::default())?,
    )
    .await?;

    // init wasm templates if set
    if let Some(wasm_templates) = template
        .extra()
        .get(PROJECT_TEMPLATE_EXTRA_INIT_WASM_TEMPLATES_FIELD_NAME)
    {
        let wasm_templates = wasm_templates.replace(" ", "");
        let wasm_template_names = wasm_templates.split(',').collect::<Vec<_>>();
        for wasm_template_name in wasm_template_names {
            let wasm_template_repo = GitRepository::new(wasm_template_repo.local_folder().clone());
            md_println!("\n⚙️ Generating WASM project: **{}**", wasm_template_name);
            new::handle(
                config.clone(),
                wasm_template_repo,
                &NewArgs {
                    name: wasm_template_name.to_string(),
                    template: Some(wasm_template_name.to_string()),
                    target: final_path.clone(),
                },
            )
            .await?;
        }
    }

    // git init
    let mut new_repo = GitRepository::new(final_path);
    new_repo.init()?;

    Ok(())
}

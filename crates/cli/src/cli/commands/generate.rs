// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use std::path::PathBuf;

use anyhow::anyhow;
use cargo_generate::{GenerateArgs as CargoGenerateArgs, TemplatePath};
use cargo_toml::{Manifest, Resolver, Workspace};
use clap::Parser;
use tokio::fs;

use crate::{
    cli::{commands::create::CreateHandlerError, config::Config, util},
    git::repository::GitRepository,
    loading,
    templates::Collector,
};

const DEFAULT_TEMPLATES_DIR: &str = "templates";

#[derive(Clone, Parser, Debug)]
pub struct GenerateArgs {
    /// Name of the new wasm template crate
    #[arg(value_parser = crate::cli::command::project_name_parser)]
    pub name: String,

    /// (Optional) The wasm template short name (e.g. "meme_coin", "fungible" etc.).
    /// You wil be prompted to select a template if not set.
    #[arg(short = 't', long)]
    pub template: Option<String>,

    /// Directory where the new generated project will be output.
    #[arg(long, short = 'o', value_name = "PATH", default_value = crate::cli::command::default_target_dir().into_os_string())]
    pub output: PathBuf,

    /// Enables more verbose output.
    #[arg(long, short, action)]
    pub verbose: bool,
}

/// Handle `new` command.
/// It creates a new Tari WASM template development project.
pub async fn handle(
    config: Config,
    wasm_template_repo: GitRepository,
    args: &GenerateArgs,
) -> anyhow::Result<()> {
    // selecting wasm template
    let templates = loading!(
        "Collecting available **WASM** templates",
        Collector::new(
            wasm_template_repo
                .local_folder()
                .join(config.wasm_template_repository.folder)
        )
        .collect()
        .await
    )?;

    let template = match &args.template {
        Some(template_id) => templates
            .iter()
            .filter(|template| template.name().eq_ignore_ascii_case(template_id))
            .next_back()
            .ok_or_else(|| {
                CreateHandlerError::TemplateNotFound(
                    template_id.to_string(),
                    templates
                        .iter()
                        .map(|template| template.id().to_string())
                        .collect(),
                )
            })?,
        None => util::cli_select("🔎 Select WASM template", templates.as_slice())?,
    };

    let template_path = template
        .path()
        .to_str()
        .ok_or(anyhow!("Invalid template path!"))?
        .to_string();

    let cargo_toml_file = args.output.join("Cargo.toml");
    let is_cargo_project = util::file_exists(&cargo_toml_file).await?;

    // use '/templates' directory if exists
    let has_templates_sub_dir = util::dir_exists(&args.output.join(DEFAULT_TEMPLATES_DIR)).await?;
    let output = if has_templates_sub_dir {
        args.output.join(DEFAULT_TEMPLATES_DIR)
    } else {
        args.output.clone()
    };

    // generate new project
    let generate_args = CargoGenerateArgs {
        name: Some(args.name.to_string()),
        destination: Some(output.clone()),
        define: vec![format!("in_cargo_workspace={}", is_cargo_project)],
        template_path: TemplatePath {
            path: Some(template_path),
            ..TemplatePath::default()
        },
        verbose: args.verbose,
        ..CargoGenerateArgs::default()
    };
    loading!(
        "Generate new project",
        cargo_generate::generate(generate_args)
    )?;

    // check if target is a cargo project and update Cargo.toml if exists
    if is_cargo_project {
        let project_name = if has_templates_sub_dir {
            format!("{}/{}", DEFAULT_TEMPLATES_DIR, args.name)
        } else {
            args.name.to_string()
        };
        loading!(
            "Update **Cargo.toml**",
            update_cargo_toml(&cargo_toml_file, project_name).await
        )?;
    } else {
        // git init as new project is a separate one
        if let Err(error) = GitRepository::new(output.join(args.name.clone())).init() {
            println!("⚠️ Git repository already initialized: {error:?}");
        }
    }

    Ok(())
}

/// Updates Cargo.toml to make sure we have the new project in workspace members.
async fn update_cargo_toml(cargo_toml_file: &PathBuf, project_name: String) -> anyhow::Result<()> {
    let mut cargo_toml = Manifest::from_path(cargo_toml_file)?;
    cargo_toml.workspace = match cargo_toml.workspace {
        Some(mut workspace) => {
            if workspace.members.contains(&project_name) {
                return Err(anyhow!(
                    "New project generated, but Cargo.toml already contains a workspace member with the same name: {}",
                    project_name
                ));
            } else {
                workspace.members.push(project_name);
            }
            Some(workspace)
        }
        None => Some(Workspace {
            members: vec![project_name],
            default_members: vec![],
            package: None,
            exclude: vec![],
            metadata: None,
            resolver: Some(Resolver::V2),
            dependencies: Default::default(),
            lints: None,
        }),
    };
    fs::write(&cargo_toml_file, toml::to_string(&cargo_toml)?).await?;
    Ok(())
}

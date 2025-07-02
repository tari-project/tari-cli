// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use std::path::PathBuf;

use anyhow::anyhow;
use cargo_generate::{GenerateArgs as CargoGenerateArgs, TemplatePath};
use cargo_toml::{Manifest, Resolver, Workspace};
use clap::Parser;
use tokio::fs;

use crate::git::find_git_root;
use crate::{
    cli::{commands::create::CreateHandlerError, config::Config, util},
    git::repository::GitRepository,
    loading, md_println,
    templates::Collector,
};

const DEFAULT_TEMPLATES_DIR: &str = "templates";

#[derive(Clone, Parser, Debug)]
pub struct AddArgs {
    /// Name of the new wasm template crate
    #[arg(value_parser = crate::cli::command::project_name_parser)]
    pub name: String,

    /// (Optional) The wasm template short name (e.g. "meme_coin", "fungible" etc.).
    /// You wil be prompted to select a template if not set.
    #[arg(short = 't', long)]
    pub template: Option<String>,

    /// Directory where the new generated project will be output.
    #[arg(long, short = 'o', value_name = "PATH", default_value = crate::cli::command::default_output_dir().into_os_string())]
    pub output: PathBuf,

    /// Enables more verbose output.
    #[arg(long, short = 'v', action)]
    pub verbose: bool,
}

/// Handle `add` command.
/// Creates a new Tari WASM template project.
pub async fn handle(
    config: Config,
    local_template_repo_dir: PathBuf,
    args: AddArgs,
) -> anyhow::Result<()> {
    // selecting wasm template
    let templates = loading!(
        "Collecting available **WASM** templates",
        Collector::new(local_template_repo_dir.join(config.wasm_template_repository.folder))
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

    let git_root = find_git_root(&args.output);
    let workspace_toml_file = git_root.as_ref().map(|r| r.join("Cargo.toml"));
    let output = if let Some(git_root) = git_root {
        if args.verbose {
            md_println!(
                "ℹ️ Output directory `{}` is a git repository at `{}`.",
                args.output.display(),
                git_root.display()
            );
        }
        git_root
    } else {
        if args.verbose {
            md_println!(
                "ℹ️ Output directory `{}` is not a git repository.",
                args.output.display()
            );
        }
        args.output
    };

    // use '/templates' directory if exists
    let has_templates_sub_dir = util::dir_exists(&output.join(DEFAULT_TEMPLATES_DIR)).await?;
    let output = if has_templates_sub_dir {
        output.join(DEFAULT_TEMPLATES_DIR)
    } else {
        output
    };

    if args.verbose {
        md_println!(
            "ℹ️ Output directory for the new project is `{}`",
            output.display()
        );
    }

    // generate new project
    let generate_args = CargoGenerateArgs {
        name: Some(args.name.clone()),
        destination: Some(output.clone()),
        define: vec![format!(
            "in_cargo_workspace={}",
            workspace_toml_file.is_some()
        )],
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
    if let Some(workspace_toml_file) = workspace_toml_file {
        let project_rel_path = if has_templates_sub_dir {
            format!("{}/{}", DEFAULT_TEMPLATES_DIR, args.name)
        } else {
            args.name.to_string()
        };
        loading!(
            "Adding members to workspace",
            add_members_to_workspace(&workspace_toml_file, project_rel_path).await
        )?;
    } else {
        // git init as new project is a separate one
        if let Err(error) = GitRepository::new(output).init() {
            println!("⚠️ Git repository already initialized: {error}");
        }
    }

    Ok(())
}

/// Updates Cargo.toml to make sure we have the new project in workspace members.
async fn add_members_to_workspace(
    cargo_toml_file: &PathBuf,
    project_path: String,
) -> anyhow::Result<()> {
    let mut cargo_toml = Manifest::from_path(cargo_toml_file)?;
    cargo_toml.workspace = match cargo_toml.workspace {
        Some(mut workspace) => {
            if workspace.members.contains(&project_path) {
                md_println!(
                    "⚠️ Project `{}` is already a member of the workspace, skipping.",
                    project_path
                );
            } else {
                workspace.members.push(project_path);
            }
            Some(workspace)
        }
        None => {
            md_println!(
                "⚠️ Cargo toml is not a workspace. Creating a new workspace in `{}`",
                cargo_toml_file.display()
            );
            Some(Workspace {
                members: vec![project_path],
                default_members: vec![],
                package: None,
                exclude: vec![],
                metadata: None,
                resolver: Some(Resolver::V2),
                dependencies: Default::default(),
                lints: None,
            })
        }
    };
    fs::write(&cargo_toml_file, toml::to_string(&cargo_toml)?).await?;
    Ok(())
}

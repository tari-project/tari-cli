// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use crate::cli::commands::add::AddArgs;
use crate::cli::commands::create::CreateArgs;
use crate::cli::commands::deploy;
use crate::cli::commands::deploy::DeployArgs;
use crate::{
    cli::{
        commands::{add, create},
        config::{Config, TemplateRepository},
        util,
    },
    git::repository::GitRepository,
    loading,
};
use anyhow::anyhow;
use clap::{
    Parser, Subcommand,
    builder::{Styles, styling::AnsiColor},
};
use convert_case::{Case, Casing};
use std::{env, path::PathBuf};

const DEFAULT_DATA_FOLDER_NAME: &str = "tari_cli";
const TEMPLATE_REPOS_FOLDER_NAME: &str = "template_repositories";
const DEFAULT_CONFIG_FILE_NAME: &str = "tari.config.toml";

pub fn cli_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::BrightMagenta.on_default())
        .usage(AnsiColor::BrightMagenta.on_default())
        .literal(AnsiColor::BrightGreen.on_default())
        .placeholder(AnsiColor::BrightMagenta.on_default())
        .error(AnsiColor::BrightRed.on_default())
        .invalid(AnsiColor::BrightRed.on_default())
        .valid(AnsiColor::BrightGreen.on_default())
}

pub fn default_base_dir() -> PathBuf {
    dirs_next::data_dir()
        .unwrap_or_else(|| env::current_dir().unwrap())
        .join(DEFAULT_DATA_FOLDER_NAME)
}

pub fn default_output_dir() -> PathBuf {
    env::current_dir().unwrap()
}

pub fn default_config_file() -> PathBuf {
    dirs_next::config_dir()
        .unwrap_or_else(|| env::current_dir().unwrap())
        .join(DEFAULT_DATA_FOLDER_NAME)
        .join(DEFAULT_CONFIG_FILE_NAME)
}

pub fn config_override_parser(config_override: &str) -> Result<ConfigOverride, String> {
    if config_override.is_empty() {
        return Err(String::from("Override cannot be empty!"));
    }

    let split: Vec<&str> = config_override.split("=").collect();
    if split.len() != 2 {
        return Err(String::from("Invalid override!"));
    }
    let (key, value) = (split.first().unwrap(), split.get(1).unwrap());

    if !Config::is_override_key_valid(key) {
        return Err(format!("Override key invalid: {key}"));
    }

    Ok(ConfigOverride {
        key: key.to_string(),
        value: value.to_string(),
    })
}

pub fn project_name_parser(project_name: &str) -> Result<String, String> {
    Ok(project_name.to_case(Case::Snake))
}

#[derive(Clone, Debug)]
pub struct ConfigOverride {
    pub key: String,
    pub value: String,
}

#[derive(Clone, Parser, Debug)]
pub struct CommonArguments {
    /// Base directory, where all the CLI data will be saved
    #[arg(short = 'b', long, value_name = "PATH", default_value = default_base_dir().into_os_string()
    )]
    base_dir: PathBuf,

    /// Config file location
    #[arg(short = 'c', long, value_name = "PATH", default_value = default_config_file().into_os_string()
    )]
    config_file_path: PathBuf,

    /// Config file overrides
    #[arg(short = 'e', long, value_name = "KEY=VALUE", value_parser = config_override_parser
    )]
    config_overrides: Vec<ConfigOverride>,
}

#[derive(Clone, Parser)]
#[command(styles = cli_styles())]
#[command(
    version,
    about = "🚀 Tari CLI 🚀",
    long_about = "🚀 Tari Ootle CLI 🚀\nDevelop and deploy Tari templates."
)]
pub struct Cli {
    #[clap(flatten)]
    args: CommonArguments,

    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Subcommand)]
pub enum Command {
    /// Creates a new workspace for your Tari template project.
    #[clap(alias = "new")]
    Create {
        #[clap(flatten)]
        args: CreateArgs,
    },
    /// Generates and adds a new Tari wasm template crate.
    /// NOTE this command does not add the new template to an existing workspace, you may also use this command independent of a workspace.
    /// Use `create`/`new` if you want to start a new Tari template project with a workspace.
    #[clap(alias = "generate", alias = "gen")]
    Add {
        #[clap(flatten)]
        args: AddArgs,
    },
    /// Deploying Tari template to a network
    #[clap(alias = "publish")]
    Deploy {
        #[clap(flatten)]
        args: DeployArgs,
    },
}

impl Cli {
    async fn init_base_dir_and_config(&self) -> anyhow::Result<Config> {
        // make sure we have all the directories set up
        util::create_dir(&self.args.base_dir).await?;

        // create config file dir if not exists
        util::create_dir(
            &self
                .args
                .config_file_path
                .parent()
                .ok_or(anyhow!("Can't find folder of configuration file!"))?
                .to_path_buf(),
        )
        .await?;

        // loading/creating config
        let path = &self.args.config_file_path;
        let mut config = if !util::file_exists(path).await? {
            println!("Existing config not found. Creating a new config at {}", path.display());
            let cfg = Config::default();
            cfg.write_to_file(path).await?;
            cfg
        } else {
            match Config::open(path).await {
                Ok(cfg) => cfg,
                Err(error) => {
                    println!("Failed to open config file: {error:?}, creating default...");
                    let cfg = Config::default();
                    cfg.write_to_file(path).await?;
                    cfg
                },
            }
        };

        // apply config overrides
        for config_override in &self.args.config_overrides {
            config.override_data(config_override.key.as_str(), config_override.value.as_str())?;
        }

        Ok(config)
    }

    async fn refresh_template_repository(&self, template_repo: &TemplateRepository) -> anyhow::Result<GitRepository> {
        util::create_dir(&self.args.base_dir.join(TEMPLATE_REPOS_FOLDER_NAME)).await?;
        let repo_url_splitted: Vec<&str> = template_repo.url.split("/").collect();
        let repo_name = repo_url_splitted
            .last()
            .ok_or(anyhow!("Failed to get repository name from URL!"))?;
        let repo_user = repo_url_splitted
            .get(repo_url_splitted.len() - 2)
            .ok_or(anyhow!("Failed to get repository owner from URL!"))?;
        let repo_folder_path = self
            .args
            .base_dir
            .join(TEMPLATE_REPOS_FOLDER_NAME)
            .join(repo_user)
            .join(repo_name);
        let mut repo = GitRepository::new(repo_folder_path.clone());

        if util::dir_exists(&repo_folder_path).await? {
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

        Ok(repo)
    }

    pub async fn handle_command(self) -> anyhow::Result<()> {
        // init config and dirs
        let config = loading!(
            "Init configuration and directories",
            self.init_base_dir_and_config().await
        )?;

        // refresh templates from provided repositories
        let project_template_repo = loading!(
            "Refresh project templates repository",
            self.refresh_template_repository(&config.project_template_repository)
                .await
        )?;
        let wasm_template_repo = loading!(
            "Refresh wasm templates repository",
            self.refresh_template_repository(&config.wasm_template_repository).await
        )?;

        match self.command {
            Command::Create { args } => create::handle(config, project_template_repo, wasm_template_repo, args).await,
            Command::Add { args } => add::handle(config, wasm_template_repo.local_folder().clone(), args).await,
            Command::Deploy { args } => deploy::handle(config, args).await,
        }
    }
}

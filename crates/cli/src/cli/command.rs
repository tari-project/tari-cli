// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use crate::cli::commands::build::BuildArgs;
use crate::cli::commands::config::ConfigCommand;
use crate::cli::commands::create::CreateArgs;
use crate::cli::commands::init::InitArgs;
use crate::cli::commands::metadata::MetadataCommand;
use crate::cli::commands::publish;
use crate::cli::commands::publish::PublishArgs;
use crate::cli::commands::template::TemplateCommand;
use crate::{
    cli::{
        commands::{build, config as config_cmd, create, init, metadata, template, wizard},
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
use ootle_network::Network;
use std::{convert::Infallible, env, path::PathBuf};
use tari_ootle_publish_lib::PublisherError;
use tari_utilities::Hidden;

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

    let Some((key, value)) = config_override.split_once('=') else {
        return Err(String::from("Invalid override! Expected KEY=VALUE."));
    };

    if !Config::is_override_key_valid(key) {
        return Err(format!("Override key invalid: {key}"));
    }

    Ok(ConfigOverride {
        key: key.to_string(),
        value: value.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn override_parser_accepts_nested_network_keys() {
        let ov = config_override_parser("networks.esmeralda.wallet-daemon-url=http://localhost:5100/")
            .expect("nested network key should parse");
        assert_eq!(ov.key, "networks.esmeralda.wallet-daemon-url");
        assert_eq!(ov.value, "http://localhost:5100/");
    }

    #[test]
    fn override_parser_rejects_unknown_network() {
        assert!(config_override_parser("networks.bogus.wallet-daemon-url=http://x").is_err());
    }

    #[test]
    fn override_parser_rejects_unknown_field() {
        assert!(config_override_parser("networks.esmeralda.bogus=http://x").is_err());
    }

    #[test]
    fn override_parser_keeps_value_with_equals() {
        let ov = config_override_parser("default_account=acc=ount").expect("split_once should keep tail intact");
        assert_eq!(ov.value, "acc=ount");
    }

    use anyhow::Context;
    use tari_ootle_publish_lib::walletd_client::error::WalletDaemonClientError;

    fn unauthorized_error() -> anyhow::Error {
        // Mirror the real flow: a wallet daemon 401 wrapped in PublisherError, then
        // given anyhow context by a handler.
        let inner = PublisherError::WalletDaemonClient(WalletDaemonClientError::Unauthorized {
            message: "token rejected".to_string(),
        });
        Err::<(), _>(inner).context("Failed to connect to the wallet").unwrap_err()
    }

    #[test]
    fn appends_setup_help_on_unauthorized_without_key() {
        let augmented = explain_wallet_daemon_auth_error(unauthorized_error(), false);
        let msg = format!("{augmented:#}");
        assert!(msg.contains("no API key was provided"), "got: {msg}");
        assert!(msg.contains("TARI_WALLET_DAEMON_API_KEY"), "got: {msg}");
    }

    #[test]
    fn appends_rejected_help_on_unauthorized_with_key() {
        let augmented = explain_wallet_daemon_auth_error(unauthorized_error(), true);
        let msg = format!("{augmented:#}");
        assert!(msg.contains("rejected the provided API key"), "got: {msg}");
    }

    #[test]
    fn leaves_non_auth_errors_untouched() {
        let original = anyhow!("some unrelated failure");
        let augmented = explain_wallet_daemon_auth_error(original, false);
        let msg = format!("{augmented:#}");
        assert_eq!(msg, "some unrelated failure");
        assert!(!msg.contains("API key"));
    }
}

pub fn project_name_parser(project_name: &str) -> Result<String, String> {
    Ok(project_name.to_case(Case::Snake))
}

fn parse_network(s: &str) -> Result<Network, String> {
    s.parse().map_err(|e: ootle_network::NetworkParseError| e.to_string())
}

/// Wraps a raw API key (from `--api-key` or `TARI_WALLET_DAEMON_API_KEY`) in
/// [`Hidden`] so it is zeroized on drop and kept out of `Debug` output.
fn parse_api_key(value: &str) -> Result<Hidden<String>, Infallible> {
    Ok(Hidden::hide(value.to_string()))
}

/// Guidance shown when the wallet daemon rejects a request because of
/// authentication. The opening line is tailored to whether the user supplied a
/// key at all, so a missing key and a bad key get distinct, actionable advice.
fn wallet_daemon_auth_help(had_api_key: bool) -> String {
    let intro = if had_api_key {
        "The wallet daemon rejected the provided API key. It may be invalid, expired, \
         or missing the required permissions."
    } else {
        "The wallet daemon requires authentication, but no API key was provided."
    };
    format!(
        "🔑 {intro}\n\n\
         Provide a key with the `--api-key` flag or the `TARI_WALLET_DAEMON_API_KEY` \
         environment variable, for example:\n\n    \
         export TARI_WALLET_DAEMON_API_KEY=\"<your-api-key>\"\n    \
         tari publish -a <account>\n\n\
         The key must be minted by the wallet daemon with at least the `templates:read`, \
         `templates:create` and `accounts:read` permissions.\n\
         See: https://tari-project.github.io/tari-cli/"
    )
}

/// If `err` was caused by a wallet daemon authentication failure, append setup
/// guidance so the user knows how to provide a valid API key. Any other error
/// is returned unchanged.
fn explain_wallet_daemon_auth_error(err: anyhow::Error, had_api_key: bool) -> anyhow::Error {
    let unauthorized = err
        .chain()
        .any(|cause| cause.downcast_ref::<PublisherError>().is_some_and(PublisherError::is_unauthorized));

    if unauthorized {
        anyhow!("{err:#}\n\n{}", wallet_daemon_auth_help(had_api_key))
    } else {
        err
    }
}

#[derive(Clone, Debug)]
pub struct ConfigOverride {
    pub key: String,
    pub value: String,
}

#[derive(Clone, Parser, Debug)]
pub struct CommonArguments {
    /// Base directory, where all the CLI data will be saved
    #[arg(short = 'b', long, value_name = "PATH", default_value = default_base_dir().into_os_string())]
    base_dir: PathBuf,

    /// Config file location
    #[arg(short = 'c', long, value_name = "PATH", default_value = default_config_file().into_os_string())]
    config_file_path: PathBuf,

    /// Config file overrides
    #[arg(short = 'e', long, value_name = "KEY=VALUE", value_parser = config_override_parser)]
    config_overrides: Vec<ConfigOverride>,

    /// Network to use. Overrides the default set in project and global config.
    /// (e.g. `esmeralda`, `igor`, `localnet`, `mainnet`)
    #[arg(short = 'n', long, value_name = "NETWORK", value_parser = parse_network, global = true)]
    network: Option<Network>,

    /// API key used to authenticate with the wallet daemon.
    /// Sent as a bearer token on every JSON-RPC request. The key must be minted
    /// with at least `templates:read`, `templates:create` and `accounts:read`
    /// permissions. Can also be set via the `TARI_WALLET_DAEMON_API_KEY`
    /// environment variable.
    #[arg(
        long,
        value_name = "API_KEY",
        env = "TARI_WALLET_DAEMON_API_KEY",
        hide_env_values = true,
        value_parser = parse_api_key,
        global = true
    )]
    api_key: Option<Hidden<String>>,
}

#[derive(Clone, Parser)]
#[command(styles = cli_styles())]
#[command(
    version,
    about = "🚀 Tari CLI 🚀",
    long_about = "🚀 Tari Ootle CLI 🚀\nDevelop and publish Tari templates."
)]
pub struct Cli {
    #[clap(flatten)]
    args: CommonArguments,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Clone, Subcommand)]
pub enum Command {
    /// Initialise the project config and template metadata in a Tari template crate.
    Init {
        #[clap(flatten)]
        args: InitArgs,
    },
    /// Create a new Tari template crate from a starter template.
    #[clap(alias = "new")]
    Create {
        #[clap(flatten)]
        args: CreateArgs,
    },
    /// Build the template WASM binary.
    Build {
        #[clap(flatten)]
        args: BuildArgs,
    },
    /// Publish a Tari template to a network.
    #[clap(alias = "deploy")]
    Publish {
        #[clap(flatten)]
        args: PublishArgs,
    },
    /// Template metadata tooling (init, inspect, publish with metadata).
    Template {
        #[command(subcommand)]
        command: TemplateCommand,
    },
    /// Template metadata operations (inspect, publish to server).
    Metadata {
        #[command(subcommand)]
        command: MetadataCommand,
    },
    /// Manage project configuration (tari.config.toml).
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
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

    pub async fn handle_command(mut self) -> anyhow::Result<()> {
        let Some(command) = self.command.take() else {
            return wizard::handle().await;
        };

        // Config command operates on project config, not CLI config
        if let Command::Config { command } = command {
            return config_cmd::handle(command).await;
        }

        if let Command::Init { args } = command {
            return init::handle(args).await;
        }

        if let Command::Build { args } = command {
            return build::handle(args).await;
        }

        if let Command::Metadata {
            command: MetadataCommand::Inspect { args },
        } = command
        {
            return template::inspect_metadata::handle(args).await;
        }

        // init config and dirs
        let config = loading!(
            "Init configuration and directories",
            self.init_base_dir_and_config().await
        )?;

        // Commands that don't need template repository refresh
        match &command {
            Command::Template { .. } | Command::Publish { .. } | Command::Metadata { .. } => {
                let network_override = self.args.network;
                // Move the key out rather than clone, so no extra plaintext copy lingers.
                let api_key = self.args.api_key.take();
                let had_api_key = api_key.is_some();
                let result = match command {
                    Command::Template { command } => match command {
                        TemplateCommand::Init { args } => template::init_metadata::handle(args).await,
                        TemplateCommand::Inspect { args } => template::inspect_metadata::handle(args).await,
                        TemplateCommand::Publish { args } => {
                            template::publish::handle(config, network_override, api_key, args).await
                        },
                    },
                    Command::Publish { args } => publish::handle(config, network_override, api_key, args).await,
                    Command::Metadata { command } => match command {
                        MetadataCommand::Publish { args } => {
                            metadata::publish::handle(config, network_override, api_key, args).await
                        },
                        MetadataCommand::Inspect { .. } => unreachable!(),
                    },
                    _ => unreachable!(),
                };
                // Surface API key setup help when the daemon rejects us for auth.
                return result.map_err(|e| explain_wallet_daemon_auth_error(e, had_api_key));
            },
            _ => {},
        }

        // Refresh template repository (only needed for `create`)
        let template_repo = loading!(
            "Refresh templates repository",
            self.refresh_template_repository(&config.template_repository).await
        )?;

        match command {
            Command::Create { args } => create::handle(config, template_repo.local_folder().clone(), args).await,
            _ => unreachable!(),
        }
    }
}

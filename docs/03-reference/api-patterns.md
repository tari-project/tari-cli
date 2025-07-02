---
title: API Patterns Reference
description: Implementation patterns and code examples extracted from the Tari CLI codebase
last_updated: 2025-06-26
version: Latest (main branch)
verified_against: Real implementation patterns from crates/cli/src/**/*.rs
audience: developers
---

# API Patterns Reference

> **Real implementation patterns** from the Tari CLI codebase for building robust smart contract development tools

## CLI Architecture Patterns

### Command Structure Pattern

<!-- SOURCE: Verified against crates/cli/src/cli/arguments.rs:121-138 -->
The Tari CLI uses clap's derive API with a hierarchical command structure:

```rust
use clap::{Parser, Subcommand};

#[derive(Clone, Parser)]
#[command(styles = cli_styles())]
pub struct Cli {
    #[clap(flatten)]
    args: CommonArguments,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, Subcommand)]
pub enum Commands {
    /// Creates a new Tari templates project
    Create {
        #[clap(flatten)]
        args: CreateArgs,
    },
    /// Creates a new Tari wasm template project
    New {
        #[clap(flatten)]
        args: NewArgs,
    },
    /// Deploying Tari template to a network
    Deploy {
        #[clap(flatten)]
        args: DeployArgs,
    },
}
```

**Pattern Benefits**:
- **Flattened arguments**: Shared options across commands
- **Subcommand isolation**: Each command has dedicated arguments
- **Type safety**: Compile-time argument validation
- **Auto-generated help**: Clap generates consistent help text

### Async Command Handler Pattern

<!-- SOURCE: Verified against crates/cli/src/cli/arguments.rs:219-245 -->
Each command follows a consistent async execution pattern:

```rust
impl Cli {
    pub async fn handle_command(&self) -> anyhow::Result<()> {
        // Initialize configuration and directories
        let config = loading!(
            "Init configuration and directories",
            self.init_base_dir_and_config().await
        )?;

        // Prepare resources (template repositories)
        let project_template_repo = loading!(
            "Refresh project templates repository",
            self.refresh_template_repository(&config.project_template_repository).await
        )?;
        
        let wasm_template_repo = loading!(
            "Refresh wasm templates repository", 
            self.refresh_template_repository(&config.wasm_template_repository).await
        )?;

        // Dispatch to specific command handler
        match &self.command {
            Commands::Create { args } => {
                create::handle(config, project_template_repo, wasm_template_repo, args).await
            }
            Commands::New { args } => new::handle(config, wasm_template_repo, args).await,
            Commands::Deploy { args } => deploy::handle(args).await,
        }
    }
}
```

**Pattern Elements**:
- **Resource initialization**: Setup required dependencies first
- **Loading feedback**: User feedback during long operations
- **Error propagation**: Consistent error handling with `anyhow::Result`
- **Command dispatch**: Clean separation of command logic

## UI/UX Patterns

### Loading Indicator Pattern

<!-- SOURCE: Verified against crates/cli/src/cli/macros.rs:5-42 -->
Consistent loading feedback across all operations:

```rust
#[macro_export]
macro_rules! loading {
    ( $text:literal, $call:expr ) => {{
        let mut skin = termimad::MadSkin::default();
        skin.bold.set_fg(termimad::crossterm::style::Color::Magenta);
        let mut loader = spinners::Spinner::new(
            spinners::Spinners::Dots, 
            skin.inline($text).to_string()
        );
        let result = match $call {
            Ok(res) => {
                loader.stop_with_symbol("✅");
                Ok(res)
            }
            Err(error) => {
                loader.stop_with_symbol("❌");
                Err(error)
            }
        };
        result
    }};
}
```

**Usage Examples**:
```rust
// Template compilation
let template_bin = loading!(
    format!("Building WASM template project **{}**", project_name),
    build_project(&project_dir, project_name.clone()).await
)?;

// Repository refresh
let project_template_repo = loading!(
    "Refresh project templates repository",
    self.refresh_template_repository(&config.project_template_repository).await
)?;
```

### Interactive Selection Pattern

<!-- SOURCE: Verified against crates/cli/src/cli/util.rs:25-33 -->
Fuzzy selection for user-friendly template choices:

```rust
use dialoguer::FuzzySelect;

pub fn cli_select<T: ToString + Clone>(prompt: &str, items: &[T]) -> anyhow::Result<T> {
    let selection = FuzzySelect::new()
        .with_prompt(prompt)
        .highlight_matches(true)
        .items(items)
        .interact()?;

    Ok(items[selection].clone())
}
```

**Usage Pattern**:
```rust
// Allow user to select from available templates
let template = match &args.template {
    Some(template_id) => {
        // Direct selection by ID
        templates.iter()
            .filter(|t| t.id().to_lowercase() == template_id.to_lowercase())
            .last()
            .ok_or(CreateHandlerError::TemplateNotFound(...))?
    }
    None => {
        // Interactive fuzzy selection
        &util::cli_select("🔎 Select project template", templates.as_slice())?
    }
};
```

### Markdown Output Pattern

<!-- SOURCE: Verified against crates/cli/src/cli/macros.rs:44-51 -->
Rich terminal output with markdown formatting:

```rust
#[macro_export]
macro_rules! md_println {
    ( $text:literal, $($args:tt)* ) => {{
        let mut skin = termimad::MadSkin::default();
        skin.bold.set_fg(termimad::crossterm::style::Color::Magenta);
        skin.print_inline(format!($text, $($args)*).as_str());
    }};
}
```

**Usage Examples**:
```rust
// Rich output with markdown formatting
md_println!("\n⚙️ Generating WASM project: **{}**", wasm_template_name);

// Success messages with formatting
println!("⭐ Your new template's address: {}", template_address);
```

## Configuration Patterns

### Hierarchical Configuration Pattern

<!-- SOURCE: Verified against crates/cli/src/cli/config.rs and project/config.rs -->
Multi-level configuration with overrides:

```rust
// Global CLI configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    pub project_template_repository: TemplateRepository,
    pub wasm_template_repository: TemplateRepository,
}

// Project-specific configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectConfig {
    network: NetworkConfig,
}
```

**Override Pattern**:
```rust
// Command-line overrides with validation
const VALID_OVERRIDE_KEYS: &[&str] = &[
    "project_template_repository.url",
    "project_template_repository.branch",
    "wasm_template_repository.url",
    // ... other valid keys
];

pub fn override_data(&mut self, key: &str, value: &str) -> anyhow::Result<&mut Self> {
    if !Self::is_override_key_valid(key) {
        return Err(anyhow!("Invalid key: {}", key));
    }

    match key {
        "project_template_repository.url" => {
            self.project_template_repository.url = value.to_string();
        }
        // ... handle other override keys
        _ => {}
    }

    Ok(self)
}
```

### Default Configuration Pattern

<!-- SOURCE: Verified against config implementations -->
Sensible defaults with easy customization:

```rust
impl Default for Config {
    fn default() -> Self {
        Self {
            project_template_repository: TemplateRepository {
                url: "https://github.com/tari-project/wasm-template".to_string(),
                branch: "main".to_string(),
                folder: "project_templates".to_string(),
            },
            wasm_template_repository: TemplateRepository {
                url: "https://github.com/tari-project/wasm-template".to_string(),
                branch: "main".to_string(),
                folder: "wasm_templates".to_string(),
            },
        }
    }
}
```

## File System Patterns

### Async File Operations Pattern

<!-- SOURCE: Verified against crates/cli/src/cli/util.rs:9-23 -->
Consistent async file system operations:

```rust
use tokio::fs;
use std::path::PathBuf;

pub async fn create_dir(dir: &PathBuf) -> io::Result<()> {
    fs::create_dir_all(dir).await
}

pub async fn file_exists(file: &PathBuf) -> io::Result<bool> {
    Ok(fs::try_exists(file).await? && path_metadata(file).await?.is_file())
}

pub async fn dir_exists(dir: &PathBuf) -> io::Result<bool> {
    Ok(fs::try_exists(dir).await? && path_metadata(dir).await?.is_dir())
}
```

**Safety Pattern**:
```rust
// Always check existence before operations
if util::file_exists(&project_config_file).await? {
    fs::remove_file(&project_config_file).await?;
}

// Create directories recursively
util::create_dir(&final_path.join(templates_dir)).await?;
```

### Template Discovery Pattern

<!-- SOURCE: Verified against crates/cli/src/templates/collector.rs:34-89 -->
Recursive template scanning with validation:

```rust
pub async fn collect(&self) -> CollectorResult<Vec<Template>> {
    let mut result = vec![];
    Self::collect_templates(&self.local_folder, &mut result).await?;
    Ok(result)
}

async fn collect_templates(dir: &PathBuf, result: &mut Vec<Template>) -> CollectorResult<()> {
    if dir.is_dir() {
        let mut entries_stream = fs::read_dir(dir).await?;
        while let Some(entry) = entries_stream.next_entry().await? {
            if entry.path().is_dir() {
                // Recursive directory scanning
                Box::pin(Self::collect_templates(&entry.path(), result)).await?;
            } else if let Some(file_name) = entry.file_name().to_str() {
                if file_name == TEMPLATE_DESCRIPTOR_FILE_NAME {
                    // Parse template descriptor
                    let toml_content = fs::read_to_string(&entry.path()).await?;
                    let template_file: TemplateFile = toml::from_str(toml_content.as_str())?;
                    
                    // Build template metadata
                    result.push(Template::new(
                        path,
                        template_id,
                        template_file.name,
                        template_file.description,
                        template_file.extra.unwrap_or_default(),
                    ));
                }
            }
        }
    }
    Ok(())
}
```

## Error Handling Patterns

### Typed Error Pattern

<!-- SOURCE: Verified against crates/cli/src/cli/commands/create.rs:40-44 -->
Domain-specific error types with helpful messages:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CreateHandlerError {
    #[error("Template not found by name: {0}. Possible values: {1:?}")]
    TemplateNotFound(String, Vec<String>),
}
```

**Usage**:
```rust
// Provide helpful error with available options
templates.iter()
    .filter(|template| template.id().to_lowercase() == template_id.to_lowercase())
    .last()
    .ok_or(CreateHandlerError::TemplateNotFound(
        template_id.to_string(),
        templates.iter().map(|template| template.id().to_string()).collect(),
    ))?
```

### Error Context Pattern

<!-- SOURCE: Verified across command implementations -->
Rich error context with `anyhow`:

```rust
use anyhow::{anyhow, Context};

// Add context to errors
fs::read_to_string(&config_file)
    .await
    .map_err(|error| {
        anyhow!("Failed to load project config file (at {config_file:?}): {error:?}")
    })?

// Chain context information
cargo_generate::generate(generate_args)
    .context("Failed to generate project from template")?
```

## Git Operations Patterns

### Repository Management Pattern

<!-- SOURCE: Verified against crates/cli/src/cli/arguments.rs:181-217 -->
Automated git repository handling:

```rust
async fn refresh_template_repository(
    &self,
    template_repo: &TemplateRepository,
) -> anyhow::Result<GitRepository> {
    // Ensure repositories directory exists
    util::create_dir(&self.args.base_dir.join(TEMPLATE_REPOS_FOLDER_NAME)).await?;
    
    // Parse repository information from URL
    let repo_url_splitted: Vec<&str> = template_repo.url.split("/").collect();
    let repo_name = repo_url_splitted.last()
        .ok_or(anyhow!("Failed to get repository name from URL!"))?;
    let repo_user = repo_url_splitted.get(repo_url_splitted.len() - 2)
        .ok_or(anyhow!("Failed to get repository owner from URL!"))?;
    
    let repo_folder_path = self.args.base_dir
        .join(TEMPLATE_REPOS_FOLDER_NAME)
        .join(repo_user)
        .join(repo_name);
    
    let mut repo = GitRepository::new(repo_folder_path.clone());

    match util::dir_exists(&repo_folder_path).await? {
        true => {
            // Update existing repository
            repo.load()?;
            let current_branch = repo.current_branch_name()?;
            if current_branch != template_repo.branch {
                repo.pull_changes(Some(template_repo.branch.clone()))?;
            } else {
                repo.pull_changes(None)?;
            }
        }
        false => {
            // Clone new repository
            repo.clone_and_checkout(template_repo.url.as_str(), template_repo.branch.as_str())?;
        }
    }

    Ok(repo)
}
```

## Smart Contract Patterns

### Template Structure Pattern

<!-- SOURCE: Verified against documentation examples in quick-start.md -->
Standard Tari smart contract template structure:

```rust
use tari_template_lib::prelude::*;

#[template]
mod my_contract {
    use super::*;

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct MyContract {
        // Contract state fields
        data: BTreeMap<String, String>,
        counter: u64,
    }

    impl MyContract {
        // Constructor - initializes the contract
        pub fn new() -> Component<Self> {
            Component::new(Self {
                data: BTreeMap::new(),
                counter: 0,
            })
        }

        // State-modifying method
        pub fn increment(&mut self) -> u64 {
            self.counter += 1;
            self.counter
        }

        // Read-only method
        pub fn get_counter(&self) -> u64 {
            self.counter
        }

        // Method with parameters
        pub fn store_data(&mut self, key: String, value: String) -> Option<String> {
            self.data.insert(key, value)
        }
    }
}
```

### Contract State Pattern

Best practices for contract state management:

```rust
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ContractState {
    // Use efficient collections
    tokens: BTreeMap<TokenId, TokenData>,
    
    // Avoid large data structures in state
    metadata_hash: Hash,  // Reference to off-chain data
    
    // Use appropriate numeric types
    next_id: u64,
    total_supply: Amount,
    
    // Consider access patterns
    owner_tokens: BTreeMap<PublicKey, BTreeSet<TokenId>>,
}
```

## Deployment Patterns

### WASM Build Pattern

<!-- SOURCE: Verified against crates/cli/src/cli/commands/deploy.rs:122-156 -->
Automated WASM compilation with error handling:

```rust
async fn build_project(dir: &Path, name: String) -> anyhow::Result<PathBuf> {
    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .arg("--target=wasm32-unknown-unknown")
        .arg("--release")
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let process = cmd.spawn()?;
    let output = process.wait_with_output().await?;

    if !output.status.success() {
        return Err(anyhow!(
            "Failed to build project: {dir:?}\nBuild Output:\n\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let output_bin = dir
        .join("target")
        .join("wasm32-unknown-unknown")
        .join("release")
        .join(format!("{}.wasm", name));
        
    if !util::file_exists(&output_bin).await? {
        return Err(anyhow!(
            "Binary is not present after build at {:?}",
            output_bin
        ));
    }

    Ok(output_bin)
}
```

### Deployment Validation Pattern

<!-- SOURCE: Verified against crates/tari_deploy/src/deployer.rs -->
Pre-deployment validation and cost estimation:

```rust
pub async fn deploy(
    &self,
    account: &ComponentAddressOrName,
    template: Template,
    max_fee: u64,
    wait_timeout: Option<Duration>,
) -> Result<TemplateAddress> {
    // 1. Prepare deployment request
    let publish_template_request = self
        .publish_template_request(account, &template, max_fee)
        .await?;
    
    // 2. Validate sufficient balance
    self.check_balance_to_deploy(account, &template).await?;
    
    // 3. Execute deployment
    self.publish_template(
        publish_template_request,
        wait_timeout.or(Some(Duration::from_secs(120))),
    )
    .await
}
```

## Testing Patterns

### Integration Test Structure

<!-- SOURCE: Verified against crates/cli/src/templates/collector.rs:154-200 -->
Comprehensive integration testing with temporary directories:

```rust
#[tokio::test]
async fn test_collect() {
    let temp_dir = TempDir::new("tari_cli_test_collect_templates").unwrap();
    let temp_dir_path = temp_dir.path().to_path_buf();
    
    let templates_to_generate = vec![
        TemplateToGenerate::new("template1", "description1", None),
        TemplateToGenerate::new("template2", "description2", None),
        TemplateToGenerate::new(
            "template3",
            "description3",
            Some(HashMap::from([(
                "templates_dir".to_string(),
                "templates".to_string(),
            )])),
        ),
    ];
    
    // Generate test templates
    for template in &templates_to_generate {
        generate_template(&temp_dir_path, template).await;
    }

    // Test collection
    let collector = Collector::new(temp_dir_path);
    let result = collector.collect().await;

    assert!(result.is_ok());
    let result = result.unwrap();
    assert_eq!(result.len(), templates_to_generate.len());

    // Verify all templates were found
    for template in &templates_to_generate {
        assert!(result.iter().any(|curr_template| {
            curr_template.name() == template.name
                && curr_template.description() == template.description
        }));
    }
}
```

## Performance Patterns

### Async Parallel Operations

Execute independent operations concurrently:

```rust
// Refresh template repositories in parallel
let (project_repo_future, wasm_repo_future) = tokio::join!(
    self.refresh_template_repository(&config.project_template_repository),
    self.refresh_template_repository(&config.wasm_template_repository)
);

let project_template_repo = project_repo_future?;
let wasm_template_repo = wasm_repo_future?;
```

### Lazy Resource Loading

Load resources only when needed:

```rust
// Load templates only when required for selection
let templates = match &args.template {
    Some(_) => {
        // Template specified, load for validation
        Collector::new(repo_path).collect().await?
    }
    None => {
        // Need interactive selection, load templates
        loading!(
            "Collecting available templates",
            Collector::new(repo_path).collect().await
        )?
    }
};
```

## Security Patterns

### Input Validation Pattern

Sanitize and validate all user inputs:

```rust
// Project name sanitization
pub fn project_name_parser(project_name: &str) -> Result<String, String> {
    Ok(project_name.to_case(Case::Snake))
}

// Configuration override validation
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
        return Err(format!("Override key invalid: {}", key));
    }

    Ok(ConfigOverride {
        key: key.to_string(),
        value: value.to_string(),
    })
}
```

### Safe File Operations Pattern

Prevent path traversal and validate file operations:

```rust
// Validate paths before operations
if !path.ancestors().any(|ancestor| ancestor == expected_root) {
    return Err(anyhow!("Invalid path: outside expected directory"));
}

// Safe file removal with existence check
if util::file_exists(&config_file).await? {
    fs::remove_file(&config_file).await?;
}
```

---

These patterns represent battle-tested approaches from the Tari CLI codebase. They emphasize **safety**, **user experience**, and **maintainability** while providing real-world examples of effective Rust CLI development.

**For implementation examples**, see the [CLI Commands Reference](cli-commands.md) and [Template Development Guide](../02-guides/template-development.md).

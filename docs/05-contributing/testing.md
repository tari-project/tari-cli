---
title: Testing Guide
description: Comprehensive guide to testing practices and infrastructure for Tari CLI
last_updated: 2025-06-26
version: Latest (main branch)
verified_against: .config/nextest.toml, CI test configuration, actual test patterns
audience: contributors
---

# Testing Guide

> **Comprehensive testing practices** for maintaining high-quality Tari CLI code

## Testing Philosophy

The Tari CLI follows a multi-layered testing approach:

1. **Unit Tests**: Test individual functions and modules in isolation
2. **Integration Tests**: Test component interactions and workflows
3. **End-to-End Tests**: Test complete user scenarios
4. **Property Tests**: Test behavior across input ranges
5. **Performance Tests**: Validate performance characteristics

## Test Infrastructure

### Test Framework Setup

<!-- SOURCE: Verified against .config/nextest.toml and CI configuration -->
**Primary Test Runner**: `cargo-nextest`
```bash
# Install nextest
cargo install cargo-nextest --locked

# Run tests with nextest
cargo nextest run --all-features

# Run with CI profile
cargo nextest run --all-features --release --profile ci
```

**Nextest Configuration** (`.config/nextest.toml`):
```toml
[profile.ci]
slow-timeout = { period = "60s", terminate-after = 4 }

[profile.ci.junit]
path = "junit.xml"
```

### Test Organization

**Directory Structure**:
```
crates/
├── cli/
│   ├── src/
│   │   ├── lib.rs
│   │   ├── templates/
│   │   │   ├── collector.rs
│   │   │   └── mod.rs
│   │   └── ...
│   └── tests/                 # Integration tests
└── tari_deploy/
    ├── src/
    │   ├── lib.rs
    │   ├── deployer.rs
    │   └── ...
    └── tests/                 # Integration tests
```

**Test Types by Location**:
- **Unit tests**: Inline with source code (`#[cfg(test)]` modules)
- **Integration tests**: In `tests/` directory
- **Doc tests**: In documentation comments
- **Example tests**: In `examples/` directory

## Unit Testing Patterns

### Basic Unit Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempdir::TempDir;
    use tokio::test;

    #[tokio::test]
    async fn test_function_name() {
        // Arrange
        let input = setup_test_data();
        
        // Act
        let result = function_under_test(input).await;
        
        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_value);
    }
}
```

### Template Collector Testing

<!-- SOURCE: Verified against crates/cli/src/templates/collector.rs:154-200 -->
**Real Example from Codebase**:
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

    // Assert all templates existence
    for template in &templates_to_generate {
        match &template.extra {
            Some(extra) => {
                assert!(result.iter().any(|curr_template| {
                    curr_template.name() == template.name
                        && curr_template.description() == template.description
                        && curr_template.extra().eq(extra)
                }));
            }
            None => {
                assert!(result.iter().any(|curr_template| {
                    curr_template.name() == template.name
                        && curr_template.description() == template.description
                }));
            }
        }
    }
}
```

### File System Testing Patterns

```rust
use tempdir::TempDir;
use tokio::fs;

#[tokio::test]
async fn test_file_operations() {
    // Use temporary directory for isolation
    let temp_dir = TempDir::new("test_file_ops").unwrap();
    let test_file = temp_dir.path().join("test.txt");
    
    // Test file creation
    fs::write(&test_file, "test content").await.unwrap();
    assert!(test_file.exists());
    
    // Test file reading
    let content = fs::read_to_string(&test_file).await.unwrap();
    assert_eq!(content, "test content");
    
    // Temporary directory is automatically cleaned up
}
```

## Integration Testing

### CLI Command Testing

```rust
use std::process::Command;
use tempdir::TempDir;

#[tokio::test]
async fn test_create_command_integration() {
    let temp_dir = TempDir::new("test_create_command").unwrap();
    
    // Test CLI command execution
    let output = Command::new("cargo")
        .args(&["run", "--bin", "tari-cli", "--", "create", "test-project"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
    
    // Verify project structure was created
    let project_dir = temp_dir.path().join("test-project");
    assert!(project_dir.exists());
    assert!(project_dir.join("Cargo.toml").exists());
    assert!(project_dir.join("tari.config.toml").exists());
}
```

### Configuration Testing

```rust
#[tokio::test]
async fn test_config_loading() {
    let temp_dir = TempDir::new("test_config").unwrap();
    let config_file = temp_dir.path().join("tari.config.toml");
    
    // Create test configuration
    let config_content = r#"
[project-template-repository]
url = "https://github.com/test-org/templates"
branch = "main"
folder = "templates"
"#;
    fs::write(&config_file, config_content).await.unwrap();
    
    // Test configuration loading
    let config = Config::open(&config_file).await.unwrap();
    assert_eq!(config.project_template_repository.url, "https://github.com/test-org/templates");
}
```

### Error Handling Testing

```rust
use anyhow::Result;

#[tokio::test]
async fn test_error_scenarios() {
    // Test with invalid input
    let result = function_that_should_fail("invalid_input").await;
    assert!(result.is_err());
    
    // Test specific error type
    match result {
        Err(e) => {
            let error_string = format!("{:?}", e);
            assert!(error_string.contains("expected error message"));
        }
        Ok(_) => panic!("Expected error but got success"),
    }
}
```

## Mock and Stub Patterns

### Git Repository Mocking

```rust
use std::collections::HashMap;

struct MockGitRepository {
    files: HashMap<String, String>,
    branches: Vec<String>,
}

impl MockGitRepository {
    fn new() -> Self {
        Self {
            files: HashMap::new(),
            branches: vec!["main".to_string()],
        }
    }
    
    fn add_file(&mut self, path: &str, content: &str) {
        self.files.insert(path.to_string(), content.to_string());
    }
    
    fn simulate_clone(&self, target_dir: &Path) -> Result<()> {
        for (file_path, content) in &self.files {
            let full_path = target_dir.join(file_path);
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(full_path, content)?;
        }
        Ok(())
    }
}

#[tokio::test]
async fn test_with_mock_repository() {
    let mut mock_repo = MockGitRepository::new();
    mock_repo.add_file("template.toml", r#"
name = "test-template"
description = "Test template for mocking"
"#);
    
    let temp_dir = TempDir::new("mock_test").unwrap();
    mock_repo.simulate_clone(temp_dir.path()).unwrap();
    
    // Test template discovery with mocked repository
    let collector = Collector::new(temp_dir.path().to_path_buf());
    let templates = collector.collect().await.unwrap();
    
    assert_eq!(templates.len(), 1);
    assert_eq!(templates[0].name(), "test-template");
}
```

### Network Request Mocking

```rust
use serde_json::json;

#[tokio::test]
async fn test_wallet_daemon_communication() {
    // Mock wallet daemon response
    let mock_response = json!({
        "jsonrpc": "2.0",
        "result": "pong",
        "id": 1
    });
    
    // Test JSON-RPC communication
    let response = simulate_wallet_daemon_call("ping", json!({})).await;
    assert_eq!(response["result"], "pong");
}

async fn simulate_wallet_daemon_call(method: &str, params: serde_json::Value) -> serde_json::Value {
    // Simulate network delay
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    
    // Return mocked response based on method
    match method {
        "ping" => json!({"jsonrpc": "2.0", "result": "pong", "id": 1}),
        "get_balance" => json!({"jsonrpc": "2.0", "result": {"balance": 1000000}, "id": 1}),
        _ => json!({"jsonrpc": "2.0", "error": {"code": -32601, "message": "Method not found"}, "id": 1}),
    }
}
```

## Property-Based Testing

### Input Validation Testing

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_project_name_sanitization(project_name in "\\PC*") {
        let sanitized = sanitize_project_name(&project_name);
        
        // Properties that should always hold
        prop_assert!(!sanitized.is_empty());
        prop_assert!(sanitized.chars().all(|c| c.is_alphanumeric() || c == '_'));
        prop_assert!(!sanitized.starts_with('_'));
        prop_assert!(!sanitized.ends_with('_'));
    }
}

fn sanitize_project_name(name: &str) -> String {
    // Implementation to test
    name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}
```

### Configuration Validation Testing

```rust
proptest! {
    #[test]
    fn test_url_validation(
        scheme in "(https?)",
        host in "[a-zA-Z0-9.-]+",
        port in 1u16..65535
    ) {
        let url = format!("{}://{}:{}/", scheme, host, port);
        let parsed = parse_wallet_daemon_url(&url);
        
        prop_assert!(parsed.is_ok());
        let parsed_url = parsed.unwrap();
        prop_assert_eq!(parsed_url.scheme(), scheme);
        prop_assert_eq!(parsed_url.host_str().unwrap(), host);
        prop_assert_eq!(parsed_url.port().unwrap(), port);
    }
}
```

## Performance Testing

### Benchmark Testing

```rust
use criterion::{criterion_group, criterion_main, Criterion};
use std::time::Duration;

fn bench_template_collection(c: &mut Criterion) {
    let temp_dir = setup_large_template_repository();
    
    c.bench_function("template_collection", |b| {
        b.iter(|| {
            let collector = Collector::new(temp_dir.path().to_path_buf());
            futures::executor::block_on(collector.collect()).unwrap()
        })
    });
}

fn bench_project_generation(c: &mut Criterion) {
    let template_data = setup_template_data();
    
    c.bench_function("project_generation", |b| {
        b.iter(|| {
            generate_project_from_template(&template_data).unwrap()
        })
    });
}

criterion_group!(benches, bench_template_collection, bench_project_generation);
criterion_main!(benches);
```

### Memory Usage Testing

```rust
#[tokio::test]
async fn test_memory_usage() {
    let initial_memory = get_memory_usage();
    
    // Perform memory-intensive operation
    let large_templates = collect_many_templates().await;
    
    let peak_memory = get_memory_usage();
    let memory_increase = peak_memory - initial_memory;
    
    // Assert reasonable memory usage
    assert!(memory_increase < 100_000_000); // 100MB limit
    
    // Clean up
    drop(large_templates);
    
    // Allow garbage collection
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    let final_memory = get_memory_usage();
    let memory_leaked = final_memory - initial_memory;
    
    // Assert minimal memory leakage
    assert!(memory_leaked < 10_000_000); // 10MB tolerance
}

fn get_memory_usage() -> usize {
    // Platform-specific memory measurement
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/proc/self/status")
            .unwrap()
            .lines()
            .find(|line| line.starts_with("VmRSS:"))
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|mem| mem.parse::<usize>().ok())
            .unwrap_or(0) * 1024 // Convert KB to bytes
    }
    
    #[cfg(not(target_os = "linux"))]
    {
        0 // Placeholder for other platforms
    }
}
```

## Test Utilities and Helpers

### Common Test Fixtures

```rust
pub struct TestFixtures;

impl TestFixtures {
    pub fn create_temp_dir(prefix: &str) -> TempDir {
        TempDir::new(prefix).expect("Failed to create temporary directory")
    }
    
    pub async fn create_test_template(dir: &Path, name: &str, description: &str) {
        let template_dir = dir.join(name);
        fs::create_dir_all(&template_dir).await.unwrap();
        
        let template_toml = format!(
            r#"
name = "{}"
description = "{}"
"#,
            name, description
        );
        
        fs::write(template_dir.join("template.toml"), template_toml)
            .await
            .unwrap();
    }
    
    pub async fn create_test_cargo_toml(dir: &Path, package_name: &str) {
        let cargo_toml = format!(
            r#"
[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
tari-template-lib = {{ git = "https://github.com/tari-project/tari-dan.git", branch = "development" }}
"#,
            package_name
        );
        
        fs::write(dir.join("Cargo.toml"), cargo_toml).await.unwrap();
    }
}
```

### Test Assertion Helpers

```rust
pub trait TestAssertions {
    fn assert_is_valid_project_structure(&self);
    fn assert_contains_template(&self, template_name: &str);
    fn assert_has_wasm_binary(&self, binary_name: &str);
}

impl TestAssertions for Path {
    fn assert_is_valid_project_structure(&self) {
        assert!(self.exists(), "Project directory should exist");
        assert!(self.join("Cargo.toml").exists(), "Should have Cargo.toml");
        assert!(self.join("tari.config.toml").exists(), "Should have tari.config.toml");
        assert!(self.join("templates").exists(), "Should have templates directory");
    }
    
    fn assert_contains_template(&self, template_name: &str) {
        let template_dir = self.join("templates").join(template_name);
        assert!(template_dir.exists(), "Template directory should exist");
        assert!(template_dir.join("Cargo.toml").exists(), "Template should have Cargo.toml");
        assert!(template_dir.join("src").exists(), "Template should have src directory");
    }
    
    fn assert_has_wasm_binary(&self, binary_name: &str) {
        let wasm_path = self
            .join("target")
            .join("wasm32-unknown-unknown")
            .join("release")
            .join(format!("{}.wasm", binary_name));
        assert!(wasm_path.exists(), "WASM binary should exist after build");
    }
}
```

## Test Data Management

### Environment-Specific Testing

```rust
#[tokio::test]
async fn test_with_mock_environment() {
    // Set test environment variables
    std::env::set_var("TARI_NETWORK", "test");
    std::env::set_var("RUST_LOG", "debug");
    
    // Ensure cleanup
    struct EnvGuard;
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            std::env::remove_var("TARI_NETWORK");
            std::env::remove_var("RUST_LOG");
        }
    }
    let _guard = EnvGuard;
    
    // Run test with environment variables set
    let result = environment_dependent_function().await;
    assert!(result.is_ok());
}
```

### Test Configuration

```rust
pub fn test_config() -> Config {
    Config {
        project_template_repository: TemplateRepository {
            url: "https://github.com/test-org/test-templates".to_string(),
            branch: "test".to_string(),
            folder: "test_templates".to_string(),
        },
        wasm_template_repository: TemplateRepository {
            url: "https://github.com/test-org/test-wasm-templates".to_string(),
            branch: "test".to_string(),
            folder: "test_wasm_templates".to_string(),
        },
    }
}
```

## Continuous Integration Testing

### CI Test Matrix

<!-- SOURCE: Verified against .github/workflows/ci.yml -->
The CI system runs tests across multiple configurations:

```yaml
strategy:
  matrix:
    os: [ubuntu-latest, macos-latest, windows-latest]
    rust: [stable, nightly-2025-01-09]
    features: [default, all-features]
```

### Test Stages in CI

1. **Format Check**: `cargo +nightly fmt --all -- --check`
2. **Lint Check**: `cargo lints clippy --all-targets --all-features`
3. **Build Check**: `cargo check --release --all-targets --locked`
4. **Test Execution**: `cargo nextest run --all-features --release --profile ci`
5. **License Validation**: Custom script checking file headers

### Coverage Reporting

```bash
# Generate coverage report
cargo install grcov
export CARGO_INCREMENTAL=0
export RUSTFLAGS="-Cinstrument-coverage"
cargo test
grcov . --binary-path ./target/debug/deps/ -s . -t html --branch --ignore-not-existing -o coverage/
```

## Testing Best Practices

### Test Organization

1. **Naming**: Use descriptive test names that explain what is being tested
2. **Structure**: Follow Arrange-Act-Assert pattern
3. **Isolation**: Each test should be independent and not rely on others
4. **Cleanup**: Use RAII patterns and guards for resource cleanup
5. **Documentation**: Document complex test scenarios

### Error Testing

```rust
#[tokio::test]
async fn test_error_conditions() {
    // Test each error condition explicitly
    let invalid_url_result = parse_url("invalid-url");
    assert!(matches!(invalid_url_result, Err(ConfigError::InvalidUrl(_))));
    
    let network_error_result = connect_to_daemon("http://non-existent:9000").await;
    assert!(matches!(network_error_result, Err(DeploymentError::NetworkError(_))));
}
```

### Async Testing Patterns

```rust
#[tokio::test]
async fn test_concurrent_operations() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(10);
    
    // Spawn concurrent tasks
    let handles: Vec<_> = (0..5)
        .map(|i| {
            let tx = tx.clone();
            tokio::spawn(async move {
                let result = async_operation(i).await;
                tx.send(result).await.unwrap();
            })
        })
        .collect();
    
    // Collect results
    let mut results = Vec::new();
    for _ in 0..5 {
        results.push(rx.recv().await.unwrap());
    }
    
    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }
    
    // Verify results
    assert_eq!(results.len(), 5);
    assert!(results.iter().all(|r| r.is_ok()));
}
```

## Test Maintenance

### Keeping Tests Current

1. **Regular Updates**: Update tests when APIs change
2. **Dependency Updates**: Ensure tests work with new dependency versions
3. **Performance Monitoring**: Track test execution time and identify slow tests
4. **Flaky Test Detection**: Identify and fix non-deterministic tests

### Test Documentation

```rust
/// Tests the template collection functionality with various template structures.
/// 
/// This test verifies that:
/// - Templates with basic metadata are discovered correctly
/// - Templates with extra configuration are parsed properly
/// - The collector handles nested directory structures
/// - Invalid templates are skipped gracefully
/// 
/// Test data includes templates with different complexity levels to ensure
/// the collector works across the full range of expected inputs.
#[tokio::test]
async fn test_template_collection_comprehensive() {
    // Test implementation...
}
```

---

**Testing is crucial for maintaining code quality**. Always write tests for new functionality, and ensure existing tests pass before submitting changes. For more details on the development workflow, see the [Development Setup Guide](development-setup.md).

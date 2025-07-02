---
title: Advanced Debugging Guide
description: Deep troubleshooting techniques for Tari CLI using actual error patterns and debug methods
last_updated: 2025-06-26
version: Latest (main branch)
verified_against: Error handling patterns from crates/cli/src/**/*.rs
audience: developers
---

# Advanced Debugging Guide

> **Deep troubleshooting** techniques for diagnosing and resolving complex Tari CLI issues

## Debug Environment Setup

### Enable Comprehensive Logging

<!-- SOURCE: Verified against CI configuration and error handling patterns -->
The Tari CLI uses Rust's `log` crate for detailed debugging information:

```bash
# Enable all debug logging
export RUST_LOG=debug
tari create my-project

# Focus on specific modules
export RUST_LOG=tari_cli::templates=debug,tari_cli::cli::commands=trace
tari new my-template

# Include dependency logging
export RUST_LOG=tari_cli=debug,cargo_generate=debug,git2=info
tari create my-project
```

### Debug Output Interpretation

**Log Level Hierarchy**:
- `ERROR`: Critical failures requiring immediate attention
- `WARN`: Potential issues that don't prevent operation
- `INFO`: General operational information
- `DEBUG`: Detailed execution flow information
- `TRACE`: Extremely verbose execution details

**Example Debug Output**:
```
[2025-06-26T17:26:50Z DEBUG tari_cli::templates::collector] Scanning directory: /path/to/templates
[2025-06-26T17:26:50Z TRACE tari_cli::templates::collector] Found file: template.toml
[2025-06-26T17:26:50Z DEBUG tari_cli::templates::collector] Parsing template: basic-template
[2025-06-26T17:26:50Z ERROR tari_cli::cli::commands::create] Template validation failed: missing required field 'name'
```

## Error Analysis Patterns

### CLI Error Categories

<!-- SOURCE: Verified against actual error types in codebase -->
The Tari CLI uses structured error types for systematic debugging:

#### 1. Configuration Errors

**URL Parsing Errors**:
```rust
// From crates/cli/src/project/config.rs
#[derive(Error, Debug)]
pub enum Error {
    #[error("URL parsing error: {0}")]
    Parse(#[from] url::ParseError),
}
```

**Debugging Steps**:
```bash
# Test URL parsing manually
curl -I http://127.0.0.1:9000/

# Validate configuration syntax
toml check tari.config.toml

# Check URL accessibility
telnet 127.0.0.1 9000
```

#### 2. Template Discovery Errors

**Template Collection Errors**:
```rust
// From crates/cli/src/templates/collector.rs
#[derive(Error, Debug)]
pub enum Error {
    #[error("Git2 error: {0}")]
    IO(#[from] io::Error),
    #[error("Failed to deserialize TOML: {0}")]
    TomlDeserialize(#[from] toml::de::Error),
}
```

**Debug Template Discovery**:
```bash
# Enable template collection debugging
RUST_LOG=tari_cli::templates=trace tari create test-project

# Manually check template repository
ls -la ~/.local/share/tari_cli/template_repositories/

# Validate template descriptors
find ~/.local/share/tari_cli/template_repositories/ -name "template.toml" -exec cat {} \;
```

#### 3. Command Execution Errors

**Create Command Errors**:
```rust
// From crates/cli/src/cli/commands/create.rs
#[derive(Error, Debug)]
pub enum CreateHandlerError {
    #[error("Template not found by name: {0}. Possible values: {1:?}")]
    TemplateNotFound(String, Vec<String>),
}
```

**Debugging Command Failures**:
```bash
# Trace command execution
RUST_LOG=tari_cli::cli::commands::create=trace tari create my-project

# Debug template selection logic
RUST_LOG=tari_cli::templates::collector=debug tari create my-project --template basic

# Check workspace detection
RUST_LOG=tari_cli::cli::commands::new=debug tari new my-template
```

## Step-by-Step Debugging Process

### Phase 1: Initial Diagnosis

**Collect System Information**:
```bash
# CLI version and build info
tari --version

# System environment
echo "OS: $(uname -a)"
echo "Rust: $(rustc --version)"
echo "Cargo: $(cargo --version)"

# Environment variables
env | grep -E "(RUST_|CARGO_|TARI_)"

# Directory permissions
ls -la ~/.local/share/tari_cli/
ls -la ~/.config/tari_cli/
```

**Test Basic Functionality**:
```bash
# Test CLI installation
which tari
tari --help

# Test configuration loading
tari create --help

# Test repository access
timeout 10s tari create test-connectivity 2>&1 | head -20
```

### Phase 2: Command-Specific Debugging

#### Debug `create` Command

**Template Repository Issues**:
```bash
# Enable git operations logging
RUST_LOG=tari_cli::git=debug,git2=info tari create test-project

# Check repository cloning manually
git clone https://github.com/tari-project/wasm-template.git /tmp/test-templates
ls -la /tmp/test-templates/project_templates/

# Test template collection
RUST_LOG=tari_cli::templates::collector=trace tari create test-project
```

**Network Connectivity Issues**:
```bash
# Test git repository access
git ls-remote https://github.com/tari-project/wasm-template.git

# Check proxy/firewall issues
curl -I https://github.com/tari-project/wasm-template.git

# Test with custom repository
tari -e "project_template_repository.url=https://github.com/my-org/templates" \
     create test-project
```

#### Debug `new` Command

**Workspace Detection Issues**:
```bash
# Enable workspace debugging
RUST_LOG=tari_cli::cli::commands::new=debug tari new my-template

# Check Cargo.toml workspace configuration
cat Cargo.toml | grep -A 5 "\[workspace\]"

# Validate workspace member detection
cargo metadata --format-version 1 | jq '.workspace_members'

# Test outside workspace
mkdir /tmp/non-workspace && cd /tmp/non-workspace
tari new test-template
```

**Template Generation Issues**:
```bash
# Debug cargo-generate integration
RUST_LOG=cargo_generate=debug tari new my-template

# Test cargo-generate directly
cargo generate --git https://github.com/tari-project/wasm-template.git \
                --name test-template \
                --destination ./test-output
```

#### Debug `deploy` Command

**WASM Compilation Issues**:
```bash
# Enable build debugging
RUST_LOG=tari_cli::cli::commands::deploy=debug tari deploy --account test my-template

# Test WASM compilation manually
cd templates/my-template
RUST_LOG=debug cargo build --target wasm32-unknown-unknown --release

# Check WASM target installation
rustup target list | grep wasm32-unknown-unknown
rustup target add wasm32-unknown-unknown
```

**Deployment Network Issues**:
```bash
# Test wallet daemon connection
curl -X POST http://127.0.0.1:9000/ \
     -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","id":1,"method":"ping"}'

# Enable deployment library debugging
RUST_LOG=tari_deploy=debug tari deploy --account test my-template

# Check account existence and balance
# (Requires wallet daemon JSON-RPC inspection)
```

### Phase 3: Advanced Debugging Techniques

#### Memory and Performance Analysis

**Enable Tokio Console** (requires rebuild with tokio-console features):
```bash
# Rebuild with console support
cargo build --features tokio-console

# Run with console
RUST_LOG=tokio=trace tari create my-project &
tokio-console
```

**Profile Resource Usage**:
```bash
# Monitor system resources
htop -p $(pgrep tari)

# Track file operations
strace -e trace=file tari create my-project

# Monitor network operations
strace -e trace=network tari create my-project
```

#### Git Repository Debugging

**Trace Git Operations**:
```bash
# Enable libgit2 debugging
export RUST_LOG=git2=trace
tari create my-project

# Manual repository inspection
cd ~/.local/share/tari_cli/template_repositories/
find . -name ".git" -type d
git log --oneline -10  # in each repository
```

**Repository State Validation**:
```bash
# Check repository integrity
cd ~/.local/share/tari_cli/template_repositories/tari-project/wasm-template/
git status
git remote -v
git branch -a
git log --oneline -5

# Verify template structure
find . -name "template.toml" -exec echo "=== {} ===" \; -exec cat {} \;
```

#### Configuration Debugging

**Trace Configuration Loading**:
```bash
# Debug configuration hierarchy
RUST_LOG=tari_cli::cli::config=trace tari create test-project

# Test configuration overrides
RUST_LOG=debug tari -e "project_template_repository.url=https://custom.git" \
                   create test-project

# Validate TOML parsing
toml_validator ~/.config/tari_cli/tari.config.toml
```

## Common Debug Scenarios

### Scenario 1: Template Not Found

**Symptoms**:
```
Template not found by name: basic. Possible values: ["advanced", "minimal"]
```

**Debug Process**:
```bash
# 1. Check template repository contents
RUST_LOG=tari_cli::templates::collector=debug tari create test-project

# 2. Manually inspect repository
ls -la ~/.local/share/tari_cli/template_repositories/tari-project/wasm-template/project_templates/

# 3. Validate template descriptors
find ~/.local/share/tari_cli/template_repositories/ -name "template.toml" \
     -exec echo "=== {} ===" \; -exec cat {} \;

# 4. Test with fresh repository
rm -rf ~/.local/share/tari_cli/template_repositories/
tari create test-project
```

### Scenario 2: Network Connectivity Issues

**Symptoms**:
```
Failed to clone repository: network operation failed
```

**Debug Process**:
```bash
# 1. Test basic connectivity
ping github.com
curl -I https://github.com

# 2. Test git operations
git ls-remote https://github.com/tari-project/wasm-template.git

# 3. Check proxy settings
echo $http_proxy $https_proxy
git config --global http.proxy
git config --global https.proxy

# 4. Test with alternative repository
tari -e "project_template_repository.url=https://gitlab.com/my-org/templates" \
     create test-project
```

### Scenario 3: WASM Compilation Failures

**Symptoms**:
```
Failed to build project: compilation failed
error: linking with `rust-lld` failed
```

**Debug Process**:
```bash
# 1. Verify WASM target installation
rustup target list | grep wasm32-unknown-unknown
rustup target add wasm32-unknown-unknown

# 2. Test manual compilation
cd templates/my-template
cargo check --target wasm32-unknown-unknown
RUST_LOG=debug cargo build --target wasm32-unknown-unknown --release

# 3. Check dependencies
cargo tree --target wasm32-unknown-unknown
cargo audit

# 4. Validate Cargo.toml
cat Cargo.toml | grep -A 10 "\[lib\]"
cat Cargo.toml | grep -A 5 "crate-type"
```

### Scenario 4: Deployment Authentication Issues

**Symptoms**:
```
Authentication failed
Connection refused (os error 61)
```

**Debug Process**:
```bash
# 1. Test wallet daemon connectivity
curl -v http://127.0.0.1:9000/

# 2. Check daemon process
ps aux | grep tari_wallet_daemon
netstat -an | grep 9000

# 3. Test JSON-RPC communication
curl -X POST http://127.0.0.1:9000/ \
     -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","id":1,"method":"ping"}' \
     -v

# 4. Enable deployment debugging
RUST_LOG=tari_deploy=debug tari deploy --account test my-template
```

## Custom Debug Tools

### Create Debug Script

```bash
#!/bin/bash
# debug-tari.sh - Comprehensive debugging script

echo "=== Tari CLI Debug Information ==="
echo "Date: $(date)"
echo "CLI Version: $(tari --version 2>/dev/null || echo 'NOT INSTALLED')"
echo "Rust Version: $(rustc --version 2>/dev/null || echo 'NOT INSTALLED')"
echo "OS: $(uname -a)"
echo ""

echo "=== Environment Variables ==="
env | grep -E "(RUST_|CARGO_|TARI_)" | sort
echo ""

echo "=== File System Check ==="
echo "Base directory: ${HOME}/.local/share/tari_cli/"
ls -la ~/.local/share/tari_cli/ 2>/dev/null || echo "NOT FOUND"
echo ""
echo "Config directory: ${HOME}/.config/tari_cli/"
ls -la ~/.config/tari_cli/ 2>/dev/null || echo "NOT FOUND"
echo ""

echo "=== Network Connectivity ==="
echo "GitHub connectivity:"
curl -I https://github.com 2>/dev/null | head -1 || echo "FAILED"
echo "Wallet daemon connectivity:"
curl -s -X POST http://127.0.0.1:9000/ \
     -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","id":1,"method":"ping"}' \
     | grep -o '"result":"pong"' || echo "FAILED"
echo ""

echo "=== Template Repositories ==="
find ~/.local/share/tari_cli/template_repositories/ -name "template.toml" 2>/dev/null | wc -l \
    | xargs echo "Template descriptors found:"
echo ""

echo "=== WASM Target Check ==="
rustup target list 2>/dev/null | grep wasm32-unknown-unknown || echo "WASM target not found"
echo ""

echo "=== Test Basic CLI Function ==="
timeout 5s tari --help >/dev/null 2>&1 && echo "CLI responds normally" || echo "CLI unresponsive"
```

### Template Validation Tool

```bash
#!/bin/bash
# validate-templates.sh - Validate template repository structure

REPO_DIR="${HOME}/.local/share/tari_cli/template_repositories"

if [ ! -d "$REPO_DIR" ]; then
    echo "❌ Template repository directory not found: $REPO_DIR"
    exit 1
fi

echo "🔍 Validating template repositories..."

find "$REPO_DIR" -name "template.toml" | while read -r template_file; do
    echo ""
    echo "📄 Checking: $template_file"
    
    # Check TOML syntax
    if ! toml_lint "$template_file" 2>/dev/null; then
        echo "❌ Invalid TOML syntax"
        continue
    fi
    
    # Check required fields
    if ! grep -q "^name = " "$template_file"; then
        echo "❌ Missing required field: name"
    fi
    
    if ! grep -q "^description = " "$template_file"; then
        echo "❌ Missing required field: description"
    fi
    
    # Check template directory structure
    template_dir=$(dirname "$template_file")
    if [ ! -f "$template_dir/Cargo.toml" ]; then
        echo "⚠️  No Cargo.toml found in template directory"
    fi
    
    echo "✅ Template validation complete"
done
```

## Performance Debugging

### Execution Time Analysis

```bash
# Time different operations
time tari create test-project
time tari new test-template
time tari deploy --account test test-template
```

### Memory Usage Monitoring

```bash
# Monitor memory usage during operations
/usr/bin/time -v tari create large-project 2>&1 | grep -E "(Maximum resident|User time|System time)"

# Use valgrind for detailed memory analysis (Linux)
valgrind --tool=massif tari create test-project
ms_print massif.out.* | head -30
```

### Async Operation Analysis

```bash
# Enable tokio debugging for async operation insight
RUST_LOG=tokio=debug,tari_cli=debug tari create test-project 2>&1 | \
    grep -E "(spawn|poll|wake)"
```

## Production Debugging

### CI/CD Environment Debugging

```yaml
# .github/workflows/debug-ci.yml
name: Debug CI Environment
on: [workflow_dispatch]

jobs:
  debug:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Debug Environment
        run: |
          echo "=== Environment ==="
          env | sort
          
          echo "=== File System ==="
          ls -la /
          df -h
          
          echo "=== Network ==="
          curl -I https://github.com
          
          echo "=== Rust Setup ==="
          rustc --version
          rustup target list | grep wasm32
          
      - name: Test CLI Installation
        run: |
          cargo install tari-cli --git https://github.com/tari-project/tari-cli
          tari --version
          
      - name: Debug Template Access
        run: |
          RUST_LOG=debug tari create ci-test-project --template basic
```

### Production Error Collection

```bash
# Structured error reporting for production issues
tari_debug() {
    local output_file="/tmp/tari-debug-$(date +%s).log"
    
    {
        echo "=== Tari CLI Debug Report ==="
        echo "Timestamp: $(date -Iseconds)"
        echo "Command: $*"
        echo "Working Directory: $(pwd)"
        echo "User: $(whoami)"
        echo ""
        
        echo "=== Environment ==="
        env | grep -E "(RUST_|CARGO_|TARI_)" | sort
        echo ""
        
        echo "=== CLI Execution ==="
        RUST_LOG=debug timeout 30s tari "$@" 2>&1
        echo "Exit code: $?"
        
    } > "$output_file" 2>&1
    
    echo "Debug report saved to: $output_file"
    echo "Please include this file when reporting issues."
}

# Usage: tari_debug create my-project
```

---

**Remember**: When debugging complex issues, always start with the simplest possible test case and gradually add complexity. The structured error types and comprehensive logging in Tari CLI are designed to make debugging systematic and efficient.

**For additional help**: Review [Common Issues](common-issues.md) for frequently encountered problems and solutions.

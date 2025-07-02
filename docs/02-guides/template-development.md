---
Last Updated: 2025-06-26
Version: Latest (main branch)
Verified Against: crates/cli/src/cli/commands/*.rs, crates/cli/src/templates/collector.rs
Test Sources: crates/cli/src/templates/collector.rs:154-200
Implementation: Template discovery and generation system
---

# Template Development Guide

This guide covers developing custom templates for the Tari CLI, including project templates and WASM smart contract templates.

## Template Types

The Tari CLI supports two types of templates:

1. **Project Templates**: Complete development environments with configuration
2. **WASM Templates**: Individual smart contract templates for specific use cases

## Creating Project Templates

Project templates provide the foundation for Tari smart contract development.

### Template Repository Structure

```
your-project-template/
├── template.toml              # Template descriptor
├── Cargo.toml                 # Workspace configuration
├── tari.config.toml          # Network configuration
├── src/                       # Template source code
├── templates/                 # WASM template directory (optional)
└── README.md                  # Template documentation
```

### Template Descriptor (`template.toml`)

<!-- SOURCE: crates/cli/src/templates/collector.rs:136-152 -->
<!-- VERIFIED: 2025-06-26 from test implementation -->
```toml
name = "project-template-name"
description = "Complete project template for Tari smart contract development"

[extra]
templates_dir = "templates"
wasm_templates = "true"
```

**Required Fields:**
- `name`: Template identifier (will be converted to snake_case)
- `description`: Human-readable description shown during selection

**Extra Configuration:**
- `templates_dir`: Directory containing WASM templates (default: current directory)
- `wasm_templates`: Set to "true" to indicate this project contains WASM templates

### Project Configuration Template

Include a default `tari.config.toml`:

```toml
[network]
wallet-daemon-jrpc-address = "http://127.0.0.1:9000/"
```

### Cargo Workspace Configuration

Project templates should include a `Cargo.toml` workspace:

```toml
[workspace]
members = []
resolver = "2"

[workspace.dependencies]
# Common dependencies for Tari templates
tari-template-lib = { git = "https://github.com/tari-project/tari-dan.git", branch = "development" }
serde = { version = "1.0", features = ["derive"] }
```

## Creating WASM Templates

WASM templates are individual smart contracts with specific functionality.

### WASM Template Structure

```
nft-template/
├── template.toml              # Template descriptor
├── Cargo.toml                 # Crate configuration
├── src/
│   └── lib.rs                # Smart contract implementation
└── README.md                  # Template usage guide
```

### WASM Template Descriptor

```toml
name = "nft-template"
description = "A simple NFT template for creating unique digital assets"

# Optional: Additional metadata
[extra]
category = "tokens"
complexity = "beginner"
```

### WASM Cargo Configuration

```toml
[package]
name = "{{project-name | snake_case}}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
tari-template-lib = { git = "https://github.com/tari-project/tari-dan.git", branch = "development" }
serde = { version = "1.0", features = ["derive"] }

# WASM optimization
[profile.release]
opt-level = "s"
lto = true
codegen-units = 1
panic = "abort"
```

### Smart Contract Implementation

Basic WASM template structure:

```rust
use tari_template_lib::prelude::*;

#[template]
mod {{project-name | snake_case}} {
    use super::*;

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct {{project-name | pascal_case}} {
        // Contract state fields
    }

    impl {{project-name | pascal_case}} {
        pub fn new() -> Component<Self> {
            Component::new(Self {
                // Initialize state
            })
        }

        pub fn my_method(&mut self) -> String {
            // Contract method implementation
            "Hello from Tari!".to_string()
        }
    }
}
```

## Template Variables

Templates support cargo-generate variables for customization:

### Common Variables

- `{{project-name}}`: Project name as provided by user
- `{{project-name | snake_case}}`: Snake case version
- `{{project-name | pascal_case}}`: Pascal case version
- `{{project-name | kebab-case}}`: Kebab case version

### Custom Variables

Define custom variables in `template.toml`:

```toml
name = "advanced-template"
description = "Advanced template with custom variables"

[extra]
author = "Your Name"
license = "MIT"

# Custom variables for cargo-generate
[template]
cargo_generate_version = ">=0.10.0"

# Prompt for additional variables
[[template.placeholders]]
name = "contract_type"
type = "string"
prompt = "What type of contract? (token, nft, defi)"
choices = ["token", "nft", "defi"]
```

Use variables in template files:

```rust
// In src/lib.rs
/// {{contract_type | title_case}} contract implementation
/// Author: {{author}}
/// License: {{license}}
```

## Template Repository Management

### Repository Setup

1. **Create Git Repository**
   ```bash
   git init my-tari-templates
   cd my-tari-templates
   ```

2. **Add Template Structure**
   ```bash
   mkdir -p basic-project/src
   mkdir -p templates/nft/src
   mkdir -p templates/token/src
   ```

3. **Configure CLI to Use Repository**
   
   Templates are automatically discovered from configured repositories. The CLI scans for `template.toml` files.

### Multiple Templates per Repository

<!-- SOURCE: Template discovery logic from collector.rs -->
Organize multiple templates in subdirectories:

```
tari-templates-repo/
├── README.md
├── basic-project/
│   ├── template.toml
│   ├── Cargo.toml
│   └── src/
├── templates/
│   ├── nft/
│   │   ├── template.toml
│   │   ├── Cargo.toml
│   │   └── src/
│   └── token/
│       ├── template.toml
│       ├── Cargo.toml
│       └── src/
```

The CLI automatically discovers all templates with `template.toml` descriptors.

## Testing Templates

### Template Validation

Before publishing, validate your templates:

1. **Test Template Generation**
   ```bash
   # Test project template
   tari create test-project --template your-template-name
   
   # Test WASM template
   cd test-project
   tari new test-contract --template your-wasm-template
   ```

2. **Test Compilation**
   ```bash
   cd test-project/test-contract
   cargo check --target wasm32-unknown-unknown
   cargo build --target wasm32-unknown-unknown --release
   ```

3. **Test Deployment**
   ```bash
   # Ensure wallet daemon is running
   tari deploy --account test-account test-contract
   ```

### Automated Testing

Create GitHub Actions for template validation:

```yaml
name: Template Validation
on: [push, pull_request]

jobs:
  test-templates:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          target: wasm32-unknown-unknown
      - name: Test template generation
        run: |
          # Install tari CLI
          cargo install tari-cli --git https://github.com/tari-project/tari-cli
          
          # Test each template
          for template in templates/*/; do
            template_name=$(basename "$template")
            tari create "test-$template_name" --template "$template_name"
            cd "test-$template_name"
            cargo check --target wasm32-unknown-unknown
            cd ..
          done
```

## Best Practices

### Template Design

1. **Keep Templates Focused**: Each template should solve a specific use case
2. **Provide Clear Documentation**: Include comprehensive README files
3. **Use Meaningful Names**: Template names should clearly indicate their purpose
4. **Include Examples**: Provide working examples and test cases

### Code Organization

1. **Follow Rust Conventions**: Use standard Rust project structure
2. **Optimize for WASM**: Use WASM-compatible dependencies only
3. **Handle Errors Gracefully**: Implement proper error handling
4. **Document Public APIs**: Use rustdoc comments for all public functions

### Security Considerations

1. **Validate Inputs**: Always validate parameters and state changes
2. **Avoid Panics**: Use `Result` types instead of `panic!`
3. **Limit Resource Usage**: Be mindful of gas costs and computation limits
4. **Test Edge Cases**: Include comprehensive test coverage

### Repository Management

1. **Version Templates**: Use git tags for template versions
2. **Maintain Compatibility**: Ensure templates work with current Tari CLI
3. **Update Dependencies**: Regularly update Tari template library versions
4. **Provide Migration Guides**: Document breaking changes between versions

## Publishing Templates

### Official Template Repository

The default template repository is maintained at:
- Project Templates: https://github.com/tari-project/wasm-template
- WASM Templates: https://github.com/tari-project/wasm-template

### Community Templates

To share community templates:

1. **Create Public Repository**: Host templates on GitHub or similar platform
2. **Follow Conventions**: Use the template structure described in this guide
3. **Add Documentation**: Include clear usage instructions
4. **Test Thoroughly**: Validate templates work with latest Tari CLI

### Template Submission

For inclusion in official repositories:

1. **Fork Repository**: Fork the appropriate template repository
2. **Add Template**: Follow existing template structure and conventions
3. **Test Locally**: Ensure template works with current CLI version
4. **Submit Pull Request**: Include description and testing instructions
5. **Update Documentation**: Add template to repository documentation

## Troubleshooting

### Common Issues

1. **Template Not Found**: Verify `template.toml` exists and is properly formatted
2. **Compilation Errors**: Check WASM target and dependency compatibility
3. **Generation Fails**: Verify template variables and file structure
4. **Import Errors**: Ensure all required files are included in template

### Debug Template Discovery

<!-- SOURCE: Template collector implementation -->
Templates are discovered by scanning repositories for `template.toml` files. To debug:

1. **Check Repository Structure**: Ensure templates follow expected directory structure
2. **Validate TOML Syntax**: Use `toml` parser to validate descriptor files
3. **Test Repository Access**: Verify git repository is accessible and templates are in expected branches

### Testing Individual Templates

```bash
# Clone template repository locally
git clone https://github.com/your-org/your-templates local-templates

# Test template discovery
cd local-templates
find . -name "template.toml" -exec echo "Found: {}" \;

# Test template generation with local path
tari create test-project --template your-template-name
```

This comprehensive guide should help developers create, test, and publish high-quality templates for the Tari ecosystem.

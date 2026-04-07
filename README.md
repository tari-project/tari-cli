# Tari Ootle CLI

[![CI Build Status](https://img.shields.io/github/actions/workflow/status/tari-project/tari-cli/pr-check.yml)](https://github.com/tari-project/tari-cli/actions/workflows/pr-check.yml)
[![Crates.io](https://img.shields.io/crates/v/tari-ootle-cli)](https://crates.io/crates/tari-ootle-cli)
[![docs.rs](https://img.shields.io/docsrs/tari-ootle-cli)](https://docs.rs/tari-ootle-cli)
[![Docs](https://img.shields.io/badge/docs-tari--cli-blue)](https://tari-project.github.io/tari-cli/)

The **Tari CLI** is the development tool for building and publishing smart contract templates on the [Tari Ootle](https://www.tari.com/) Layer-2 network.

## Quick Start

### Install

```bash
cargo install tari-ootle-cli
```

Requires the WASM target:

```bash
rustup target add wasm32-unknown-unknown
```

### Create a template

```bash
tari create my-token
```

You'll be prompted to pick a starter template. This scaffolds a crate with `build.rs` and metadata configuration ready to go.

### Build

```bash
cd my-token
tari build
# ✅ WASM binary: target/.../my_token.wasm (42.3 KB)
# 📄 Metadata:    target/.../template_metadata.cbor
```

### Inspect metadata

```bash
tari metadata inspect

#   Name:           my-token
#   Version:        0.1.0
#   Tags:           token, defi
#   Category:       token
#   Logo URL:       https://example.com/logo.png
#   Metadata hash:  ...
```

### Publish to network

```bash
tari publish -a myaccount

# ✅ WASM size: 42.3 KB
# 🔑 Metadata hash: ...
# ⭐ Your new template's address: template_f807989828e70a...
```

### Publish metadata to community server

```bash
# Hash-verified (template must have been published with metadata hash)
tari metadata publish -t template_f807989828e70a...

# Author-signed (signs via wallet daemon, no secret keys in the CLI)
tari metadata publish -t template_f807989828e70a... --signed
```

## Commands

| Command | Description |
|---------|-------------|
| `tari create [NAME]` | Create a new template crate (interactive if name omitted) |
| `tari build [PATH]` | Build the WASM binary |
| `tari publish [PATH]` | Publish template to the network |
| `tari template init` | Set up metadata generation in an existing crate |
| `tari template inspect` | Inspect built metadata |
| `tari metadata publish` | Publish metadata to a community server |
| `tari metadata inspect` | Inspect built metadata (alias) |
| `tari config init/set/get/show` | Manage project configuration |

Run `tari --help` or `tari <command> --help` for full details.

## Configuration

Project config lives in `tari.config.toml` (created by `tari config init` or the wizard):

```toml
[network]
wallet-daemon-jrpc-address = "http://127.0.0.1:9000/json_rpc"

# metadata-server-url = "http://localhost:3000"
# default_account = "myaccount"
```

Settings are resolved: **CLI flag > project config > global config > default**.

See the [Configuration Schema Reference](https://tari-project.github.io/tari-cli/03-reference/configuration-schema/) for all options.

## Documentation

Full documentation is available at **[tari-project.github.io/tari-cli](https://tari-project.github.io/tari-cli/)**.

- [CLI Commands Reference](https://tari-project.github.io/tari-cli/03-reference/cli-commands/)
- [Configuration Schema](https://tari-project.github.io/tari-cli/03-reference/configuration-schema/)
- [Getting Started](https://tari-project.github.io/tari-cli/01-getting-started/quick-start/)

## Prerequisites

- [Tari Wallet Daemon](https://github.com/tari-project/tari-dan) running and accessible
- Rust toolchain with `wasm32-unknown-unknown` target

## License

BSD-3-Clause. See [LICENSE](LICENSE).

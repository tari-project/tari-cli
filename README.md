# tari-cli

![GitHub Release](https://img.shields.io/github/v/release/tari-project/tari-cli)
![CI Build Status](https://img.shields.io/github/actions/workflow/status/tari-project/tari-cli/ci.yml)

This CLI tool is the starting point for the development of Tari templates (smart contracts in other blockchains).

# Installation

### Using cargo

```shell
cargo install tari-cli --git https://github.com/tari-project/tari-cli --force
```

### Downloading binaries

You can download latest binary from [Releases](https://github.com/tari-project/tari-cli/releases) page.

# Prerequisites

- A locally
  running [Tari Ootle Wallet Daemon](https://github.com/tari-project/tari-dan?tab=readme-ov-file#running-the-tari-dan-wallet-daemon).
- Properly configured project (worth checking after creating a new one) pointing to the right **wallet daemon JSON-RPC
  URL** (in `project_dir/tari.config.toml`).
    - Example:
      ```toml
      [networks.local]
      wallet-daemon-jrpc-address = "http://127.0.0.1:12009/"
      ```

# Usage

1. After [installation](#Installation) of the latest version it is recommended to create a new project:

    ```shell
    tari create YOUR_PROJECT_NAME
    ```

   Example output:
    ```shell
    $ tari create test                                                                                                                                                                                                                                                                                                                           [11:26:24]
    ✅ Init configuration and directories
    ✅ Refresh project templates repository
    ✅ Refresh wasm templates repository
    ✅ Collecting available project templates
    🔎 Select project template: Basic - The basic project template to get started on wasm template development.
    ⠋ Generate new project[1/5] ⠁
    ✅ Generate new project
    ```
   **Please note** that currently this will create a new skeleton project which contains configuration and everything to
   create any new **Tari template/smart contract** project!

2. Create new template
    ```shell
    tari new YOUR_TEMPLATE_PROJECT_NAME
    ```

   Example output:
    ```shell
   $ tari new SomeNFT                                                                                                                                                                                                                                                                                                                      [11:30:48]
    ✅ Init configuration and directories
    ✅ Refresh project templates repository
    ✅ Refresh wasm templates repository
    ✅ Collecting available WASM templates
    🔎 Select WASM template: NFT - A simple NFT template to create your own.
    ⠋ Generate new project[ 1/10] ⠁
    ✅ Generate new project
    ✅ Update Cargo.toml
    ```

3. Deploy new template

   **Important**: You should have an account created with enough funds to deploy!

   ```shell
   tari deploy --account YOUR_ACCOUNT_NAME_OR_ADDRESS YOUR_TEMPLATE_PROJECT_NAME
   ```

   Example output:
    ```shell
    tari deploy --account acc some_nft                                                                                                                                                                                                                                                                                                    [11:50:43]
    ✅ Init configuration and directories
    ✅ Refresh project templates repository
    ✅ Refresh wasm templates repository
    ✅ Building WASM template project "some_nft"
    ❓Deploying this template costs 256875 XTR (estimated), are you sure to continue? yes
    ✅ Deploying project "some_nft" to local network
    ⭐ Your new template's address: f807989828e70a18050e5785f30a7bd01475797d76d6b4700af175b859c32774
    ```
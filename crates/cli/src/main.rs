// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use clap::Parser;
use std::process::exit;

use crate::cli::arguments::Cli;

mod cli;
mod git;
mod project;
mod templates;

#[tokio::main]
async fn main() {
    if let Err(error) = Cli::parse().handle_command().await {
        println!("❌ {error:?}");
        exit(1);
    }

    exit(0);
}

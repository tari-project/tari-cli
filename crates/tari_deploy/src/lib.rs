// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause
//! Tari Deploy library helps developers register new templates on Tari Layer-1 chain and
//! manage Layer-2 resources for seamless development flow creating and working with Tari templates.

mod config;
pub mod deployer;
mod error;
pub mod uploader;

pub use config::*;
pub use error::Error as DeployerError;

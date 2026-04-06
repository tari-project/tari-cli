// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

pub mod publish;

use clap::Subcommand;

use crate::cli::commands::template::inspect_metadata::InspectMetadataArgs;
use publish::PublishMetadataArgs;

#[derive(Clone, Subcommand)]
pub enum MetadataCommand {
    /// Publish template metadata to a metadata server.
    Publish {
        #[clap(flatten)]
        args: PublishMetadataArgs,
    },
    /// Inspect a template metadata CBOR file.
    Inspect {
        #[clap(flatten)]
        args: InspectMetadataArgs,
    },
}

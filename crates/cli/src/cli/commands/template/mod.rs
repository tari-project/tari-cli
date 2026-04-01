// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

pub mod init_metadata;
pub mod inspect_metadata;
pub mod publish;

use clap::Subcommand;

use init_metadata::InitMetadataArgs;
use inspect_metadata::InspectMetadataArgs;
use publish::TemplatePublishArgs;

#[derive(Clone, Subcommand)]
pub enum TemplateCommand {
    /// Set up an existing template crate for metadata generation.
    #[clap(alias = "init-metadata")]
    Init {
        #[clap(flatten)]
        args: InitMetadataArgs,
    },
    /// Publish a template with its metadata hash.
    Publish {
        #[clap(flatten)]
        args: TemplatePublishArgs,
    },
    /// Inspect a template metadata CBOR file.
    InspectMetadata {
        #[clap(flatten)]
        args: InspectMetadataArgs,
    },
}

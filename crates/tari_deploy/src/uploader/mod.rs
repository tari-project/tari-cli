// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

mod local;

pub use local::*;
use std::io;

use async_trait::async_trait;
use std::path::Path;
use thiserror::Error;
use url::Url;

/// Possible errors for [`TemplateBinaryUploader`].
#[derive(Error, Debug)]
pub enum Error {
    #[error("I/O error: {0}")]
    IO(#[from] io::Error),
    #[error("HTTP call error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Template binary upload failed: {0}")]
    UploadFailed(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[async_trait]
pub trait TemplateBinaryUploader {
    async fn upload(&self, binary: &Path) -> Result<Url>;
}

// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use crate::uploader::{Error, TemplateBinaryUploader};
use async_trait::async_trait;
use reqwest::multipart::Form;
use reqwest::Client;
use serde::Deserialize;
use std::path::Path;
use url::Url;

#[derive(Deserialize)]
struct UploadResponse {
    success: bool,
    template_url: Option<Url>,
    error: String,
}

/// Template binary uploader for the locally running swarm.
pub struct LocalSwarmUploader {
    swarm_web_upload_endpoint: Url,
}

impl LocalSwarmUploader {
    pub fn new(swarm_web_upload_endpoint: Url) -> Self {
        Self {
            swarm_web_upload_endpoint
        }
    }
}

#[async_trait]
impl TemplateBinaryUploader for LocalSwarmUploader {
    async fn upload(&self, binary: &Path) -> crate::uploader::Result<Url> {
        let form = Form::new().file("template", binary.canonicalize()?).await?;
        let client = Client::new();
        let resp = client
            .post(self.swarm_web_upload_endpoint.as_str())
            .multipart(form)
            .send()
            .await?
            .json::<UploadResponse>()
            .await?;

        if !resp.success {
            return Err(Error::UploadFailed(resp.error));
        }

        if resp.template_url.is_none() {
            return Err(Error::UploadFailed("Missing template URL in response!".to_string()));
        }

        Ok(resp.template_url.unwrap())
    }
}
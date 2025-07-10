// Copyright 2024 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use std::path::PathBuf;

use convert_case::{Case, Casing};
use thiserror::Error;
use tokio::{fs, io};

use crate::templates::{Template, TemplateFile};

const TEMPLATE_DESCRIPTOR_FILE_NAME: &str = "template.toml";

#[derive(Error, Debug)]
pub enum Error {
    #[error("Git2 error: {0}")]
    IO(#[from] io::Error),
    #[error("Failed to deserialize TOML: {0}")]
    TomlDeserialize(#[from] toml::de::Error),
}

pub type CollectorResult<T> = Result<T, Error>;

pub struct Collector {
    local_folder: PathBuf,
}

impl Collector {
    pub fn new(local_folder: PathBuf) -> Self {
        Self { local_folder }
    }

    /// Collect and return all templates from [`Collector::local_folder`].
    pub async fn collect(&self) -> CollectorResult<Vec<Template>> {
        let mut result = vec![];
        Self::collect_templates(&self.local_folder, &mut result).await?;

        Ok(result)
    }

    /// Collecting recursively all the templates from a starting folder `dir`.
    /// All the results will be pushed into `result`.
    async fn collect_templates(dir: &PathBuf, result: &mut Vec<Template>) -> CollectorResult<()> {
        if dir.is_dir() {
            let mut entries_stream = fs::read_dir(dir).await?;
            while let Some(entry) = entries_stream.next_entry().await? {
                if entry.path().is_dir() {
                    Box::pin(Self::collect_templates(&entry.path(), result)).await?;
                } else if let Some(file_name) = entry.file_name().to_str() {
                    if file_name == TEMPLATE_DESCRIPTOR_FILE_NAME {
                        let toml_content = fs::read_to_string(&entry.path()).await?;
                        let template_file: TemplateFile =
                            toml::from_str(toml_content.as_str()).map_err(Error::TomlDeserialize)?;
                        let template_id = match entry.path().parent() {
                            Some(dir) => {
                                if dir.is_dir() {
                                    if let Some(dir_name) = dir.file_name() {
                                        if let Some(dir_name) = dir_name.to_str() {
                                            dir_name.to_case(Case::Snake)
                                        } else {
                                            template_file.name.to_case(Case::Snake)
                                        }
                                    } else {
                                        template_file.name.to_case(Case::Snake)
                                    }
                                } else {
                                    template_file.name.to_case(Case::Snake)
                                }
                            },
                            None => template_file.name.to_case(Case::Snake),
                        };
                        let path = match entry.path().parent() {
                            Some(curr_path) => curr_path.to_path_buf(),
                            None => entry.path(),
                        };
                        result.push(Template::new(
                            path,
                            template_id,
                            template_file.name,
                            template_file.description,
                            template_file.extra.unwrap_or_default(),
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::Path;
    use tempdir::TempDir;

    struct TemplateToGenerate<'a> {
        name: &'a str,
        description: &'a str,
        extra: Option<HashMap<String, String>>,
    }

    impl<'a> TemplateToGenerate<'a> {
        pub fn new(name: &'a str, description: &'a str, extra: Option<HashMap<String, String>>) -> Self {
            Self {
                name,
                description,
                extra,
            }
        }
    }

    async fn generate_template(dir: &Path, template: &TemplateToGenerate<'_>) -> PathBuf {
        let template_dir = dir.join(template.name);
        fs::create_dir_all(template_dir.clone()).await.unwrap();
        let extra_str = template
            .extra
            .as_ref()
            .map(|extra| {
                let values = extra.iter().fold(String::new(), |mut value, (k, v)| {
                    value.push_str(format!("{k} = \"{v}\"\n").as_str());
                    value
                });
                format!(
                    r#"
            [extra]
            {values}
            "#,
                )
            })
            .unwrap_or_default();
        let template_toml = format!(
            r#"
        name = "{}"
        description = "{}"
        
        {}
        "#,
            template.name, template.description, extra_str
        );
        fs::write(template_dir.join(TEMPLATE_DESCRIPTOR_FILE_NAME), template_toml)
            .await
            .unwrap();
        template_dir
    }

    #[tokio::test]
    async fn test_collect() {
        let temp_dir = TempDir::new("tari_cli_test_collect_templates").unwrap();
        let temp_dir_path = temp_dir.path().to_path_buf();
        let templates_to_generate = vec![
            TemplateToGenerate::new("template1", "description1", None),
            TemplateToGenerate::new("template2", "description2", None),
            TemplateToGenerate::new(
                "template3",
                "description3",
                Some(HashMap::from([("templates_dir".to_string(), "templates".to_string())])),
            ),
        ];
        for template in &templates_to_generate {
            generate_template(&temp_dir_path, template).await;
        }

        let collector = Collector::new(temp_dir_path);
        let result = collector.collect().await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.len(), templates_to_generate.len());

        // assert all templates existence
        for template in &templates_to_generate {
            match &template.extra {
                Some(extra) => {
                    assert!(result.iter().any(|curr_template| {
                        curr_template.name() == template.name
                            && curr_template.description() == template.description
                            && curr_template.extra().eq(extra)
                    }));
                },
                None => {
                    assert!(result.iter().any(|curr_template| {
                        curr_template.name() == template.name && curr_template.description() == template.description
                    }));
                },
            }
        }
    }
}

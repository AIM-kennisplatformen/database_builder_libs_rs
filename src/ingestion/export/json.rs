use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use serde::Serialize;
use thiserror::Error;

use crate::models::{domain::Paper, paths::tei_xml::TeiXmlPath};

#[derive(Debug, Error)]
pub enum JsonExportError {
    #[error("failed to create JSON output directory at {path}: {source}")]
    CreateOutputDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to create JSON output file at {path}: {source}")]
    CreateFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to serialize domain model as JSON: {0}")]
    Serialize(#[from] serde_json::Error),
}

pub fn domain_model_to_json<T>(model: &T) -> Result<String, JsonExportError>
where
    T: Serialize + ?Sized,
{
    Ok(serde_json::to_string_pretty(model)?)
}

pub fn write_domain_model_json<T>(model: &T, path: impl AsRef<Path>) -> Result<(), JsonExportError>
where
    T: Serialize + ?Sized,
{
    let path = path.as_ref();

    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|source| JsonExportError::CreateOutputDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let file = File::create(path).map_err(|source| JsonExportError::CreateFile {
        path: path.to_path_buf(),
        source,
    })?;

    write_domain_model_json_to_writer(model, file)
}

pub fn write_domain_model_json_to_writer<T, W>(model: &T, writer: W) -> Result<(), JsonExportError>
where
    T: Serialize + ?Sized,
    W: Write,
{
    Ok(serde_json::to_writer_pretty(writer, model)?)
}

pub fn write_paper_json(paper: &Paper, path: impl AsRef<Path>) -> Result<(), JsonExportError> {
    write_domain_model_json(paper, path)
}

pub fn json_path_for_tei_xml(path: &TeiXmlPath, output_dir: impl AsRef<Path>) -> PathBuf {
    let mut file_name = path
        .as_path()
        .file_name()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("paper.tei.xml"));
    file_name.set_extension("json");

    output_dir.as_ref().join(file_name)
}

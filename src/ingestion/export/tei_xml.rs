use std::{fs, path::PathBuf};

use thiserror::Error;

use crate::models::paths::tei_xml::TeiXmlPath;

#[derive(Debug, Error)]
pub enum TeiXmlExportError {
    #[error("failed to create TEI XML output directory at {path}")]
    CreateOutputDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to write TEI XML file at {path}")]
    WriteFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

pub fn write_tei_xml(path: &TeiXmlPath, tei_xml: &str) -> Result<(), TeiXmlExportError> {
    if let Some(parent) = path
        .as_path()
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|source| TeiXmlExportError::CreateOutputDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    fs::write(path.as_path(), tei_xml).map_err(|source| TeiXmlExportError::WriteFile {
        path: path.as_path().to_path_buf(),
        source,
    })
}

use crate::models::paths::tei_xml::TeiXmlPath;
use std::{fs, io::Error};

pub fn write_tei_xml(path: &TeiXmlPath, tei_xml: &str) -> Result<(), Error> {
    if let Some(parent) = path
        .as_path()
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|error| {
            eprintln!(
                "Failed to create TEI XML output directory at {}: {error}",
                parent.display()
            );
            Error::other(error)
        })?;
    }

    fs::write(path.as_path(), tei_xml).map_err(|error| {
        eprintln!("Failed to write TEI XML to {}: {error}", path.display());
        Error::other(error)
    })
}

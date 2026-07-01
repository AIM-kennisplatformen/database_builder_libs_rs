use derive_more::{AsRef, Deref};
use std::{ffi::OsStr, path::PathBuf};
use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq, Deref, AsRef)]
pub struct PdfPath(PathBuf);

#[derive(Debug, Clone, Eq, PartialEq, Error)]
#[error("Path is not a PDF file: {}", .0.display())]
pub struct PdfPathError(pub PathBuf);

impl TryFrom<PathBuf> for PdfPath {
    type Error = PdfPathError;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"))
        {
            Ok(PdfPath(path))
        } else {
            Err(PdfPathError(path))
        }
    }
}

impl PdfPath {
    pub fn file_stem(&self) -> Option<&OsStr> {
        self.0.file_stem()
    }
}

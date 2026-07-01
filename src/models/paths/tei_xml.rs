use derive_more::{AsRef, Deref};
use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq, Deref, AsRef)]
pub struct TeiXmlPath(PathBuf);

#[derive(Debug, Clone, Eq, PartialEq, Error)]
#[error("Path is not a .tei.xml file: {}", .0.display())]
pub struct TeiXmlPathError(pub PathBuf);

impl TryFrom<PathBuf> for TeiXmlPath {
    type Error = TeiXmlPathError;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let has_xml = path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("xml"));

        let has_tei = path
            .file_stem()
            .map(Path::new)
            .and_then(|p| p.extension())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("tei"));

        if has_xml && has_tei {
            Ok(TeiXmlPath(path))
        } else {
            Err(TeiXmlPathError(path))
        }
    }
}

impl TeiXmlPath {
    pub fn filename_from_stem(file_name: impl AsRef<OsStr>, output_dir: impl AsRef<Path>) -> Self {
        let mut new_filename = OsString::from(file_name.as_ref());
        new_filename.push(".tei.xml");

        Self(output_dir.as_ref().join(PathBuf::from(new_filename)))
    }
}

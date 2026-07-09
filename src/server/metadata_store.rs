use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::ingestion::metadata::UploadInterfaceMetadata;

/// Disk-backed, content-addressed store for extracted upload_interface
/// Field-schema metadata, keyed by the same sha256 source hash used
/// throughout the rest of this pipeline. Lets a previously-extracted
/// result be re-fetched (GET) without re-running GROBID.
#[derive(Clone, Debug)]
pub struct MetadataStore {
    directory: PathBuf,
}

impl MetadataStore {
    pub fn new(directory: impl Into<PathBuf>) -> Result<Self> {
        let directory = directory.into();
        std::fs::create_dir_all(&directory)
            .with_context(|| format!("creating metadata directory `{}`", directory.display()))?;

        Ok(Self { directory })
    }

    fn path(&self, sha256: &str) -> PathBuf {
        self.directory.join(format!("{sha256}.json"))
    }

    pub fn save(&self, sha256: &str, metadata: &UploadInterfaceMetadata) -> Result<()> {
        let path = self.path(sha256);
        let json = serde_json::to_vec_pretty(metadata).context("serializing metadata")?;

        std::fs::write(&path, json)
            .with_context(|| format!("writing metadata to `{}`", path.display()))
    }

    pub fn load(&self, sha256: &str) -> Result<Option<UploadInterfaceMetadata>> {
        let path = self.path(sha256);

        if !path.is_file() {
            return Ok(None);
        }

        let json = std::fs::read(&path)
            .with_context(|| format!("reading metadata from `{}`", path.display()))?;

        serde_json::from_slice(&json)
            .with_context(|| format!("deserializing metadata from `{}`", path.display()))
            .map(Some)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::*;

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_dir() -> PathBuf {
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "scepa-rs-metadata-test-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn round_trips_saved_metadata() {
        let store = MetadataStore::new(temp_dir()).unwrap();
        let metadata = UploadInterfaceMetadata {
            title: Some("A Paper".to_owned()),
            ..UploadInterfaceMetadata::default()
        };

        store.save("abc123", &metadata).unwrap();

        assert_eq!(store.load("abc123").unwrap(), Some(metadata));
    }

    #[test]
    fn load_returns_none_for_a_missing_entry() {
        let store = MetadataStore::new(temp_dir()).unwrap();

        assert_eq!(store.load("does-not-exist").unwrap(), None);
    }
}

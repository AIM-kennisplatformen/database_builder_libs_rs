use std::path::{Path, PathBuf};

use async_trait::async_trait;
use rootcause::prelude::{Report, ResultExt};

#[async_trait]
pub trait PdfStorage: Send + Sync {
    async fn store_pdf(&self, pdf_file: &Path, pdf_hash: &str) -> Result<PathBuf, Report>;
}

#[derive(Clone)]
pub struct LocalPdfStorage {
    directory: PathBuf,
}

impl LocalPdfStorage {
    pub fn new(directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: directory.into(),
        }
    }
}

#[async_trait]
impl PdfStorage for LocalPdfStorage {
    async fn store_pdf(&self, pdf_file: &Path, pdf_hash: &str) -> Result<PathBuf, Report> {
        let artifact_path = self.directory.join(format!("{pdf_hash}.pdf"));

        tokio::fs::create_dir_all(&self.directory)
            .await
            .context(format!(
                "failed to create parsed PDF directory `{}`",
                self.directory.display()
            ))?;
        if is_existing_file(&artifact_path).await? {
            tracing::warn!(
                hash = %pdf_hash,
                artifact = %artifact_path.display(),
                "PDF artifact with the same hash already exists; overwriting it"
            );
        }
        tokio::fs::copy(pdf_file, &artifact_path)
            .await
            .context(format!(
                "failed to copy PDF `{}` to `{}`",
                pdf_file.display(),
                artifact_path.display()
            ))?;
        tracing::debug!(artifact = %artifact_path.display(), "saved PDF artifact");
        Ok(artifact_path)
    }
}

async fn is_existing_file(path: &Path) -> Result<bool, Report> {
    match tokio::fs::metadata(path).await {
        Ok(metadata) => Ok(metadata.is_file()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error)
            .context(format!(
                "failed to inspect parsed PDF artifact `{}`",
                path.display()
            ))
            .map_err(Into::into),
    }
}

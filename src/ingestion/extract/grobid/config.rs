use std::path::PathBuf;

#[derive(Debug)]
pub struct GrobidConfig {
    pub url: String,
    pub output_dir: PathBuf,
}

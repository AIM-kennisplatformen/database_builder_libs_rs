use std::{fs, path::Path};

use rootcause::{
    option_ext::OptionExt,
    prelude::{Report, ResultExt},
};
use tracing::Instrument;

pub mod error;
pub mod source;

use crate::{Config, pipeline::source::grobid::GrobidClient, progress::Progress};

pub const TOTAL_STEPS: usize = 2;
const DEBUG_ARTIFACTS_DIR: &str = "log/tei";

#[derive(Clone)]
pub struct PipelineSources {
    pub grobid: GrobidClient,
}

pub async fn run(
    config: &Config,
    pdf_file: &Path,
    progress: &Progress,
    worker_id: usize,
    sources: PipelineSources,
) -> Result<(), Report> {
    let span = tracing::info_span!(
        "pipeline",
        pdf = %pdf_file.display(),
        worker_id,
    );

    let result = async {
        progress.start_file(worker_id, pdf_file, TOTAL_STEPS, "extracting from Grobid");

        let tei_xml = sources
            .grobid
            .extract_pdf_to_tei_xml_with_retry(pdf_file)
            .await?;
        progress.step(worker_id, 1, Some("extracted TEI XML".to_owned()));

        if config.save_debug_artifacts {
            save_debug_artifact(pdf_file, ".tei.xml", &tei_xml)?;
            progress.step(worker_id, 2, Some("saved TEI XML".to_owned()));
        } else {
            progress.step(worker_id, 2, None);
        }

        Ok(())
    }
    .instrument(span)
    .await;

    progress.finish_file(worker_id);
    result
}

fn save_debug_artifact(pdf_file: &Path, suffix: &str, content: &str) -> Result<(), Report> {
    let mut artifact_name = pdf_file
        .file_stem()
        .context(format!(
            "failed to read file stem for {}",
            pdf_file.display()
        ))?
        .to_os_string();
    artifact_name.push(suffix);
    let artifact_path = Path::new(DEBUG_ARTIFACTS_DIR).join(artifact_name);

    fs::create_dir_all(DEBUG_ARTIFACTS_DIR).context(format!(
        "failed to create debug artifact directory `{DEBUG_ARTIFACTS_DIR}`"
    ))?;
    fs::write(&artifact_path, content.as_bytes()).context(format!(
        "failed to write debug artifact `{}`",
        artifact_path.display()
    ))?;
    Ok(())
}

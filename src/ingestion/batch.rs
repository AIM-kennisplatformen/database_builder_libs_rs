use std::{
    collections::VecDeque,
    env, fmt, fs,
    io::{self, Error, ErrorKind, Write},
    path::{Path, PathBuf},
};

use tokio::task::JoinSet;

use crate::{ingestion::pipeline, models::paths::pdf::PdfPath};

const PARALLELISM: usize = 10;
const PROGRESS_BAR_WIDTH: usize = 24;

pub struct RunSummary {
    pub total: usize,
    pub failure_count: usize,
}

impl RunSummary {
    pub fn has_failures(&self) -> bool {
        self.failure_count > 0
    }
}

pub async fn run_source(
    pdf_source: PathBuf,
    tei_xml_dir: PathBuf,
    json_dir: PathBuf,
) -> Result<RunSummary, Error> {
    let pdf_paths = pdf_paths_from_source(pdf_source)?;

    let total = pdf_paths.len();
    let mut pending = VecDeque::from(pdf_paths);
    let mut tasks = JoinSet::new();
    let mut progress = ProgressBar::new(total, PARALLELISM.min(total));
    let mut failure_count = 0;

    spawn_until_full(
        &mut tasks,
        &mut pending,
        &tei_xml_dir,
        &json_dir,
        &mut progress,
    );

    while let Some(result) = tasks.join_next().await {
        progress.finish_document();

        match result {
            Ok(document) => {
                if document.failed {
                    failure_count += 1;
                    progress.errors(&document.path, &document.errors);
                } else {
                    progress.render();
                }
            }
            Err(error) => {
                failure_count += 1;
                progress.worker_error(&error.to_string());
            }
        }

        spawn_until_full(
            &mut tasks,
            &mut pending,
            &tei_xml_dir,
            &json_dir,
            &mut progress,
        );
    }

    progress.finish(failure_count);

    Ok(RunSummary {
        total,
        failure_count,
    })
}

fn pdf_paths_from_source(pdf_source: PathBuf) -> Result<Vec<PdfPath>, Error> {
    if pdf_source.is_file() {
        return Ok(vec![
            PdfPath::try_from(pdf_source)
                .map_err(|error| Error::new(ErrorKind::InvalidInput, error))?,
        ]);
    };

    let pdf_dir = &pdf_source;

    let entries = fs::read_dir(pdf_dir).map_err(|source| {
        Error::new(
            source.kind(),
            format!(
                "failed to read PDF directory {}: {source}",
                pdf_dir.display()
            ),
        )
    })?;

    let mut pdf_paths = Vec::new();

    for entry in entries {
        let path = entry
            .map_err(|source| {
                Error::new(
                    source.kind(),
                    format!("failed to read entry in {}: {source}", pdf_dir.display()),
                )
            })?
            .path();

        if path.is_file()
            && let Ok(pdf_path) = PdfPath::try_from(path)
        {
            pdf_paths.push(pdf_path);
        }
    }

    pdf_paths.sort_by(|left, right| left.as_path().cmp(right.as_path()));

    if pdf_paths.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("PDF directory contains no PDF files: {}", pdf_dir.display()),
        ));
    }

    Ok(pdf_paths)
}

fn spawn_until_full(
    tasks: &mut JoinSet<DocumentResult>,
    pending: &mut VecDeque<PdfPath>,
    tei_xml_dir: &Path,
    json_dir: &Path,
    progress: &mut ProgressBar,
) {
    while tasks.len() < PARALLELISM {
        let Some(pdf_path) = pending.pop_front() else {
            break;
        };

        progress.start_document();
        tasks.spawn(run_pipeline_task(
            pdf_path,
            tei_xml_dir.to_path_buf(),
            json_dir.to_path_buf(),
        ));
    }
}

async fn run_pipeline_task(
    pdf_path: PdfPath,
    tei_xml_dir: PathBuf,
    json_dir: PathBuf,
) -> DocumentResult {
    let document_path = pdf_path.as_path().to_path_buf();
    let mut errors = Vec::new();
    let result = pipeline::run_with_reporter(pdf_path, tei_xml_dir, json_dir, |message| {
        if message.starts_with("Failed") || message.starts_with("failed") {
            errors.push(message.split_whitespace().collect::<Vec<_>>().join(" "));
        }
    })
    .await;

    let failed = match result {
        Ok(()) => false,
        Err(error) => {
            if errors.is_empty() {
                errors.push(
                    error
                        .to_string()
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" "),
                );
            }
            true
        }
    };

    DocumentResult {
        path: document_path,
        failed,
        errors,
    }
}

struct DocumentResult {
    path: PathBuf,
    failed: bool,
    errors: Vec<String>,
}

struct ProgressBar {
    total: usize,
    completed: usize,
    running: usize,
    max_parallelism: usize,
}

impl ProgressBar {
    fn new(total: usize, max_parallelism: usize) -> Self {
        Self {
            total,
            completed: 0,
            running: 0,
            max_parallelism,
        }
    }

    fn start_document(&mut self) {
        self.running += 1;
        self.render();
    }

    fn errors(&mut self, path: &Path, errors: &[String]) {
        clear_progress_line();
        for message in errors {
            eprintln!("Error in {}: {message}", path.display());
        }
        self.render();
    }

    fn worker_error(&mut self, message: &str) {
        clear_progress_line();
        eprintln!(
            "Worker failed: {}",
            message.split_whitespace().collect::<Vec<_>>().join(" ")
        );
        self.render();
    }

    fn finish_document(&mut self) {
        self.completed += 1;
        self.running = self.running.saturating_sub(1);
    }

    fn finish(&mut self, failure_count: usize) {
        clear_progress_line();

        let succeeded = self.total - failure_count;
        eprintln!(
            "Processed {}: {} succeeded, {} failed",
            PdfCount(self.total),
            succeeded,
            failure_count
        );
    }

    fn render(&self) {
        let filled = (self.completed * PROGRESS_BAR_WIDTH)
            .checked_div(self.total)
            .unwrap_or(PROGRESS_BAR_WIDTH);
        let bar = format!(
            "{}{}",
            "#".repeat(filled),
            "-".repeat(PROGRESS_BAR_WIDTH - filled)
        );
        let mut line = format!(
            "[{bar}] {}/{} | running {}/{}",
            self.completed, self.total, self.running, self.max_parallelism
        );
        let width = env::var("COLUMNS")
            .ok()
            .and_then(|columns| columns.parse::<usize>().ok())
            .filter(|columns| *columns >= 40)
            .unwrap_or(120);

        if line.chars().count() > width {
            line = line.chars().take(width - 3).collect::<String>() + "...";
        }

        let mut stderr = io::stderr().lock();
        let _ = write!(stderr, "\r\x1b[2K{line}");
        let _ = stderr.flush();
    }
}

struct PdfCount(usize);

impl fmt::Display for PdfCount {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            1 => write!(formatter, "1 PDF"),
            count => write!(formatter, "{count} PDFs"),
        }
    }
}

fn clear_progress_line() {
    let mut stderr = io::stderr().lock();
    let _ = write!(stderr, "\r\x1b[2K");
    let _ = stderr.flush();
}

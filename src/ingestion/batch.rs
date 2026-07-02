use std::{
    collections::VecDeque,
    env, fmt, fs,
    io::{self, Error, ErrorKind, Write},
    path::{Path, PathBuf},
};

use tokio::task::JoinSet;

use crate::{ingestion::pipeline, models::paths::pdf::PdfPath};

const PARALLELISM: usize = 10;
const PROGRESS_WIDTH: usize = 24;

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
    Ok(run_paths(pdf_paths, tei_xml_dir, json_dir).await)
}

pub async fn run_paths(
    pdf_paths: Vec<PdfPath>,
    tei_xml_dir: PathBuf,
    json_dir: PathBuf,
) -> RunSummary {
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

    RunSummary {
        total,
        failure_count,
    }
}

fn pdf_paths_from_source(pdf_source: PathBuf) -> Result<Vec<PdfPath>, Error> {
    if pdf_source.is_dir() {
        pdf_paths_in_dir(&pdf_source)
    } else {
        Ok(vec![PdfPath::try_from(pdf_source).map_err(|error| {
            Error::new(ErrorKind::InvalidInput, error)
        })?])
    }
}

fn pdf_paths_in_dir(pdf_dir: &Path) -> Result<Vec<PdfPath>, Error> {
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
        if is_error_message(message) {
            errors.push(single_line(message));
        }
    })
    .await;

    let failed = match result {
        Ok(()) => false,
        Err(error) => {
            if errors.is_empty() {
                errors.push(single_line(&error.to_string()));
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
        eprintln!("Worker failed: {}", single_line(message));
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
        let bar = progress_bar(self.completed, self.total);
        let line = format!(
            "[{bar}] {}/{} | running {}/{}",
            self.completed, self.total, self.running, self.max_parallelism
        );
        write_progress_line(&line);
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

fn progress_bar(completed: usize, total: usize) -> String {
    let filled = (completed * PROGRESS_WIDTH)
        .checked_div(total)
        .unwrap_or(PROGRESS_WIDTH);
    format!(
        "{}{}",
        "#".repeat(filled),
        "-".repeat(PROGRESS_WIDTH - filled)
    )
}

fn single_line(message: &str) -> String {
    message.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_error_message(message: &str) -> bool {
    message.starts_with("Failed") || message.starts_with("failed")
}

fn write_progress_line(line: &str) {
    let line = truncate_to_terminal_width(line);
    let mut stderr = io::stderr().lock();
    let _ = write!(stderr, "\r\x1b[2K{line}");
    let _ = stderr.flush();
}

fn clear_progress_line() {
    let mut stderr = io::stderr().lock();
    let _ = write!(stderr, "\r\x1b[2K");
    let _ = stderr.flush();
}

fn truncate_to_terminal_width(line: &str) -> String {
    let width = env::var("COLUMNS")
        .ok()
        .and_then(|columns| columns.parse::<usize>().ok())
        .filter(|columns| *columns >= 40)
        .unwrap_or(120);
    let line_len = line.chars().count();

    if line_len <= width {
        return line.to_owned();
    }

    line.chars().take(width - 3).collect::<String>() + "..."
}

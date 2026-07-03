use std::{
    env, fmt,
    io::{self, Write},
};

const PROGRESS_BAR_WIDTH: usize = 24;

pub struct ProgressBar {
    total: usize,
    completed: usize,
    running: usize,
    max_parallelism: usize,
}

impl ProgressBar {
    pub fn new(total: usize, max_parallelism: usize) -> Self {
        Self {
            total,
            completed: 0,
            running: 0,
            max_parallelism,
        }
    }

    pub fn start_document(&mut self) {
        self.running += 1;
        self.render();
    }

    pub fn error(&mut self, message: &str) {
        clear_progress_line();
        eprintln!("{message}\n");
        self.render();
    }

    pub fn worker_error(&mut self, message: &str) {
        clear_progress_line();
        eprintln!(
            "Worker failed: {}",
            message.split_whitespace().collect::<Vec<_>>().join(" ")
        );
        self.render();
    }

    pub fn finish_document(&mut self) {
        self.completed += 1;
        self.running = self.running.saturating_sub(1);
    }

    pub fn finish(&mut self, failure_count: usize) {
        clear_progress_line();

        let succeeded = self.total - failure_count;
        eprintln!(
            "Processed {}: {} succeeded, {} failed",
            PdfCount(self.total),
            succeeded,
            failure_count
        );
    }

    pub fn render(&self) {
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

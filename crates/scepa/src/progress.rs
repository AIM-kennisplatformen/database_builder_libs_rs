use std::{
    io::{self, IsTerminal, Write},
    path::Path,
};

use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};

#[derive(Clone)]
pub struct Progress {
    _multi: MultiProgress,
    interactive: bool,
    overall: ProgressBar,
    workers: Vec<ProgressBar>,
}

impl Progress {
    pub fn new(total_files: usize, worker_count: usize) -> Self {
        let interactive = io::stdout().is_terminal();
        let draw_target = if interactive {
            ProgressDrawTarget::stdout()
        } else {
            ProgressDrawTarget::hidden()
        };
        let multi = MultiProgress::with_draw_target(draw_target);

        let overall = multi.add(ProgressBar::new(total_files as u64));
        overall.set_style(
            ProgressStyle::with_template(
                "{prefix:.bold} [{bar:40.cyan/blue}] {pos}/{len} files ({eta})",
            )
            .expect("valid overall progress template")
            .progress_chars("##-"),
        );
        overall.set_prefix("Pipeline");

        let workers = (0..worker_count)
            .map(|worker_id| {
                let bar = multi.add(ProgressBar::new(1));
                bar.set_style(
                    ProgressStyle::with_template(
                        "{prefix}: [{bar:30.green/white}] {pos}/{len} steps {msg}",
                    )
                    .expect("valid worker progress template")
                    .progress_chars("##-"),
                );
                bar.set_prefix(worker_id.to_string());
                bar.set_message("idle".to_owned());
                bar
            })
            .collect();

        Self {
            _multi: multi,
            interactive,
            overall,
            workers,
        }
    }

    pub fn log_writer(&self) -> Box<dyn Write + Send> {
        Box::new(ProgressWriter {
            multi: self._multi.clone(),
            interactive: self.interactive,
            buffer: Vec::new(),
        })
    }

    pub fn start_file(
        &self,
        worker_id: usize,
        file_path: &Path,
        total_steps: usize,
        message: impl Into<String>,
    ) {
        let bar = &self.workers[worker_id];
        let file_name = file_path.file_name().map_or_else(
            || file_path.to_string_lossy().into_owned(),
            |name| name.to_string_lossy().into_owned(),
        );
        bar.set_prefix(file_name);
        bar.set_length(total_steps as u64);
        bar.set_position(0);
        bar.set_message(message.into());
        bar.force_draw();
    }

    pub fn step(&self, worker_id: usize, step: usize, message: Option<String>) {
        let bar = &self.workers[worker_id];
        bar.set_position(step as u64);
        if let Some(message) = message {
            bar.set_message(message);
        }
    }

    pub fn finish_file(&self, worker_id: usize) {
        self.overall.inc(1);
        self.workers[worker_id].set_position(0);
        self.workers[worker_id].set_prefix(worker_id.to_string());
        self.workers[worker_id].set_message("idle".to_owned());
    }

    pub fn finish(&self) {
        self.overall.finish();
        for worker in &self.workers {
            worker.finish();
        }
    }
}

struct ProgressWriter {
    multi: MultiProgress,
    interactive: bool,
    buffer: Vec<u8>,
}

impl Write for ProgressWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.buffer.extend_from_slice(bytes);

        // `tracing_subscriber` writes a formatted event in several chunks and does not necessarily flush the writer afterwards.
        // Flush complete log lines here so console logging is visible while progress bars remain in control of terminal redraws.
        if bytes.contains(&b'\n') {
            self.flush()?;
        }

        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        let bytes = std::mem::take(&mut self.buffer);
        if bytes.is_empty() {
            return Ok(());
        }

        let message = String::from_utf8_lossy(&bytes);
        let message = message.trim_end_matches(['\r', '\n']);

        if self.interactive {
            if !message.is_empty() {
                self.multi.println(message)?;
            }
        } else {
            io::stdout().write_all(message.as_bytes())?;
            io::stdout().write_all(b"\n")?;
        }

        if self.interactive {
            Ok(())
        } else {
            io::stdout().flush()
        }
    }
}

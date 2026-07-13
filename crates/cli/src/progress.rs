use std::io::{self, IsTerminal, Write};

use indicatif::{
    MultiProgress, ProgressBar as IndicatifProgressBar, ProgressDrawTarget, ProgressStyle,
};
use scepa_rs::progress::{Progress, ProgressEvent};

#[derive(Clone)]
pub struct ProgressBar {
    _multi: MultiProgress,
    interactive: bool,
    overall: IndicatifProgressBar,
    workers: Vec<IndicatifProgressBar>,
}

impl ProgressBar {
    pub fn new(total_files: usize, worker_count: usize) -> Self {
        let interactive = io::stdout().is_terminal();
        let draw_target = if interactive {
            ProgressDrawTarget::stdout()
        } else {
            ProgressDrawTarget::hidden()
        };
        let multi = MultiProgress::with_draw_target(draw_target);

        let overall = multi.add(IndicatifProgressBar::new(total_files as u64));
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
                let bar = multi.add(IndicatifProgressBar::new(1));
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

    pub fn finish(&self) {
        self.overall.finish();
        for worker in &self.workers {
            worker.finish();
        }
    }

    fn report_event(&self, event: ProgressEvent) {
        match event {
            ProgressEvent::FileStarted {
                worker_id,
                file_path,
                total_steps,
                message,
            } => {
                let bar = &self.workers[worker_id];
                let file_name = std::path::Path::new(&file_path).file_name().map_or_else(
                    || file_path.clone(),
                    |name| name.to_string_lossy().into_owned(),
                );
                bar.set_prefix(file_name);
                bar.set_length(total_steps as u64);
                bar.set_position(0);
                bar.set_message(message);
                bar.force_draw();
            }
            ProgressEvent::Step {
                worker_id,
                step,
                message,
            } => {
                let bar = &self.workers[worker_id];
                bar.set_position(step as u64);
                if let Some(message) = message {
                    bar.set_message(message);
                }
            }
            ProgressEvent::FileFinished { worker_id } => {
                self.overall.inc(1);
                let bar = &self.workers[worker_id];
                bar.set_position(0);
                bar.set_prefix(worker_id.to_string());
                bar.set_message("idle".to_owned());
            }
        }
    }
}

impl Progress for ProgressBar {
    fn report(&self, event: ProgressEvent) {
        self.report_event(event);
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

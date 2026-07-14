use rootcause::prelude::Report;

pub mod log;
pub mod models;
pub mod pipeline;
pub mod progress;

#[derive(Debug)]
pub struct Config {
    pub save_debug_artifacts: bool,
    pub worker_count: usize,
    pub grobid_url: String,
}

impl Config {
    pub fn new(
        save_debug_artifacts: bool,
        worker_count: usize,
        grobid_url: String,
    ) -> Result<Self, Report> {
        if worker_count == 0 {
            return Err(rootcause::report!("worker count must be greater than zero"));
        }

        Ok(Self {
            save_debug_artifacts,
            worker_count,
            grobid_url,
        })
    }
}

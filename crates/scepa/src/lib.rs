use clap::Parser;
use rootcause::prelude::Report;

pub mod log;
pub mod pipeline;
pub mod progress;

#[derive(Parser)]
#[command(author, version, about)]
pub struct Env {
    #[arg(long, env = "SAVE_DEBUG_ARTIFACTS", default_value_t = false)]
    save_debug_artifacts: bool,

    #[arg(long, env = "WORKER_COUNT", default_value_t = 4)]
    worker_count: usize,

    #[arg(long, env = "GROBID_URL")]
    grobid_url: String,
}

#[derive(Debug)]
pub struct Config {
    pub save_debug_artifacts: bool,
    pub worker_count: usize,
    pub grobid_url: String,
}

impl TryFrom<Env> for Config {
    fn try_from(env: Env) -> Result<Self, Report> {
        if env.worker_count == 0 {
            return Err(rootcause::report!("WORKER_COUNT must be greater than zero"));
        }

        Ok(Self {
            save_debug_artifacts: env.save_debug_artifacts,
            worker_count: env.worker_count,
            grobid_url: env.grobid_url,
        })
    }

    type Error = Report;
}

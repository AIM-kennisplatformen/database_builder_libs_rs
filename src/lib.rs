use anyhow::Result;
use clap::Parser;

pub mod log;
pub mod progress;

#[derive(Parser)]
#[command(author, version, about)]
pub struct Env {
    #[arg(long, env = "WORKER_COUNT", default_value_t = 4)]
    worker_count: usize,
}

#[derive(Debug)]
pub struct Config {
    pub worker_count: usize,
}

impl TryFrom<Env> for Config {
    fn try_from(env: Env) -> Result<Self> {
        anyhow::ensure!(
            env.worker_count > 0,
            "WORKER_COUNT must be greater than zero"
        );

        Ok(Self {
            worker_count: env.worker_count,
        })
    }

    type Error = anyhow::Error;
}

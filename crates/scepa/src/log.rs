use std::{
    fs::{self, OpenOptions},
    io::Write,
    sync::Mutex,
};

use rootcause::prelude::Report;
use tracing_subscriber::{
    EnvFilter, Layer, filter::LevelFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt,
};

const LOG_DIR: &str = "log";
const LOG_FILE: &str = "log/pipeline.log";
const WORKSPACE_TARGETS: &str = "cli=trace,scepa_rs=trace,scepa_macros=trace";

pub fn clear_log_dir() -> Result<(), Report> {
    match fs::remove_dir_all(LOG_DIR) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

pub fn setup_tracing(console_writer: Box<dyn Write + Send>, rust_log: &str) -> Result<(), Report> {
    fs::create_dir_all(LOG_DIR)?;

    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(LOG_FILE)?;

    let builder = EnvFilter::builder().with_default_directive(LevelFilter::INFO.into());

    let filter = if rust_log
        .split(',')
        .any(|directive| !directive.contains('='))
    {
        builder.parse_lossy(rust_log)
    } else {
        // parse_lossy will use the latest LOG_LEVEL passed in for the scope, allowing for overwriting with higher or lower log levels
        // trailing commas do not result in failure when rust_log is empty
        builder.parse_lossy(format!("info,{rust_log}"))
    };

    // Keep dependency logging at the level requested by RUST_LOG, while
    // retaining every level emitted by this workspace's crates.
    let file_filter = builder.parse_lossy(WORKSPACE_TARGETS);

    let console_layer = fmt::layer()
        .with_target(true)
        .with_file(true)
        .with_line_number(true)
        // ANSI allows for better formatting in the console
        .with_ansi(true)
        .with_writer(Mutex::new(console_writer))
        .with_filter(filter);

    let file_layer = fmt::layer()
        .with_target(true)
        .with_file(true)
        .with_line_number(true)
        // ANSI must be disabled as normal text editors do not support ANSI characters
        .with_ansi(false)
        .with_writer(file)
        .with_filter(file_filter);

    tracing_subscriber::registry()
        // file_layer must be added before console_layer to prevent ANSI characters from leaking
        .with(file_layer)
        .with(console_layer)
        .try_init()?;

    Ok(())
}

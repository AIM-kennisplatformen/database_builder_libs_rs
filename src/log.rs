use std::{fs::OpenOptions, io::Write, sync::Mutex};

use anyhow::Result;
use tracing_subscriber::{
    EnvFilter, Layer, filter::LevelFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt,
};

pub fn setup_tracing(console_writer: Box<dyn Write + Send>) -> Result<()> {
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("log/pipeline.log")?;

    let builder = EnvFilter::builder().with_default_directive(LevelFilter::INFO.into());
    let rust_log = std::env::var(EnvFilter::DEFAULT_ENV).unwrap_or_default();

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
        .with_writer(file);

    tracing_subscriber::registry()
        // file_layer must be added before console_layer to prevent ANSI characters from leaking
        .with(file_layer)
        .with(console_layer)
        .try_init()?;

    Ok(())
}

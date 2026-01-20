// src/logging.rs

use anyhow::{Context, Result};
use directories::ProjectDirs;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Init the logger system
///
/// Return a WorkerGuard，must hold it until the program ends.
/// Once the WorkerGuard is dropped, the background log writing thread will also stop.
pub fn init() -> Result<WorkerGuard> {
    // 1. Determine the log file path
    // Windows: C:\Users\YourName\AppData\Local\zgy\happybird\logs
    // macOS: /Users/YourName/Library/Logs/zgy.happybird
    // Linux: /home/YourName/.cache/happybird/logs
    let project_dirs = ProjectDirs::from("org", "zgy", "happybird")
        .context("Failed to determine project directories")?;

    // if is debug mode, output to project directory's logs
    let log_dir = if cfg!(debug_assertions) {
        std::env::current_dir()?.join("logs")
    } else {
        project_dirs.data_local_dir().join("logs")
    };

    // confirm the directory exists
    if !log_dir.exists() {
        std::fs::create_dir_all(&log_dir).context("Failed to create log directory")?;
    }

    // print tracing log store dir
    println!("Logs will be written to: {:?}", log_dir);

    // 2. config rolling Appender, daily rolling
    let file_appender = tracing_appender::rolling::daily(&log_dir, "happybird.log");

    // 3. config non block
    let (non_blocking_file_writer, guard) = tracing_appender::non_blocking(file_appender);

    // 4. config EnvFilter
    // default INFO，but can modify by system env `RUST_LOG`
    // Example：set `RUST_LOG=happybird=debug` can see happybird debug info
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,happybird=debug"));

    // 5. Stdout Layer
    let stdout_layer = fmt::layer()
        .with_target(true) // mod pwd
        .with_thread_ids(true) // thread id
        .with_file(false)
        .with_line_number(true) // show line num
        .pretty()
        .with_writer(std::io::stdout)
        .with_filter(env_filter.clone());

    // 6. Fsout Layer
    let file_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .with_ansi(true)
        .with_writer(non_blocking_file_writer)
        .with_filter(env_filter);

    // 7. Register all layer
    // use try_init avoid init multi and panic
    if let Err(e) = tracing_subscriber::registry()
        .with(stdout_layer)
        .with(file_layer)
        .try_init()
    {
        eprintln!("⚠️ Tracing subscriber init failed (ignored): {}", e);
    }

    if let Err(e) = tracing_log::LogTracer::init() {
        eprintln!(
            "⚠️ LogTracer init failed: {} (This usually means another logger is already active, ignoring...)",
            e
        );
    }

    Ok(guard)
}

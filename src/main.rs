use std::sync::LazyLock;

use tracing_subscriber::{fmt::format::FmtSpan, prelude::*};

mod ui;

static RUNTIME: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(1)
        .build()
        .unwrap()
});

fn main() -> anyhow::Result<()> {
    let reg = tracing_subscriber::registry();

    let env = tracing_subscriber::EnvFilter::builder().parse(
        ["HAPPYBIRD_LOG"]
            .iter()
            .find_map(|key| std::env::var(key).ok())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "debug,blade_graphics=warn,symphonia=warn,zbus=warn".to_owned()),
    )?;

    reg.with(
        tracing_subscriber::fmt::layer()
            .with_thread_names(true) // nice to have until we replace with tasks
            .with_span_events(FmtSpan::FULL) // there's nothing below debug_span
            .with_timer(tracing_subscriber::fmt::time::uptime()) // date's useless
            .with_filter(env),
    )
    .init();

    tracing::info!("Starting application");

    crate::ui::app::run()
}

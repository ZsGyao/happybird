mod ui;
#[macro_use]
mod zlog;

use std::sync::LazyLock;

use crate::zlog::filter::LEVEL_ENABLED_MAX_DEFAULT;

#[allow(dead_code)]
static RUNTIME: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(1)
        .build()
        .unwrap()
});

fn main() -> anyhow::Result<()> {
    // Init log
    zlog::init();
    zlog::init_output_stdout();

    // App run
    crate::ui::app::run()
}

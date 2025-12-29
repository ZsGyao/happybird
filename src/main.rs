mod ui;
#[macro_use]
mod zlog;

use std::sync::LazyLock;

#[allow(dead_code)]
static RUNTIME: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(1)
        .build()
        .unwrap()
});

fn main() -> anyhow::Result<()> {
    zlog::init();
    zlog::sink::init_output_stdout();

    crate::ui::app::run()
}

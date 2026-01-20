// #![windows_subsystem = "windows"]

use tracing::{error, info};

mod backend;
mod logging;
mod ui;

fn main() -> anyhow::Result<()> {
    let _guard = logging::init()?;
    info!("HappyBird application starting...");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));

    if let Err(e) = crate::ui::app::run() {
        // 使用 tracing 记录崩溃错误
        error!("Application crashed: {:?}", e);
        return Err(e);
    }

    info!("Application stopped gracefully.");
    Ok(())
}

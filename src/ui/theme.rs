use gpui::App;

use crate::ui::theme::extra::AppThemeExtra;

pub mod extra;
pub mod loader;
pub mod style;

pub fn init(cx: &mut App) {
    // 初始化扩展状态
    cx.set_global(AppThemeExtra::default());

    // AssetSource 定义了 #[folder = "./assets"]，所以这里不需要写 assets 前缀
    let asset_path = "themes/happybird.json";

    let json_bytes = cx
        .asset_source()
        .load(asset_path)
        .expect("AssetSource error")
        .expect("Theme file not found in assets!"); // 如果这里 panic，说明资源没打包进去

    let json = std::str::from_utf8(&json_bytes).expect("Theme JSON is not valid UTF-8");
    // 初始加载不需要 window，传 None
    crate::ui::theme::loader::apply_theme(cx, None, &json, "Happybird Dark").ok();
}

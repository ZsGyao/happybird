use std::sync::Arc;

use gpui::{App, Global};
use tracing::{error, info};

use crate::ui::theme::{
    impls::default::DefaultTheme,
    infra::{extra::AppThemeExtra, loader},
    strategy::ThemeStrategy,
};

/// Global theme manage model
pub struct ThemeModel {
    /// Current active theme strategy
    pub active_strategy: Arc<dyn ThemeStrategy>,
}

/// Assign global
impl Global for ThemeModel {}

impl ThemeModel {
    /// Init and register to the global context
    pub fn init(cx: &mut App) {
        // init ThemeModel
        let model = Self {
            active_strategy: Arc::new(DefaultTheme::new(cx)),
        };
        cx.set_global(model);

        // init AppThemeExtra (数据层 - 必须做，否则 style.rs 里的 AppThemeExtra::global(cx) 会崩溃)
        cx.set_global(AppThemeExtra::default());

        // 3. 加载默认的 JSON 颜色配置 (infra层)
        Self::load_default_json_theme(cx);
    }

    /// Aux fun: Load default JSON theme
    fn load_default_json_theme(cx: &mut App) {
        let asset_path = "themes/happybird.json";

        // try load resource
        let load_result = cx.asset_source().load(asset_path);

        match load_result {
            Ok(Some(json_bytes)) => {
                // try convert to utf8
                if let Ok(json_str) = std::str::from_utf8(&json_bytes) {
                    if let Err(e) = loader::apply_theme(cx, None, json_str, "Happybird Dark") {
                        error!("Theme apply error: {:?}", e);
                    } else {
                        info!("Default theme loaded successfully: {}", asset_path);
                    }
                } else {
                    error!("Theme JSON is not valid UTF-8: {}", asset_path);
                }
            }
            Ok(None) => {
                error!("Theme file not found in assets: {}", asset_path);
            }
            Err(e) => {
                error!("AssetSource error loading theme: {:?}", e);
            }
        }
    }

    pub fn set_theme(&mut self, name: &str, cx: &mut App) {
        match name {
            // "modern" => {
            //     self.active_strategy = Arc::new(ModernTheme);
            //     // 进阶：如果 modern 主题有专门的 JSON (比如 themes/modern.json)
            //     // 你可以在这里调用类似的加载逻辑来覆盖颜色
            // }
            _ => {
                self.active_strategy = Arc::new(DefaultTheme::new(cx));
                // 如果切换回默认，且之前被改过颜色，这里应该重新加载默认 JSON
                // Self::load_default_json_theme(cx);
            }
        }

        // 通知 UI 刷新布局
        cx.refresh_windows();
    }

    /// 便捷切换方法 (用于测试)
    pub fn toggle(&mut self, cx: &mut App) {
        info!("Toggling theme layout...");
        if self.active_strategy.name() == "default" {
            self.set_theme("modern", cx);
        } else {
            self.set_theme("default", cx);
        }
    }
}

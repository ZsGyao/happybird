use std::sync::Arc;

use gpui::{App, Global};

use crate::ui::theme::{infra::loader, strategy::ThemeStrategy};

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
        // 默认加载 DefaultTheme
        let model = Self {
            active_strategy: Arc::new(DefaultTheme),
        };
        cx.set_global(model);

        // 同时加载默认的颜色配置 (JSON)
        loader::load_theme("happybird", cx);
    }

    /// 切换主题策略
    pub fn set_theme(&mut self, name: &str, cx: &mut App) {
        match name {
            "modern" => {
                self.active_strategy = Arc::new(ModernTheme);
                // 如果 modern 主题有对应的 json 颜色文件，这里也可以加载
                // loader::load_theme("modern_dark", cx);
            }
            _ => {
                self.active_strategy = Arc::new(DefaultTheme);
                // 恢复默认颜色
                loader::load_theme("happybird", cx);
            }
        }

        // 通知 UI 刷新
        cx.refresh_windows();
    }

    /// 便捷切换方法 (用于测试)
    pub fn toggle(&mut self, cx: &mut App) {
        if self.active_strategy.name() == "default" {
            self.set_theme("modern", cx);
        } else {
            self.set_theme("default", cx);
        }
    }
}

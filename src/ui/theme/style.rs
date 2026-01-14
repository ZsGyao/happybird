use super::extra::AppThemeExtra;
use gpui::{App, Styled, px};
use gpui_component::ActiveTheme; // 引入库的 trait 以使用 .theme()

/// 主题样式扩展 Trait
///
/// 通过扩展 `Styled` trait，让所有 GPUI 元素都能方便地应用混合主题。
pub trait ThemedStyle: Styled {
    /// 应用应用程序标准的面板样式
    ///
    /// 组合逻辑：
    /// * 背景色 -> 来自 gpui_component (标准主题)
    /// * 边框颜色 -> 来自 gpui_component (标准主题)
    /// * 边框宽度 -> 来自 AppThemeExtra (扩展配置)
    /// * 圆角 -> 来自 AppThemeExtra (扩展配置)
    fn app_panel_style(self, cx: &App) -> Self {
        let theme = cx.theme(); // 获取库的主题 (Colors)
        let extra = AppThemeExtra::global(cx); // 获取扩展主题 (Borders, Radius)

        let style = self
            .bg(theme.colors.secondary)
            .border_color(theme.colors.border)
            .border(px(extra.border_width))
            .rounded(px(extra.radius_panel));

        // 如果配置了特殊字体，应用它
        if let Some(font) = &extra.font_family {
            style.font_family(font.clone())
        } else {
            style
        }
    }

    /// 应用标准的交互元素圆角（如按钮、输入框）
    ///
    /// 这里的策略是：直接使用库的 radius。
    /// 因为我们在 Loader 中已经通过 `overrides` 修改了库的 `radius` 值，
    /// 所以这里直接调用 `theme.radius` 就能拿到正确的值（例如 0.0 或 6.0）。
    fn app_rounded_element(self, cx: &App) -> Self {
        let theme = cx.theme();
        self.rounded(theme.radius)
    }
}

// 为所有 GPUI 元素自动实现此 Trait
impl<E: Styled> ThemedStyle for E {}

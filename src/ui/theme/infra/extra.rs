use gpui::{App, Global};
use serde::Deserialize;

/// 图标风格策略，对应 assets/icons/ 下的不同子目录
#[derive(Debug, Clone, Copy, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum IconThemeStyle {
    #[default]
    Outline, // assets/icons/outline/
    Solid, // assets/icons/solid/
    Pixel, // assets/icons/pixel/
}

/// 应用程序专属的深度定制配置
///
/// 这个结构体存储了 `gpui_component` 标准主题系统中不存在的、
/// 或者我们需要更细粒度控制的设计 Token。
#[derive(Debug, Clone, Deserialize)]
pub struct AppThemeExtra {
    /// 图标集风格
    #[serde(default)]
    pub icon_style: IconThemeStyle,

    /// 全局边框宽度 (px)
    /// gpui_component 只有 radius，没有 border width 的全局配置
    #[serde(default = "default_border_width")]
    pub border_width: f32,

    /// 特殊面板（如侧边栏、浮窗）的圆角 (px)
    #[serde(default = "default_panel_radius")]
    pub radius_panel: f32,

    /// 自定义字体名称 (Optional)
    /// 用于覆盖特定区域的字体，制造“像素风”或“手写风”
    pub font_family: Option<String>,
}

// --- Serde Default Helpers ---

fn default_border_width() -> f32 {
    1.0
}
fn default_panel_radius() -> f32 {
    8.0
}

// --- Global Trait Implementation ---

impl Default for AppThemeExtra {
    fn default() -> Self {
        Self {
            icon_style: IconThemeStyle::Outline,
            border_width: 1.0,
            radius_panel: 8.0,
            font_family: None,
        }
    }
}

// 注册为 Global 状态，这使得任何 View 都能订阅它的变化
impl Global for AppThemeExtra {}

impl AppThemeExtra {
    /// 获取全局扩展配置的便捷方法
    ///
    /// # Panics
    /// 如果在 App 初始化时没有调用 `cx.set_global(AppThemeExtra::default())`，
    /// 此方法可能会 panic。但在生产环境中，建议使用 `try_global` 模式或确保初始化。
    pub fn global(cx: &App) -> &Self {
        cx.global::<Self>()
    }
}

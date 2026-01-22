use gpui::{AnyElement, App, Pixels, Window, px};

/// Theme layout metric
///
/// describe diff theme the num param that may changed
#[derive(Clone, Copy, Debug)]
pub struct ThemeMetrics {
    /// Siderbar width
    pub sidebar_width: Pixels,
    /// Sidebar collapse width
    pub sidebar_collapsed_width: Pixels,
    /// Sidebar background opcaity (0.0 ~ 1.0)
    pub sidebar_bg_opacity: f32,
    /// Top header height
    pub header_height: Pixels,
    /// Context rounding size
    pub content_rounding: Pixels,
    /// macOS window traffic button left padding
    pub titlebar_padding_left: Pixels,
}

impl Default for ThemeMetrics {
    fn default() -> Self {
        Self {
            sidebar_width: px(180.0),
            header_height: px(37.0),
            sidebar_bg_opacity: 1.0,
            content_rounding: px(0.0),
            titlebar_padding_left: px(72.0),
            sidebar_collapsed_width: px(56.0),
        }
    }
}

/// Theme strategy trait
///
/// Each theme(like default, morden ...) need impl this trait
pub trait ThemeStrategy: Send + Sync {
    /// Theme name, unique identify id
    fn name(&self) -> &str;
    /// Get the theme num metrics
    fn metrics(&self) -> ThemeMetrics;
    /// Component factory：Render header
    fn render_header(&self) -> AnyElement;
    /// Component factory：Render Sidebar
    fn render_sidebar(&self) -> AnyElement;

    // add ...
}

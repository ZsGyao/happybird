use gpui::{IntoElement, px};

use crate::ui::{
    header::Header,
    sidebar::HappyBirdSideBar,
    theme::strategy::{ThemeMetrics, ThemeStrategy},
};

pub mod default;

pub struct DefaultTheme;

impl ThemeStrategy for DefaultTheme {
    fn name(&self) -> &str {
        "default"
    }

    fn metrics(&self) -> super::strategy::ThemeMetrics {
        ThemeMetrics {
            sidebar_width: px(180.0),
            sidebar_collapsed_width: px(56.0),
            sidebar_bg_opacity: 1.0, // no opacity
            header_height: px(37.0),
            content_rounding: px(8.0),
            titlebar_padding_left: px(72.0),
        }
    }

    fn render_header(&self, window: &mut gpui::Window, cx: &mut gpui::App) -> gpui::AnyElement {
        Header::new(cx).into_any_element()
    }

    fn render_sidebar(&self, window: &mut gpui::Window, cx: &mut gpui::App) -> gpui::AnyElement {
        HappyBirdSideBar::new(cx).into_any_element()
    }
}

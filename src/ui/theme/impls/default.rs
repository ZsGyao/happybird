use gpui::{AnyElement, App, IntoElement, Window};

use crate::ui::{
    header::Header,
    sidebar::HappyBirdSideBar,
    theme::strategy::{ThemeMetrics, ThemeStrategy},
};

pub struct DefaultTheme;

impl ThemeStrategy for DefaultTheme {
    fn name(&self) -> &str {
        "default"
    }

    fn metrics(&self) -> ThemeMetrics {
        ThemeMetrics::default()
    }

    fn render_header(&self, _window: &mut Window, cx: &mut App) -> AnyElement {
        Header::new(cx).into_any_element()
    }

    fn render_sidebar(&self, _window: &mut Window, cx: &mut App) -> AnyElement {
        HappyBirdSideBar::new(cx).into_any_element()
    }
}

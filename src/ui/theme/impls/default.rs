use gpui::{AnyElement, App, Entity, IntoElement};

use crate::ui::{
    components::layout::{header::default::Header, sidebar::default::HappyBirdSideBar},
    theme::strategy::{ThemeMetrics, ThemeStrategy},
};

pub struct DefaultTheme {
    pub header: Entity<Header>,
    pub sidebar: Entity<HappyBirdSideBar>,
}

impl DefaultTheme {
    pub fn new(cx: &mut App) -> Self {
        let header = Header::new(cx);
        let sidebar = HappyBirdSideBar::new(cx);
        Self { header, sidebar }
    }
}

impl ThemeStrategy for DefaultTheme {
    fn name(&self) -> &str {
        "default"
    }

    fn metrics(&self) -> ThemeMetrics {
        ThemeMetrics::default()
    }

    fn render_header(&self) -> AnyElement {
        self.header.clone().into_any_element()
    }

    fn render_sidebar(&self) -> AnyElement {
        self.sidebar.clone().into_any_element()
    }
}

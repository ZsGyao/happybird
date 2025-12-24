use gpui::{App, AppContext, Entity, ParentElement, Render, Styled, Window, div, px};
use gpui_component::{
    ActiveTheme,
    list::{List, ListState},
};

use crate::ui::info_browser::InfoBrowser;

pub struct WatchList {
    info_browser: Entity<InfoBrowser>,
}

impl WatchList {
    pub fn new(cx: &mut App, window: &mut Window) -> Entity<Self> {
        cx.new(|cx| WatchList {
            info_browser: InfoBrowser::new(cx, window),
        })
    }
}

impl Render for WatchList {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        div()
            .w_1_5()
            .flex()
            .flex_col()
            .border_1()
            .border_color(cx.theme().sidebar_border)
            .bg(cx.theme().background)
            .child(div().h(px(6.0)))
            .child(self.info_browser.clone())
    }
}

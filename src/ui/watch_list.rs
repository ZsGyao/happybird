use gpui::{App, AppContext, Entity, ParentElement, Render, Styled, Window, div};
use gpui_component::{
    ActiveTheme,
    list::{List, ListState},
};

use crate::ui::info_browser::InfoBrowserDelegate;

pub struct WatchList {
    list_state: Entity<ListState<InfoBrowserDelegate>>,
}

impl WatchList {
    pub fn new(cx: &mut App, window: &mut Window) -> Entity<Self> {
        cx.new(|cx| WatchList {
            list_state: InfoBrowserDelegate::new(cx, window),
        })
    }
}

impl Render for WatchList {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        div()
            .w_1_5()
            .border_1()
            .border_color(cx.theme().sidebar_border)
            .child(List::new(&self.list_state.clone()))
    }
}

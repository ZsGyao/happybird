use std::vec;

use gpui::{
    App, AppContext, Context, Entity, ParentElement, Render, Styled, Window, div,
    prelude::FluentBuilder,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, IndexPath, h_flex,
    label::Label,
    list::{List, ListDelegate, ListItem, ListState},
};
use tracing::info;

use crate::ui::folder_browser::FileBrowserDelegate;

pub struct WatchList {}

impl WatchList {
    pub fn new(cx: &mut App) -> Entity<Self> {
        cx.new(|_cx| WatchList {})
    }
}

impl Render for WatchList {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        let state = FileBrowserDelegate::new(cx, window);
        // div().child(List::new(&state))

        List::new(&state)

        // div()
        //     .w_auto()
        //     .h_full()
        //     .border_r_1()
        //     .border_color(cx.theme().sidebar_border)
        //     .child(List::new(&state))
    }
}

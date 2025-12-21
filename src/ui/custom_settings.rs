use gpui::{App, AppContext, Entity, ParentElement, Render, Styled, div, px};
use gpui_component::{Icon, IconName, button::Button};
use tracing::info;

pub struct CustomSettings {}

impl CustomSettings {
    pub fn new(cx: &mut App) -> Entity<Self> {
        cx.new(|_| CustomSettings {})
    }
}

impl Render for CustomSettings {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        div().justify_center().relative().child(
            Button::new("custom-settings")
                .child(div().child(Icon::new(IconName::Settings)))
                .on_click(|_, _, _| info!("Custom Setting click")),
        )
    }
}

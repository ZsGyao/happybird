use gpui::{App, AppContext, Entity, Render, div};

pub struct InfoPanel {}

impl InfoPanel {
    pub fn new(cx: &mut App) -> Entity<Self> {
        cx.new(|_cx| InfoPanel {})
    }
}

impl Render for InfoPanel {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        div()
    }
}

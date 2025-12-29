use gpui::{App, AppContext, Entity, ParentElement, Render, Styled, div, px};
use gpui_component::{
    ActiveTheme, StyledExt, h_flex,
    list::ListItem,
    tree::{TreeItem, TreeState, tree},
};

#[derive(Clone)]
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
        let tree_state = cx.new(|cx| {
            TreeState::new(cx).items(vec![
                TreeItem::new("src", "src")
                    .expanded(false)
                    .child(TreeItem::new("src/lib.rs", "lib.rs"))
                    .child(TreeItem::new("src/main.rs", "main.rs")),
                TreeItem::new("Cargo.toml", "Cargo.toml"),
                TreeItem::new("README.md", "README.md"),
            ])
        });

        div()
            .h_full()
            .w(px(180.0))
            .v_flex()
            .border_1()
            .border_color(cx.theme().border)
            .child(tree(&tree_state, |ix, entry, selected, window, cx| {
                ListItem::new(ix).child(h_flex().gap_2().child(entry.item().label.clone()))
            }))
    }
}

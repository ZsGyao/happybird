use gpui::Render;
use gpui_component::{Side, sidebar::{Sidebar, SidebarGroup}};

struct HappyBirdSidebar {
    /// Current sidebar is collopsed
    is_collopsed: bool,
}

impl Render for HappyBirdSidebar {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        Sidebar::new(Side::Left)
            .collapsed(self.is_collopsed)
            .collapsible(true).child(SidebarGroup::new("info-panel-group").child(child))
    }
}

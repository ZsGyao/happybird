use gpui::*;
use gpui_component::{ActiveTheme, Sizable, StyledExt, avatar::Avatar, h_flex, v_flex};

pub struct CustomAvatar;

impl CustomAvatar {
    pub fn new(cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self {})
    }
}

impl Render for CustomAvatar {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        div().child(
            Avatar::new()
                .name("Shan shan")
                .size(px(35.5))
                .relative()
                .justify_center()
                .items_center()
                .border_1()
                .border_color(cx.theme().foreground)
                .corner_radii(Corners {
                    top_left: px(11.0),
                    top_right: px(11.0),
                    bottom_right: px(11.0),
                    bottom_left: px(11.0),
                }),
        )
    }
}

use gpui::{
    App, AppContext, Context, IntoElement, ParentElement, Render, RenderOnce, Window, div, px,
};
use gpui::{Entity, Styled};
use gpui_component::sidebar::{Sidebar, SidebarGroup, SidebarMenu, SidebarMenuItem};
use gpui_component::{ActiveTheme, Icon, IconName, Side, StyledExt};

use crate::zlog::log_impl::info;

use crate::ui::constants::APP_SIDEBAR_W;
use crate::ui::custom_avatar::CustomAvatar;
use crate::ui::custom_settings::CustomSettings;
use crate::ui::models::Models;

pub struct CustomSidebar {
    // is_open_home: Entity<bool>,
    pub custom_avatar: Entity<CustomAvatar>,
    pub custom_settings: Entity<CustomSettings>,
}

impl CustomSidebar {
    pub fn new(cx: &mut App) -> Entity<Self> {
        let custom_avatar = CustomAvatar::new(cx);
        let custom_settings = CustomSettings::new(cx);

        cx.new(|_| CustomSidebar {
            custom_avatar,
            custom_settings,
        })
    }
}

impl Render for CustomSidebar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let height = window.bounds().size.height - px(180.0);

        div()
            .h_full()
            .child(
                div().v_flex().h(height).child(
                    Sidebar::new(Side::Left)
                        .w(APP_SIDEBAR_W)
                        .child(
                            SidebarGroup::new("").child(
                                SidebarMenu::new().child(
                                    SidebarMenuItem::new("sidebar_home")
                                        .icon(Icon::new(IconName::LayoutDashboard).size(px(21.0)))
                                        .on_click(|_, _, _| {
                                            info!("Sidebar Home click");
                                        }),
                                ),
                            ),
                        )
                        .child(
                            SidebarGroup::new("").child(
                                SidebarMenu::new().children([
                                    SidebarMenuItem::new("store_folder")
                                        .icon(Icon::new(IconName::Folder).size(px(20.0)))
                                        .on_click(|_, _, cx| {
                                            let show_folder =
                                                cx.global::<Models>().show_folder.clone();
                                            show_folder.write(cx, !show_folder.read(cx));
                                            info!("Folder show {}", show_folder.read(cx));
                                        }),
                                    SidebarMenuItem::new("reserve_item")
                                        .icon(Icon::new(IconName::Bell).size(px(20.0)))
                                        .on_click(|_, _, _| info!("Reserve Item click")),
                                ]),
                            ),
                        ),
                ),
            )
            .child(
                div()
                    .v_flex()
                    .h_full()
                    .flex_col()
                    .w(APP_SIDEBAR_W)
                    .border_color(cx.theme().sidebar_border)
                    .border_r_1()
                    .border_t_1()
                    .bg(cx.theme().sidebar)
                    .gap_7()
                    .p_3()
                    .child(self.custom_settings.clone())
                    .child(self.custom_avatar.clone()),
            )
    }
}

#[derive(IntoElement)]
pub struct SidebarSeparator {}

impl RenderOnce for SidebarSeparator {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .w(px(59.0))
            //.my(px(5.0))
            .border_b_2()
            .border_color(cx.theme().sidebar_border)
    }
}

pub fn sidebar_separator() -> SidebarSeparator {
    SidebarSeparator {}
}

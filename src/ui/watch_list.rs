use gpui::{App, AppContext, Entity, ParentElement, px};
use gpui::{Render, Styled, div};
use gpui_component::accordion::Accordion;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::checkbox::Checkbox;
use gpui_component::group_box::{GroupBox, GroupBoxVariants};
use gpui_component::label::Label;
use gpui_component::radio::{Radio, RadioGroup};
use gpui_component::select::Select;
use gpui_component::{h_flex, v_flex};

use crate::ui::constants::APP_SIDEBAR_W;

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
        div().border_l(px(200.0)).child(
            GroupBox::new()
                .title("Email Subscriptions")
                .child(
                    v_flex()
                        .gap_2()
                        .child(Checkbox::new("newsletter").label("Weekly Newsletter"))
                        .child(Checkbox::new("updates").label("Product Updates"))
                        .child(Checkbox::new("security").label("Security Alerts"))
                        .child(Checkbox::new("marketing").label("Marketing Communications")),
                )
                .child(
                    h_flex()
                        .justify_between()
                        .mt_4()
                        .child(
                            Button::new("unsubscribe-all")
                                .link()
                                .label("Unsubscribe All"),
                        )
                        .child(Button::new("save").primary().label("Update Preferences")),
                ),
        )
    }
}

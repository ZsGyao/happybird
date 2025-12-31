use gpui::{prelude::FluentBuilder, *};
use gpui_component::{Icon, IconName, StyledExt};

use crate::ui::{constants::APP_ROUNDING, models::Models, theme::UsrTheme};

pub struct Header;

impl Header {
    pub fn new(cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self {})
    }
}

impl Render for Header {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        let decorations = window.window_decorations();
        let theme = cx.global::<UsrTheme>();

        div()
            .flex()
            .w_full()
            .text_sm()
            .min_h(px(37.0))
            .max_h(px(37.0))
            .bg(theme.background_secondary)
            .text_sm()
            .border_b_1()
            .id("titlebar")
            .border_color(theme.border_color)
            .window_control_area(WindowControlArea::Drag)
            .when(cfg!(not(target_os = "windows")), |this| {
                this.on_mouse_down(MouseButton::Left, move |ev, window, _| {
                    if ev.click_count != 2 {
                        window.start_window_move();
                    }
                })
                .on_click(|ev, window, _| {
                    if ev.click_count() == 2 {
                        window.zoom_window();
                    }
                })
            })
            .map(|div| match decorations {
                Decorations::Server => div,
                Decorations::Client { tiling } => div
                    .when(!(tiling.top || tiling.left), |div| {
                        div.rounded_tl(APP_ROUNDING)
                    })
                    .when(!(tiling.top || tiling.right), |div| {
                        div.rounded_tr(APP_ROUNDING)
                    }),
            })
            .when(cfg!(target_os = "macos"), |this| {
                this.child(div().w(px(72.0)))
            })
            .child(div().pl(px(12.0)).pb(px(8.0)).pt(px(7.0)).flex().when(
                cfg!(not(target_os = "macos")),
                |this| {
                    this.child(
                        div()
                            .id("happybird-name")
                            .cursor_pointer()
                            .h_flex()
                            .on_mouse_down(MouseButton::Left, |_, window, cx| {
                                window.prevent_default();
                                cx.stop_propagation();
                            })
                            .on_click(|_, _, cx| {
                                let show_about = cx.global::<Models>().show_about.clone();
                                show_about.write(cx, true);
                            })
                            .child(
                                img("images/happybird_logo_sm.png")
                                    .w(px(26.0))
                                    .mr(px(6.0))
                                    .corner_radii(Corners::all(px(8.0))),
                            )
                            .font_bold()
                            .child("Happybird")
                            .mr(px(6.0)),
                    )
                },
            ))
            .child(div().ml_auto())
            .when(cfg!(not(target_os = "macos")), |this| {
                this.child(
                    div()
                        .flex()
                        .child(WindowButton::Minimize)
                        .child(WindowButton::Maximize)
                        .child(WindowButton::Close),
                )
            })
    }
}

#[derive(PartialEq, Clone, Copy, IntoElement)]
pub enum WindowButton {
    Close,
    Minimize,
    Maximize,
}

impl RenderOnce for WindowButton {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.global::<UsrTheme>();

        let (bg, hover, active) = if self == WindowButton::Close {
            (
                theme.close_button,
                theme.close_button_hover,
                theme.close_button_active,
            )
        } else {
            (
                theme.window_button,
                theme.window_button_hover,
                theme.window_button_active,
            )
        };

        div()
            .flex()
            .w(px(36.0))
            .h(px(37.0))
            .pb(px(1.0))
            .items_center()
            .justify_center()
            .cursor_pointer()
            .id(match self {
                WindowButton::Close => "close",
                WindowButton::Minimize => "minimize",
                WindowButton::Maximize => "maximize",
            })
            .bg(bg)
            .hover(|this| this.bg(hover))
            .active(|this| this.bg(active))
            .window_control_area(match self {
                WindowButton::Close => WindowControlArea::Close,
                WindowButton::Minimize => WindowControlArea::Min,
                WindowButton::Maximize => WindowControlArea::Max,
            })
            .text_size(px(11.0))
            .on_mouse_down(MouseButton::Left, |_, window, cx| {
                cx.stop_propagation();
                window.prevent_default();
            })
            .child(match self {
                WindowButton::Close => Icon::new(IconName::WindowClose).size(px(14.0)),
                WindowButton::Minimize => Icon::new(IconName::WindowMinimize).size(px(14.0)),
                WindowButton::Maximize => Icon::new(IconName::WindowMaximize).size(px(14.0)),
            })
            .when(self == WindowButton::Close, |this| this.rounded_tr(px(4.0)))
            .on_click(move |_, window, cx| match self {
                WindowButton::Close => cx.quit(),
                WindowButton::Minimize => window.minimize_window(),
                WindowButton::Maximize => window.zoom_window(),
            })
    }
}

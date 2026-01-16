use gpui::{prelude::FluentBuilder, *};
use gpui_component::{ActiveTheme, Icon, IconName, StyledExt};

use crate::ui::{constants::APP_ROUNDING, models::GlobalAppState};

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

        div()
            .id("titlebar")
            .flex()
            .w_full()
            .text_sm()
            .min_h(px(37.0))
            .max_h(px(37.0))
            .bg(cx.theme().colors.background)
            .text_sm()
            .border_b_1()
            .border_color(cx.theme().colors.border)
            .window_control_area(WindowControlArea::Drag)
            // Windows 上的双击最大化和拖动逻辑
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
            // 圆角处理
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
            // macOS 的红绿灯占位
            .when(cfg!(target_os = "macos"), |this| {
                this.child(div().w(px(72.0)))
            })
            // --- Header 左侧内容区域 ---
            .child(div().pl(px(12.0)).pb(px(8.0)).pt(px(7.0)).flex().when(
                cfg!(not(target_os = "macos")),
                |this| {
                    this.child(
                        div()
                            .id("header-left-content")
                            .h_flex()
                            .items_center() // 垂直居中
                            .gap(px(8.0))
                            // =========================================================
                            // [新增] SideBar 切换按钮 (Hamburger Menu)
                            // =========================================================
                            .child(
                                div()
                                    .id("toggle-sidebar-btn")
                                    .cursor_pointer()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .p(px(4.0)) // 增加点击热区
                                    .rounded_md()
                                    .hover(|s| s.bg(cx.theme().colors.info_hover)) // 悬停效果
                                    // 防止事件冒泡触发窗口拖动
                                    .on_mouse_down(MouseButton::Left, |_, window, cx| {
                                        window.prevent_default();
                                        cx.stop_propagation();
                                    })
                                    // 点击切换 SideBar 状态
                                    .on_click(move |_, _, cx| {
                                        let global = cx.global::<GlobalAppState>().0.clone();
                                        global.update(cx, |model, cx| {
                                            model.toggle_sidebar(cx);
                                        });
                                    })
                                    .child(
                                        Icon::new(IconName::Menu) // 使用菜单图标
                                            .size(px(16.0))
                                            .text_color(cx.theme().colors.muted_foreground),
                                    ),
                            )
                            // =========================================================
                            // [修改] 应用 Logo 和名称
                            // =========================================================
                            // 将 Logo 和名称包裹在一个 div 中，作为整体处理
                            .child(
                                div()
                                    .id("happybird-name")
                                    .cursor_pointer()
                                    .h_flex()
                                    .on_mouse_down(MouseButton::Left, |_, window, cx| {
                                        window.prevent_default();
                                        cx.stop_propagation();
                                    })
                                    .on_click(|_, _, cx| {
                                        cx.global::<GlobalAppState>().0.clone().update(
                                            cx,
                                            |val, _| {
                                                val.show_about = !val.show_about;
                                            },
                                        );
                                    })
                                    .child(
                                        img("images/happybird_logo_sm.png")
                                            .w(px(26.0))
                                            .mr(px(6.0))
                                            .corner_radii(Corners::all(px(8.0))),
                                    )
                                    .font_bold()
                                    .child("HappyBird")
                                    .mr(px(6.0)),
                            ),
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
        let (bg, hover, active) = (
            cx.theme().colors.background,
            cx.theme().colors.primary_hover,
            cx.theme().colors.primary_active,
        );

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

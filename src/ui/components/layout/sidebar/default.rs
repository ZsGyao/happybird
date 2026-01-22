// src/ui/sidebar.rs

use gpui::{prelude::FluentBuilder, *};
use gpui_component::{
    ActiveTheme, Colorize, Icon, IconName,
    button::{Button, ButtonVariants},
    h_flex,
    label::Label,
    menu::{DropdownMenu, PopupMenuItem},
    v_flex,
};

use crate::ui::{
    models::{AppPage, GlobalAppState},
    theme::infra::hb_icons::HappyBirdIcons,
};

/// 侧边导航栏组件。
///
/// 负责展示应用的全局导航菜单、应用标识以及底部的全局设置入口。
/// 它是应用的主要导航枢纽，位于界面最左侧。
pub struct HappyBirdSideBar;

impl HappyBirdSideBar {
    /// 创建一个新的 `SideBar` 实例。
    ///
    /// # Arguments
    /// * `_cx` - 应用上下文，可用于初始化状态或订阅事件。
    pub fn new(cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            // [关键] 订阅全局状态
            let global_model = cx.global::<GlobalAppState>().0.clone();
            cx.observe(&global_model, |_, _, cx| {
                // 当全局状态变化时，重新渲染 SideBar
                cx.notify();
            })
            .detach();

            Self
        })
    }

    /// 辅助函数：渲染一个统一风格的导航菜单项。
    ///
    /// # Arguments
    /// * `icon` - 菜单项左侧的图标。
    /// * `label` - 菜单项显示的文本。
    /// * `page_id` - 该菜单项对应的页面标识符。
    /// * `cx` - 组件上下文，用于获取主题和处理事件。
    // 辅助函数：渲染导航菜单项
    fn render_nav_item(
        &self,
        icon: IconName,
        label: &str,
        target_page: AppPage, // 这里接收目标页面枚举
        is_collapsed: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();

        // [关键] 从全局状态读取当前页面
        let global_state = cx.global::<GlobalAppState>().0.read(cx);
        let is_active = global_state.current_page == target_page;

        let (bg, fg) = if is_active {
            (theme.colors.secondary, theme.colors.foreground)
        } else {
            (gpui::transparent_black(), theme.colors.muted_foreground)
        };

        let hover_active_bg = theme.colors.secondary.darken(0.05);
        let hover_inactive_bg = theme.colors.link_hover;

        // 获取全局 Model 的句柄，用于在点击闭包中更新状态
        let global_handle = cx.global::<GlobalAppState>().0.clone();
        let target_page_clone = target_page.clone(); // 克隆一份给闭包使用

        h_flex()
            .id("sidebar-item")
            .w_full()
            .items_center()
            .gap(px(8.0))
            .px(px(12.0))
            .py(px(8.0))
            .rounded_md()
            .cursor_pointer()
            .bg(bg)
            .hover(move |s| {
                if is_active {
                    s.bg(hover_active_bg)
                } else {
                    s.bg(hover_inactive_bg)
                }
            })
            .child(Icon::new(icon).text_color(fg))
            .when(!is_collapsed, |this| {
                this.child(
                    Label::new(label.to_string())
                        .text_color(fg)
                        .text_sm()
                        .font_weight(if is_active {
                            FontWeight::SEMIBOLD
                        } else {
                            FontWeight::NORMAL
                        }),
                )
            })
            // [关键] 点击事件：只负责更新全局状态
            .on_click(move |_, _, cx| {
                global_handle.update(cx, |model, cx| {
                    // 调用 Model 的方法来切换页面
                    model.navigate_to(target_page_clone.clone(), cx);
                    // 这里不需要 cx.notify()，因为 model.navigate_to 内部已经 notify 了
                    // 并且 observe 会自动触发 SideBar 的重绘
                });
            })
    }
}

impl Render for HappyBirdSideBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_collapsed = cx
            .global::<GlobalAppState>()
            .0
            .read(cx)
            .is_sidebar_collapsed;

        // 侧边栏整体容器：垂直布局，占满高度，设置背景和右边框
        v_flex()
            .size_full()
            .bg(cx.theme().colors.background)
            .border_r_1()
            .border_color(cx.theme().colors.border)
            // =========================================================
            // 2. 中间区域：主导航菜单 (Main Navigation)
            // =========================================================
            // 使用 flex_1 占据剩余垂直空间，内容过多时可滚动
            .child(
                v_flex()
                    .id("main-navigation")
                    .flex_1()
                    .gap(px(4.0)) // 菜单项之间的垂直间距
                    .overflow_y_scroll()
                    .py(px(8.0))
                    .px(px(8.0))
                    .when(!is_collapsed, |this| {
                        this.child(
                            Label::new("MENU")
                                .text_xs()
                                .text_color(cx.theme().colors.muted_foreground)
                                .px(px(12.0))
                                .pb(px(4.0)),
                        )
                    })
                    // 渲染导航项
                    .child(self.render_nav_item(
                        IconName::User,
                        "Users",
                        AppPage::Users,
                        is_collapsed,
                        cx,
                    )),
                // 示例：可以添加更多导航项
                //  .child(self.render_nav_item(IconName::BarChart, "Dashboard", AppPage::Dashboard, cx)),
            )
            // =========================================================
            // 3. 底部区域：全局设置与操作 (Footer)
            // =========================================================
            // 固定在底部，放置不常用的全局功能
            .child(
                v_flex()
                    .flex_shrink_0() // 固定高度，不被压缩
                    .border_t_1() // 顶部分隔线
                    .border_color(cx.theme().colors.border)
                    .child(
                        // 设置按钮：点击弹出菜单
                        Button::new("settings-footer-btn")
                            .icon(IconName::Settings)
                            .when(!is_collapsed, |btn| btn.label("Settings"))
                            .ghost() // 幽灵样式，融入背景
                            .w_full() // 占满宽度
                            // [修改] 折叠时居中，展开时靠左
                            .when_else(
                                is_collapsed,
                                |btn| btn.justify_center(),
                                |btn| btn.justify_start().px(px(12.0)), // 展开时添加内边距
                            )
                            .h(px(40.0))
                            .items_center()
                            .dropdown_menu(move |menu, _, cx| {
                                menu
                                    // --- 应用安全 ---
                                    .item(
                                        PopupMenuItem::new("Lock App")
                                            .icon(HappyBirdIcons::Lock.load(cx))
                                            .on_click(|_, _, cx| {
                                                // TODO: 触发锁定应用的全局 Action
                                                println!("Action: Lock App Triggered");
                                                let g = cx.global::<GlobalAppState>().0.clone();
                                                g.update(cx, |model, cx| model.try_lock_app(cx));
                                            }),
                                    )
                                    // --- 主题切换 ---
                                    .separator()
                                    // 注：GPUI 目前菜单项不支持直接显示当前选中状态，
                                    // 这里仅作为功能入口示例。
                                    .item(
                                        PopupMenuItem::new("Light Mode")
                                            .icon(IconName::Sun)
                                            .on_click(|_, _, _| println!("Theme: Light")),
                                    )
                                    .item(
                                        PopupMenuItem::new("Dark Mode")
                                            .icon(IconName::Moon)
                                            .on_click(|_, _, _| println!("Theme: Dark")),
                                    )
                                    // --- 其他信息 ---
                                    .separator()
                                    .item(
                                        PopupMenuItem::new("About HappyBird").icon(IconName::Info),
                                    )
                            }),
                    ), // 示例：可以在这里添加当前登录用户的简单信息
                       // .child(
                       //     h_flex().items_center().gap(px(8.0)).px(px(12.0)).py(px(8.0))
                       //         .child(Icon::new(IconName::UserCircle).text_color(theme.colors.muted_foreground))
                       //         .child(Label::new("Admin").text_sm())
                       // )
            )
    }
}

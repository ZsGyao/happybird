// src/ui/status_bar.rs

use crate::ui::{models::GlobalAppState, theme::infra::hb_icons::HappyBirdIcons};
use gpui::*;
use gpui_component::{ActiveTheme, Icon, IconName, h_flex, label::Label};

pub struct StatusBar;

impl StatusBar {
    pub fn new(cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self)
    }

    /// 渲染状态栏的通用 Item 容器
    fn render_item(&self, content: impl IntoElement) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .px(px(8.0))
            .h_full()
            .cursor_pointer()
            // Hover 效果
            .hover(|s| s.bg(gpui::white().opacity(0.1)))
            // 文本样式
            .text_xs()
            .child(content)
    }
}

impl Render for StatusBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 1. 获取全局状态 (Data Snapshot)
        let (total_count, selected_count, is_exporting, version) = {
            if cx.has_global::<GlobalAppState>() {
                let global = cx.global::<GlobalAppState>().0.read(cx);
                (
                    global.total_count,
                    global.multi_selection.selected_ids.len(),
                    global.export_state.is_exporting,
                    "v0.1.0",
                )
            } else {
                (0, 0, false, "Unknown")
            }
        };

        // 2. 预加载图标 (解决 cx 借用冲突)
        // Icon::load(cx) 需要 cx，而后面构建 UI 时需要 theme (也借用 cx)
        // 所以我们先加载图标
        let icon_git = HappyBirdIcons::Github.load(cx);
        let icon_loader = IconName::Loader; // 内置图标不需要 load
        let icon_db = HappyBirdIcons::Database; // 假设用这个代替圆点
        let icon_msg = HappyBirdIcons::MessagesSquare.load(cx);
        let icon_bell = IconName::Bell;

        // 3. 获取 Theme (从这里开始 cx 被不可变借用)
        let theme = cx.theme();
        let bg_color = theme.colors.tab_bar;
        let border_color = theme.colors.border;
        let text_color = theme.colors.muted_foreground;
        let hover_bg = theme.colors.list_hover;
        let primary_color = theme.colors.primary;
        let warning_color = theme.colors.warning;
        let foreground_color = theme.colors.foreground;

        // 4. 构建 UI
        div()
            .w_full()
            .h(px(28.0))
            .flex()
            .items_center()
            .justify_between()
            .bg(bg_color)
            .border_t_1()
            .border_color(border_color)
            .text_xs()
            .text_color(text_color)
            // --- 左侧区域 ---
            .child(
                h_flex()
                    .h_full()
                    .items_center()
                    // 1. 版本号
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .px(px(8.0))
                            .h_full()
                            .cursor_pointer()
                            .hover(|s| s.bg(hover_bg))
                            .child(
                                h_flex()
                                    .gap_1()
                                    .items_center()
                                    .child(Icon::new(icon_git).size(px(12.0)))
                                    .child(Label::new(version)),
                            ),
                    )
                    // 2. 数据统计
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .px(px(8.0))
                            .h_full()
                            .cursor_pointer()
                            .hover(|s| s.bg(hover_bg))
                            .child(if selected_count > 0 {
                                Label::new(format!("{} selected", selected_count))
                                    .text_color(primary_color)
                            } else {
                                Label::new(format!("{} subjects", total_count))
                            }),
                    ),
            )
            // --- 中间区域 (Task Status) ---
            .child(h_flex().h_full().items_center().children(if is_exporting {
                Some(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            Icon::new(icon_loader)
                                .size(px(12.0))
                                .text_color(warning_color),
                        )
                        .child(Label::new("Exporting data...").text_color(foreground_color)),
                )
            } else {
                None
            }))
            // --- 右侧区域 ---
            .child(
                h_flex()
                    .h_full()
                    .items_center()
                    // 1. 数据库状态
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .px(px(8.0))
                            .h_full()
                            .cursor_pointer()
                            .hover(|s| s.bg(hover_bg))
                            .child(
                                h_flex()
                                    .gap_1()
                                    .items_center()
                                    .child(div().size(px(8.0)).rounded_full().bg(gpui::green()))
                                    .child(Label::new("Connected")),
                            ),
                    )
                    // 2. Feedback
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .px(px(8.0))
                            .h_full()
                            .cursor_pointer()
                            .hover(|s| s.bg(hover_bg))
                            .child(Icon::new(icon_msg).size(px(14.0))),
                    )
                    // 3. Notifications
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .px(px(8.0))
                            .h_full()
                            .cursor_pointer()
                            .hover(|s| s.bg(hover_bg))
                            .child(Icon::new(icon_bell).size(px(14.0))),
                    ),
            )
    }
}

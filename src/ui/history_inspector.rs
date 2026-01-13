use std::collections::{BTreeMap, HashMap};

use gpui::{
    AnyElement, App, FontWeight, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div, px,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable,
    button::{Button, ButtonVariants},
    h_flex,
    label::Label,
    v_flex,
};

use crate::{
    backend::db::models::ChangeLogEntry,
    ui::{detail_panel::SwitchInspectorMode, models::HistoryViewMode},
};

/// 历史记录检查器组件
///
/// 负责渲染右侧边栏的变更历史时间线
pub struct HistoryInspector;

impl HistoryInspector {
    /// 渲染主入口
    ///
    /// # Arguments
    /// * `history` - 历史记录列表的 Option。None 表示还未加载，Some 表示已加载（可能为空）。
    /// * `cx` - WindowContext
    pub fn render(
        history: Option<&Vec<ChangeLogEntry>>,
        mode: HistoryViewMode,
        cx: &mut App,
    ) -> AnyElement {
        let theme = cx.theme();

        v_flex()
            .w(px(320.0))
            .h_full()
            .bg(theme.colors.tab_bar)
            .border_l_1()
            .border_color(theme.colors.border)
            // --- Header ---
            .child(
                h_flex()
                    .h(px(48.0))
                    .px(px(16.0))
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .items_center()
                    .justify_between()
                    .child(Label::new("History").font_weight(FontWeight::BOLD))
                    // [修复问题 2] 添加显式的切换视图按钮
                    .child(
                        Button::new("switch-view-mode")
                            .icon(match mode {
                                HistoryViewMode::Timeline => IconName::Menu, // 或者其他表示列表的图标
                                HistoryViewMode::GroupByField => IconName::Calendar, // 或者表示时间的图标
                            })
                            .ghost()
                            .small()
                            .tooltip({
                                let text = match mode {
                                    HistoryViewMode::Timeline => "Group by Field",
                                    HistoryViewMode::GroupByField => "View Timeline",
                                };
                                text
                            })
                            // 触发 DetailPanel 定义的 Action
                            .on_click(|_e, _window, cx| {
                                cx.dispatch_action(&SwitchInspectorMode);
                            }),
                    ),
            )
            // --- Content ---
            .child(
                div().id("area-content").flex_1().overflow_y_scroll().child(
                    v_flex()
                        .p(px(16.0))
                        .gap(px(16.0))
                        .child(if let Some(logs) = history {
                            if logs.is_empty() {
                                Self::render_empty(cx)
                            } else {
                                match mode {
                                    HistoryViewMode::Timeline => Self::render_timeline(logs, cx),
                                    HistoryViewMode::GroupByField => Self::render_grouped(logs, cx),
                                }
                            }
                        } else {
                            Self::render_loading(cx)
                        }),
                ),
            )
            .into_any_element()
    }

    /// 渲染时间线列表
    fn render_timeline(logs: &Vec<ChangeLogEntry>, cx: &App) -> AnyElement {
        let theme = cx.theme();

        v_flex()
            .gap(px(16.0))
            .children(logs.iter().map(|entry| {
                let is_create = entry.action_type == "CREATE";

                h_flex()
                    .items_start()
                    .gap(px(10.0))
                    .child(
                        // 左侧时间轴线
                        v_flex()
                            .items_center()
                            .h_full()
                            .child(
                                div()
                                    .size(px(8.0))
                                    .rounded_full()
                                    .bg(if is_create {
                                        theme.colors.success
                                    } else {
                                        theme.colors.primary
                                    })
                                    .mt(px(6.0)),
                            )
                            .child(
                                div() // 竖线
                                    .w(px(1.0))
                                    .flex_1()
                                    .bg(theme.colors.border)
                                    .my(px(4.0))
                                    .min_h(px(20.0)),
                            ),
                    )
                    .child(
                        // 右侧卡片内容
                        v_flex()
                            .flex_1()
                            .gap(px(4.0))
                            .child(
                                h_flex()
                                    .justify_between()
                                    .child(
                                        Label::new(entry.action_type.clone())
                                            .text_xs()
                                            .font_weight(FontWeight::BOLD),
                                    )
                                    .child(
                                        Label::new(entry.created_at.clone())
                                            .text_xs()
                                            .text_color(theme.colors.muted_foreground),
                                    ),
                            )
                            .child(Self::render_diff_detail(entry, cx)),
                    )
            }))
            .into_any_element()
    }

    /// 模式 B: 按字段分组视图
    fn render_grouped(logs: &Vec<ChangeLogEntry>, cx: &App) -> AnyElement {
        let theme = cx.theme();
        let mut groups: BTreeMap<String, Vec<&ChangeLogEntry>> = BTreeMap::new();

        for entry in logs {
            let key = entry.field_key.clone().unwrap_or("General".to_string());
            groups.entry(key).or_default().push(entry);
        }

        v_flex()
            .gap(px(20.0))
            .children(groups.into_iter().map(|(field, entries)| {
                v_flex()
                    .gap(px(8.0))
                    .child(
                        Label::new(field.to_uppercase())
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .text_color(theme.colors.primary),
                    )
                    .child(
                        v_flex()
                            .pl(px(12.0))
                            .border_l_2()
                            .border_color(theme.colors.border)
                            .children(
                                entries.iter().map(|e| {
                                    div().py(px(4.0)).child(Self::render_diff_detail(e, cx))
                                }),
                            ),
                    )
            }))
            .into_any_element()
    }

    /// 渲染 Diff 详情 (旧值 -> 新值)
    fn render_diff_detail(entry: &ChangeLogEntry, cx: &App) -> AnyElement {
        let theme = cx.theme();
        if entry.action_type == "CREATE" {
            return div()
                .child(
                    Label::new("User created.")
                        .text_sm()
                        .text_color(theme.colors.muted_foreground),
                )
                .into_any_element();
        }

        let old = entry.old_value.as_deref().unwrap_or("-");
        let new = entry.new_value.as_deref().unwrap_or("-");

        v_flex()
            .bg(theme.colors.background)
            .rounded_md()
            .p(px(8.0))
            .border_1()
            .border_color(theme.colors.border)
            .child(
                h_flex()
                    .items_center()
                    .gap(px(6.0))
                    .flex_wrap()
                    .child(
                        Label::new(old.to_string())
                            .text_xs()
                            .text_decoration_2()
                            .text_color(theme.colors.muted_foreground),
                    )
                    .child(
                        Icon::new(IconName::ArrowRight)
                            .size(px(10.0))
                            .text_color(theme.colors.muted_foreground),
                    )
                    .child(
                        Label::new(new.to_string())
                            .text_xs()
                            .text_color(theme.colors.foreground),
                    ),
            )
            .into_any_element()
    }

    fn render_empty(cx: &App) -> AnyElement {
        div()
            .flex()
            .justify_center()
            .py(px(20.0))
            .child(
                Label::new("No history found")
                    .text_sm()
                    .text_color(cx.theme().colors.muted_foreground),
            )
            .into_any_element()
    }

    fn render_loading(_cx: &App) -> AnyElement {
        div()
            .flex()
            .justify_center()
            .py(px(20.0))
            .child(Label::new("Loading...").text_sm())
            .into_any_element()
    }
}

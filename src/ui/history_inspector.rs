use std::collections::BTreeMap;

use gpui::{
    AppContext, Context, Entity, FontWeight, InteractiveElement, IntoElement, ParentElement,
    Render, StatefulInteractiveElement, Styled, Subscription, Window, div, px,
};
use gpui_component::{
    ActiveTheme, Icon, Sizable,
    button::{Button, ButtonVariants},
    h_flex,
    input::{Input, InputEvent, InputState},
    label::Label,
    v_flex,
};

use crate::{
    backend::db::models::ChangeLogEntry,
    ui::{
        hb_icons::HappyBirdIcons,
        models::{GlobalAppState, HistoryStore, HistoryViewMode},
        theme::style::ThemedStyle,
    },
};

/// 历史记录检查器组件
///
/// 负责渲染右侧边栏的变更历史时间线
pub struct HistoryInspector {
    /// 共享数据句柄 (充当 Model)
    history_entity: Entity<HistoryStore>,
    subject_id: i32,
    /// [状态] 当前视图模式
    mode: HistoryViewMode,
    /// [交互] 当前正在编辑的 Log ID
    editing_id: Option<i32>,
    /// [交互] 输入框状态 (必须由本 Entity 持有)
    input_state: Entity<InputState>,
    /// 保存一个 Subscription 以保持监听活跃
    _input_subscription: Subscription,
}

impl HistoryInspector {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        subject_id: i32,
        history_entity: Entity<HistoryStore>,
    ) -> Self {
        let input_state = cx.new(|cx| InputState::new(window, cx).placeholder("Add a note..."));

        // 订阅 Input 事件来实现 Blur 保存和 Enter 保存
        let _input_subscription = cx.subscribe_in(
            &input_state,
            window,
            |this: &mut Self, _state, event, _window, cx| {
                match event {
                    InputEvent::Blur => {
                        // 失焦自动保存,停止编辑
                        if let Some(id) = this.editing_id {
                            println!("Loss focus");
                            this.save_remark(id, cx);
                            this.cancel_editing(cx);
                        }
                    }
                    InputEvent::PressEnter { .. } => {
                        // 回车保存
                        if let Some(id) = this.editing_id {
                            this.save_remark(id, cx);
                        }
                    }
                    InputEvent::Focus => {}
                    InputEvent::Change => {}
                }
            },
        );
        // 订阅数据变化
        // cx.observe 可以在 Entity 之间建立响应式连接
        cx.observe(&history_entity, |_, _, cx| {
            cx.notify();
        })
        .detach();

        Self {
            history_entity,
            subject_id,
            mode: HistoryViewMode::Timeline,
            editing_id: None,
            input_state,
            _input_subscription,
        }
    }

    pub fn toggle_mode(&mut self, cx: &mut Context<Self>) {
        self.mode = match self.mode {
            HistoryViewMode::Timeline => HistoryViewMode::GroupByField,
            HistoryViewMode::GroupByField => HistoryViewMode::Timeline,
        };
        cx.notify();
    }

    fn start_editing(
        &mut self,
        log_id: i32,
        current_text: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.editing_id = Some(log_id);
        let text = current_text.unwrap_or_default();

        self.input_state
            .update(cx, |state, cx| state.set_value(text, window, cx));

        cx.notify();
    }

    fn cancel_editing(&mut self, cx: &mut Context<Self>) {
        self.editing_id = None;
        cx.notify();
    }

    fn save_remark(&mut self, log_id: i32, cx: &mut Context<Self>) {
        let text = self.input_state.read(cx).value();
        let new_remark = if text.trim().is_empty() {
            None
        } else {
            Some(text.to_string())
        };

        // 1. 异步 DB 更新
        if let Some(global) = cx.try_global::<GlobalAppState>() {
            let db_manager = global.0.read(cx).get_db_manager();
            let remark_for_db = new_remark.clone();
            cx.spawn(async move |_, _cx| {
                if let Ok(conn) = db_manager.get_conn() {
                    let _ = crate::backend::db::ops::DataService::update_log_remark(
                        &conn,
                        log_id,
                        remark_for_db,
                    );
                }
            })
            .detach();
        }

        // 2. 本地更新 Entity (Model)
        self.history_entity.update(cx, |store, _| {
            if let Some(entry) = store.entries.iter_mut().find(|e| e.id == log_id) {
                entry.remark = new_remark;
            }
        });

        self.editing_id = None;
        // update 会触发 observe 的 notify，所以这里不需要手动 notify
    }
}

impl Render for HistoryInspector {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        // 从 Entity 读取数据
        let store = self.history_entity.read(cx);
        let history = &store.entries;

        v_flex()
            .w(px(320.0))
            .h_full()
            .bg(theme.colors.tab_bar)
            .border_l_1()
            .border_color(theme.colors.border)
            .child(
                // Header
                h_flex()
                    .h(px(48.0))
                    .px(px(16.0))
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(theme.colors.border)
                    .child(Label::new("History").font_weight(FontWeight::BOLD))
                    .child(
                        // 视图切换按钮
                        Button::new("mode-switch")
                            .icon(match self.mode {
                                HistoryViewMode::Timeline => HappyBirdIcons::List.load(cx),
                                HistoryViewMode::GroupByField => {
                                    HappyBirdIcons::LayoutList.load(cx)
                                }
                            })
                            .ghost()
                            .small()
                            .on_click(cx.listener(|this, _, _, cx| this.toggle_mode(cx))),
                    ),
            )
            .child(
                div()
                    .id("history-li")
                    .flex_1()
                    .overflow_y_scroll()
                    .p(px(16.0))
                    .child(if history.is_empty() {
                        Label::new("No history found")
                            .text_color(theme.colors.muted_foreground)
                            .into_any_element()
                    } else {
                        match self.mode {
                            HistoryViewMode::Timeline => {
                                self.render_timeline(history, cx).into_any_element()
                            }
                            HistoryViewMode::GroupByField => {
                                self.render_grouped(history, cx).into_any_element()
                            }
                        }
                    }),
            )
    }
}

impl HistoryInspector {
    fn render_timeline(
        &self,
        history: &Vec<ChangeLogEntry>,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        v_flex()
            .gap(px(12.0))
            .children(history.iter().map(|entry| self.render_item(entry, cx)))
            .into_any_element()
    }

    fn render_grouped(
        &self,
        history: &Vec<ChangeLogEntry>,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();
        let mut groups: BTreeMap<String, Vec<&ChangeLogEntry>> = BTreeMap::new();
        for entry in history {
            let key = entry
                .field_key
                .clone()
                .unwrap_or_else(|| "General".to_string());
            groups.entry(key).or_default().push(entry);
        }

        v_flex()
            .gap(px(20.0))
            .children(groups.into_iter().map(|(field, entries)| {
                v_flex()
                    .gap(px(8.0))
                    .child(
                        h_flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(
                                Icon::new(HappyBirdIcons::Hash.load(cx))
                                    .size(px(12.0))
                                    .text_color(theme.colors.primary),
                            )
                            .child(Label::new(field).font_weight(FontWeight::BOLD).text_sm()),
                    )
                    .child(
                        v_flex()
                            .pl(px(12.0))
                            .border_l_1()
                            .border_color(theme.colors.border)
                            .gap(px(8.0))
                            .children(entries.into_iter().map(|entry| self.render_item(entry, cx))),
                    )
            }))
            .into_any_element()
    }

    fn render_item(&self, entry: &ChangeLogEntry, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let id = entry.id;
        let is_editing = self.editing_id == Some(id);
        let remark = entry.remark.clone();

        let remark_for_click = entry.remark.clone();

        div()
            .group("item")
            .app_panel_style(cx)
            .p_3()
            .bg(theme.colors.background)
            .child(
                h_flex()
                    .justify_between()
                    .text_xs()
                    .text_color(theme.colors.muted_foreground)
                    .child(entry.action_type.clone())
                    .child(entry.created_at.clone()),
            )
            .child(div().mt_1().text_sm().child(format!(
                "{}: {} -> {}",
                entry.field_key.as_deref().unwrap_or("?"),
                entry.old_value.as_deref().unwrap_or("∅"),
                entry.new_value.as_deref().unwrap_or("∅")
            )))
            .child(if is_editing {
                div().mt_2().child(
                    Input::new(&self.input_state)
                        .small()
                        .appearance(false)
                        .border_b(px(1.0))
                        .border_color(theme.colors.primary),
                )
            } else {
                div()
                    .mt_1()
                    .min_h(px(20.0))
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap(px(8.0))
                    .child(if let Some(note) = remark {
                        h_flex()
                            .gap_1()
                            .items_start()
                            .flex_1()
                            .min_w(px(0.0))
                            .child(
                                div().pt(px(2.0)).child(
                                    Icon::new(HappyBirdIcons::Message.load(cx))
                                        .size(px(12.0))
                                        .text_color(gpui::yellow()),
                                ),
                            )
                            .child(
                                div().flex_1().w_full().child(
                                    Label::new(note)
                                        .text_xs()
                                        .text_color(theme.colors.muted_foreground)
                                        .whitespace_normal(),
                                ),
                            )
                    } else {
                        div()
                    })
                    .child(
                        // Edit button
                        div()
                            .id("edit-cli")
                            .flex_shrink_0()
                            .invisible()
                            .group_hover("item", |s| s.visible())
                            .cursor_pointer()
                            .child(
                                Icon::new(HappyBirdIcons::Edit.load(cx))
                                    .size(px(14.0))
                                    .text_color(theme.colors.primary),
                            )
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.start_editing(id, remark_for_click.clone(), window, cx);
                            })),
                    )
            })
    }
}

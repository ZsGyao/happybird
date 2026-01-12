// src/ui/detail_panel.rs

use gpui::{
    App, AppContext, Context, Div, Entity, FocusHandle, IntoElement, KeyBinding, Render,
    ViewContext, Window, prelude::*,
};
use gpui_component::{
    ActiveTheme,
    Icon,
    IconName,
    Sizable,
    StyledExt,
    button::{Button, ButtonVariants},
    h_flex,
    input::TextInput, // 假设使用了 gpui_component 的输入组件
    label::Label,
    scroll::ScrollArea,
    v_flex,
};
use serde_json::Value;
use std::collections::BTreeMap;

use crate::ui::models::GlobalAppState;

// 定义局部 Action，用于处理面板内部的快捷键
gpui::actions!(
    detail_panel,
    [SaveActiveTab, CloseActiveTab, NextTab, PrevTab]
);

pub struct DetailPanel {
    focus_handle: FocusHandle,
    // 如果需要保留滚动位置，可以在这里添加 ScrollHandle
}

impl DetailPanel {
    pub fn new(cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let focus_handle = cx.focus_handle();

            // 1. 绑定快捷键 (作用域仅限此 Panel)
            cx.bind_keys([
                KeyBinding::new("cmd-s", SaveActiveTab, None),
                KeyBinding::new("cmd-w", CloseActiveTab, None),
                KeyBinding::new("ctrl-tab", NextTab, None),
                KeyBinding::new("ctrl-shift-tab", PrevTab, None),
            ]);

            // 2. 注册 Action 监听器
            // cx.on_action(Self::action_save);
            // cx.on_action(Self::action_close);
            // cx.on_action(Self::action_next_tab);
            // cx.on_action(Self::action_prev_tab);

            Self { focus_handle }
        })
    }

    // ========================================================================
    //  Action Handlers
    // ========================================================================

    fn action_save(&mut self, _: &SaveActiveTab, cx: &mut Context<Self>) {
        let global = cx.global::<GlobalAppState>().0.clone();

        // 触发保存逻辑 (异步)
        // 注意：这里我们只负责通知 Model 层去持久化，UI 不直接操作 DB
        global.update(cx, |model, cx| {
            if let Some(active_id) = model.active_tab_id {
                // 找到对应的 Tab
                if let Some(tab) = model.tabs.iter_mut().find(|t| t.subject_id == active_id) {
                    if tab.is_dirty {
                        // 1. 获取变更数据
                        let id = tab.subject_id;
                        let new_attrs = tab.working_attributes.clone();

                        // 2. 乐观更新：先标记为已保存，提升响应速度
                        tab.mark_saved();
                        cx.notify();

                        // 3. 异步写入数据库
                        // 实际项目中，这里应该调用 Backend Service
                        let db_manager = model.get_db_manager(); // 假设 Models 有此方法暴露 DB
                        cx.spawn(move |_view, _cx| async move {
                            // let conn = db_manager.get_conn()?;
                            // DataService::update_fields(&conn, id, new_attrs)?;
                            println!(">>> [Mock Save] Saving Subject {}: {:?}", id, new_attrs);
                        })
                        .detach();
                    }
                }
            }
        });
    }

    fn action_close(&mut self, _: &CloseActiveTab, cx: &mut Context<Self>) {
        let global = cx.global::<GlobalAppState>().0.clone();
        global.update(cx, |model, cx| {
            if let Some(id) = model.active_tab_id {
                model.close_tab(id);
                cx.notify(); // 通知 UI 重绘
            }
        });
    }

    fn action_next_tab(&mut self, _: &NextTab, cx: &mut Context<Self>) {
        let global = cx.global::<GlobalAppState>().0.clone();
        global.update(cx, |model, cx| {
            if model.tabs.is_empty() {
                return;
            }
            if let Some(curr_id) = model.active_tab_id {
                if let Some(pos) = model.tabs.iter().position(|t| t.subject_id == curr_id) {
                    let next_pos = (pos + 1) % model.tabs.len();
                    model.activate_tab(model.tabs[next_pos].subject_id);
                    cx.notify();
                }
            }
        });
    }

    fn action_prev_tab(&mut self, _: &PrevTab, cx: &mut Context<Self>) {
        let global = cx.global::<GlobalAppState>().0.clone();
        global.update(cx, |model, cx| {
            if model.tabs.is_empty() {
                return;
            }
            if let Some(curr_id) = model.active_tab_id {
                if let Some(pos) = model.tabs.iter().position(|t| t.subject_id == curr_id) {
                    let prev_pos = if pos == 0 {
                        model.tabs.len() - 1
                    } else {
                        pos - 1
                    };
                    model.activate_tab(model.tabs[prev_pos].subject_id);
                    cx.notify();
                }
            }
        });
    }

    // ========================================================================
    //  Render Helpers
    // ========================================================================

    /// 渲染顶部的 Tab 栏
    fn render_tab_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let global = cx.global::<GlobalAppState>().0.read(cx);
        let tabs = &global.tabs;
        let active_id = global.active_tab_id;

        // 使用 h_flex 布局，支持横向滚动 (如果标签太多)
        h_flex()
            .id("tab-bar")
            .w_full()
            .h(px(36.0))
            .bg(cx.theme().colors.tab_bar_background)
            .border_b_1()
            .border_color(cx.theme().colors.border)
            .items_end() // 底部对齐，模拟文件卡片感
            .child(ScrollArea::horizontal(div().flex().children(
                tabs.iter().map(|tab| {
                    let is_active = Some(tab.subject_id) == active_id;
                    let tab_id = tab.subject_id;
                    let is_dirty = tab.is_dirty;

                    div()
                        .id(SharedString::from(format!("tab-{}", tab_id)))
                        .group("tab") // 用于 hover 样式
                        .flex()
                        .items_center()
                        .h_full()
                        .px(px(10.0))
                        .gap(px(6.0))
                        .min_w(px(120.0))
                        .max_w(px(240.0))
                        .border_r_1()
                        .border_color(cx.theme().colors.border)
                        .cursor_pointer()
                        // 激活状态样式
                        .bg(if is_active {
                            cx.theme().colors.editor_background
                        } else {
                            cx.theme().colors.tab_bar_background
                        })
                        .text_color(if is_active {
                            cx.theme().colors.text
                        } else {
                            cx.theme().colors.muted_foreground
                        })
                        // 悬停样式 (仅对非激活 Tab 生效)
                        .hover(|s| {
                            if !is_active {
                                s.bg(cx.theme().colors.element_hover)
                            } else {
                                s
                            }
                        })
                        // 点击激活
                        .on_click(cx.listener(move |_, _, cx| {
                            cx.global::<GlobalAppState>().0.update(cx, |m, cx| {
                                m.activate_tab(tab_id);
                                cx.notify();
                            });
                        }))
                        // 图标
                        .child(Icon::new(IconName::File).size(px(14.0)))
                        // 标题
                        .child(
                            Label::new(tab.name.clone())
                                .text_sm()
                                .flex_1()
                                .line_clamp_1(),
                        )
                        // 关闭按钮 / 脏状态圆点
                        .child(
                            div()
                                .w(px(18.0))
                                .h(px(18.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded_sm()
                                .hover(|s| s.bg(cx.theme().colors.element_hover))
                                // 点击关闭
                                .on_click(cx.listener(move |_, _, cx| {
                                    // 阻止冒泡非常重要，否则关闭的同时会激活 Tab
                                    cx.stop_propagation();
                                    cx.global::<GlobalAppState>().0.update(cx, |m, cx| {
                                        m.close_tab(tab_id);
                                        cx.notify();
                                    });
                                }))
                                .children(if is_dirty {
                                    // 脏状态：显示实心圆点
                                    vec![
                                        div()
                                            .size(px(8.0))
                                            .rounded_full()
                                            .bg(cx.theme().colors.warning) // 使用警告色 (通常是黄色/橙色)
                                            .into_any_element(),
                                    ]
                                } else {
                                    // 正常状态：默认隐藏，Hover Tab 时显示关闭图标
                                    // (这里简化为始终显示 Icon，为了更好的触摸体验)
                                    vec![
                                        Icon::new(IconName::Close)
                                            .size(px(12.0))
                                            .text_color(cx.theme().colors.muted_foreground)
                                            .into_any_element(),
                                    ]
                                }),
                        )
                }),
            )))
    }

    /// 渲染表单编辑器区域
    fn render_editor(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let global = cx.global::<GlobalAppState>().0.read(cx);

        // 空状态处理 (Empty State)
        if global.active_tab_id.is_none() || global.tabs.is_empty() {
            return div()
                .flex()
                .size_full()
                .items_center()
                .justify_center()
                .bg(cx.theme().colors.editor_background)
                .child(
                    v_flex()
                        .gap(px(12.0))
                        .items_center()
                        .child(
                            Icon::new(IconName::Info)
                                .size(px(48.0))
                                .text_color(cx.theme().colors.border), // 淡淡的图标
                        )
                        .child(
                            Label::new("No Selection")
                                .text_lg()
                                .font_weight(gpui::FontWeight::BOLD)
                                .text_color(cx.theme().colors.muted_foreground),
                        )
                        .child(
                            Label::new("Select a user from the sidebar or press Cmd+P to search.")
                                .text_sm()
                                .text_color(cx.theme().colors.muted_foreground),
                        ),
                )
                .into_any_element();
        }

        let active_tab = global.get_active_tab().unwrap();
        let subject_id = active_tab.subject_id;

        // 使用 BTreeMap 对 Key 进行排序，保证渲染顺序稳定
        let fields: BTreeMap<String, Value> = active_tab
            .working_attributes
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        div()
            .size_full()
            .bg(cx.theme().colors.editor_background)
            .flex()
            .flex_col()
            // 1. 顶部 Toolbar (显示当前文件名 + 操作按钮)
            .child(
                h_flex()
                    .w_full()
                    .h(px(40.0))
                    .px(px(16.0))
                    .border_b_1()
                    .border_color(cx.theme().colors.border)
                    .items_center()
                    .justify_between()
                    .child(
                        h_flex().gap_2().items_center().child(
                            Label::new(active_tab.name.clone())
                                .text_base()
                                .font_weight(gpui::FontWeight::BOLD),
                        ),
                    )
                    .child(
                        h_flex().gap(px(8.0)).child(
                            Button::new("save-btn")
                                .label("Save")
                                .icon(IconName::Save)
                                .small()
                                .primary() // 强调色
                                // 只有脏状态才可点击
                                .disabled(!active_tab.is_dirty)
                                .on_click(cx.listener(|this, _, cx| {
                                    this.action_save(&SaveActiveTab, cx);
                                })),
                        ),
                    ),
            )
            // 2. 属性编辑表单 (滚动区域)
            .child(
                div().flex_1().relative().child(
                    ScrollArea::vertical(
                        div()
                            .p(px(24.0)) // 舒适的内边距
                            .max_w(px(800.0)) // 限制内容最大宽度，防止在大屏上太散
                            .mx_auto() // 居中显示
                            .child(v_flex().gap(px(16.0)).children(fields.into_iter().map(
                                |(key, value)| self.render_field_row(subject_id, key, value, cx),
                            ))),
                    )
                    .size_full(),
                ),
            )
            .into_any_element()
    }

    /// 渲染单行字段编辑器 (Label + Input)
    fn render_field_row(
        &self,
        subject_id: i32,
        key: String,
        value: Value,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();

        // 垂直布局：Label 在上，Input 在下
        v_flex()
            .w_full()
            .gap(px(6.0))
            .child(
                Label::new(key.clone())
                    .text_xs()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(theme.colors.muted_foreground)
                    .uppercase(), // 标签大写，更像 IDE 属性面板
            )
            .child(match value {
                Value::String(s) => {
                    // 使用 Input 组件
                    // 这里为了演示，我们构造一个闭包来处理 onChange
                    let key_clone = key.clone();
                    TextInput::new(format!("input-{}-{}", subject_id, key))
                        .value(s)
                        .placeholder("Empty")
                        .on_change(move |new_val, _window, cx| {
                            let global = cx.global::<GlobalAppState>().0.clone();
                            global.update(cx, |model, _| {
                                if let Some(tab) = model.get_active_tab_mut() {
                                    tab.update_field(&key_clone, Value::String(new_val));
                                }
                            });
                        })
                        .into_any_element()
                }
                Value::Number(n) => {
                    // 数字输入框 (暂时复用 TextInput，实际可加校验)
                    let key_clone = key.clone();
                    TextInput::new(format!("input-{}-{}", subject_id, key))
                        .value(n.to_string())
                        .on_change(move |new_val, _window, cx| {
                            // 尝试解析为数字，解析失败则不更新或保持原样
                            if let Ok(num) = new_val.parse::<f64>() {
                                let global = cx.global::<GlobalAppState>().0.clone();
                                global.update(cx, |model, _| {
                                    if let Some(tab) = model.get_active_tab_mut() {
                                        // serde_json::Number 处理比较麻烦，这里简化处理
                                        if let Some(v) = serde_json::Number::from_f64(num) {
                                            tab.update_field(&key_clone, Value::Number(v));
                                        }
                                    }
                                });
                            }
                        })
                        .into_any_element()
                }
                Value::Bool(b) => {
                    // 布尔值使用 Toggle/Button
                    let key_clone = key.clone();
                    let current_val = b;
                    Button::new(format!("toggle-{}-{}", subject_id, key))
                        .label(if b { "True" } else { "False" })
                        .variant(if b {
                            ButtonVariants::Primary
                        } else {
                            ButtonVariants::Ghost
                        })
                        .on_click(move |_, _, cx| {
                            let global = cx.global::<GlobalAppState>().0.clone();
                            global.update(cx, |model, _| {
                                if let Some(tab) = model.get_active_tab_mut() {
                                    tab.update_field(&key_clone, Value::Bool(!current_val));
                                }
                            });
                        })
                        .into_any_element()
                }
                Value::Null => Label::new("Null")
                    .text_sm()
                    .text_color(theme.colors.muted_foreground)
                    .into_any_element(),
                _ => {
                    // 数组或对象，只读展示
                    Label::new(format!("{:?}", value))
                        .text_xs()
                        .text_color(theme.colors.muted_foreground)
                        .into_any_element()
                }
            })
    }
}

impl Render for DetailPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(cx.theme().colors.editor_background)
            // 确保 Panel 能接收键盘事件 (保存、关闭标签等)
            .track_focus(&self.focus_handle)
            .on_mouse_down(gpui::MouseButton::Left, |_, cx| cx.focus_self())
            // 1. Tab Bar
            .child(self.render_tab_bar(cx))
            // 2. Main Editor Content
            .child(self.render_editor(cx))
    }
}

// src/ui/detail_panel.rs

use gpui::{
    AnyElement, App, AppContext, Context, Entity, FocusHandle, IntoElement, KeyBinding, Render,
    SharedString, Window, div, prelude::*, px,
};
use gpui_component::{
    ActiveTheme, Disableable, Icon, IconName,
    button::{Button, ButtonVariants},
    h_flex,
    label::Label,
    v_flex,
};
use serde_json::Value;
use std::collections::BTreeMap;

use crate::ui::models::GlobalAppState;

// =============================================================================
//  1. Actions (定义面板专属动作)
// =============================================================================

gpui::actions!(
    detail_panel,
    [SaveActiveTab, CloseActiveTab, NextTab, PrevTab]
);

// =============================================================================
//  2. Component Struct (组件结构)
// =============================================================================

pub struct DetailPanel {
    /// 焦点句柄，用于接收键盘事件 (Cmd+S, Cmd+W 等)
    focus_handle: FocusHandle,
}

impl DetailPanel {
    /// 创建 DetailPanel 实例
    pub fn new(cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let focus_handle = cx.focus_handle();

            // --- 绑定快捷键 ---
            // 这些快捷键只有当焦点在 DetailPanel 内时才生效
            cx.bind_keys([
                KeyBinding::new("cmd-s", SaveActiveTab, None),
                KeyBinding::new("cmd-w", CloseActiveTab, None),
                // 类似浏览器的标签切换体验
                KeyBinding::new("ctrl-tab", NextTab, None),
                KeyBinding::new("ctrl-shift-tab", PrevTab, None),
            ]);

            Self { focus_handle }
        })
    }

    // ========================================================================
    //  3. Action Handlers (业务逻辑控制器)
    // ========================================================================

    /// 处理保存动作
    fn action_save(&mut self, _: &SaveActiveTab, _: &mut Window, cx: &mut Context<Self>) {
        let global = cx.global::<GlobalAppState>().0.clone();

        // 1. 在 Model 中检查并获取需要保存的数据
        let save_task = global.update(cx, |model, cx| {
            if let Some(tab) = model.get_active_tab_mut() {
                if tab.is_dirty {
                    let id = tab.subject_id;
                    // 克隆数据准备传给后台线程
                    let new_attrs = tab.working_attributes.clone();

                    // 乐观更新：立即在 UI 上标记为“已保存”，提升用户体感速度
                    tab.mark_saved();
                    cx.notify();

                    return Some((id, new_attrs, model.get_db_manager()));
                }
            }
            None
        });

        // 2. 只有当确实需要保存时，才启动后台任务
        // [修正] 使用 background_executor 避免 AsyncFnOnce 生命周期问题，
        // 因为我们不需要在这个 async 块里操作 UI，只需要操作 DB。
        if let Some((id, new_attrs, db_manager)) = save_task {
            cx.background_executor()
                .spawn(async move {
                    // 将 serde_json::Map 转换为 HashMap
                    let updates: std::collections::HashMap<String, Value> =
                        new_attrs.into_iter().collect();

                    // 执行数据库更新
                    if let Ok(mut conn) = db_manager.get_conn() {
                        match crate::backend::db::ops::DataService::update_fields(
                            &mut conn, id, updates,
                        ) {
                            Ok(_) => println!(">>> [DB] Subject {} saved successfully.", id),
                            Err(e) => eprintln!(">>> [DB Error] Save failed: {}", e),
                        }
                    }
                })
                .detach();
        }
    }

    /// 处理关闭当前标签
    fn action_close(&mut self, _: &CloseActiveTab, _: &mut Window, cx: &mut Context<Self>) {
        let global = cx.global::<GlobalAppState>().0.clone();
        global.update(cx, |model, cx| {
            if let Some(id) = model.active_tab_id {
                model.close_tab(id);
                cx.notify();
            }
        });
    }

    /// 切换到下一个标签
    fn action_next_tab(&mut self, _: &NextTab, _: &mut Window, cx: &mut Context<Self>) {
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

    /// 切换到上一个标签
    fn action_prev_tab(&mut self, _: &PrevTab, _: &mut Window, cx: &mut Context<Self>) {
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
    //  4. UI Renderers (视图渲染)
    // ========================================================================

    /// 渲染顶部的 Tab 栏
    fn render_tab_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let global = cx.global::<GlobalAppState>().0.read(cx);
        let tabs = &global.tabs;
        let active_id = global.active_tab_id;
        let global_model = cx.global::<GlobalAppState>().0.clone();

        h_flex()
            .id("tab-bar")
            .w_full()
            .h(px(36.0))
            .bg(cx.theme().colors.tab)
            .border_b_1()
            .border_color(cx.theme().colors.border)
            .items_end()
            .child(
                // [修正] 使用原生 overflow_x_scroll 替代未定义的 ScrollArea
                div()
                    .id("tab-store")
                    .flex()
                    .size_full()
                    .overflow_x_scroll()
                    .children(tabs.iter().map(|tab| {
                        let is_active = Some(tab.subject_id) == active_id;
                        let tab_id = tab.subject_id;
                        let is_dirty = tab.is_dirty;
                        let model_for_click = global_model.clone();
                        let model_for_close = global_model.clone();

                        div()
                            .id(SharedString::from(format!("tab-{}", tab_id)))
                            .group("tab")
                            .flex()
                            .items_center()
                            .h_full()
                            .px(px(12.0))
                            .gap(px(6.0))
                            .min_w(px(100.0))
                            .max_w(px(200.0))
                            .border_r_1()
                            .border_color(cx.theme().colors.border)
                            .cursor_pointer()
                            // 样式
                            .bg(if is_active {
                                cx.theme().colors.tab_active
                            } else {
                                cx.theme().colors.tab
                            })
                            .text_color(if is_active {
                                cx.theme().colors.tab_active_foreground
                            } else {
                                cx.theme().colors.muted_foreground
                            })
                            .hover(|s| {
                                if !is_active {
                                    s.bg(cx.theme().colors.primary_hover)
                                } else {
                                    s
                                }
                            })
                            // 激活逻辑
                            .on_click(move |_, _, cx| {
                                model_for_click.update(cx, |m, cx| {
                                    m.activate_tab(tab_id);
                                    cx.notify();
                                });
                            })
                            // 1. 图标
                            .child(Icon::new(IconName::File).size(px(14.0)))
                            // 2. 标题
                            .child(Label::new(tab.name.clone()).text_sm().flex_1())
                            // 3. 关闭按钮
                            .child(
                                div()
                                    .id("close-button")
                                    .w(px(18.0))
                                    .h(px(18.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded_sm()
                                    .hover(|s| s.bg(cx.theme().colors.list_hover))
                                    // 关闭逻辑
                                    .on_click(cx.listener(move |_, _, _, cx| {
                                        cx.stop_propagation();
                                        model_for_close.update(cx, |m, cx| {
                                            m.close_tab(tab_id);
                                            cx.notify();
                                        });
                                    }))
                                    .children(if is_dirty {
                                        vec![
                                            div()
                                                .size(px(8.0))
                                                .rounded_full()
                                                .bg(cx.theme().colors.warning)
                                                .into_any_element(),
                                        ]
                                    } else {
                                        vec![
                                            Icon::new(IconName::Close)
                                                .size(px(12.0))
                                                .text_color(cx.theme().colors.muted_foreground)
                                                .into_any_element(),
                                        ]
                                    }),
                            )
                    })),
            )
    }

    /// 渲染编辑区域
    fn render_editor(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let global_read = cx.global::<GlobalAppState>().0.read(cx);

        if global_read.active_tab_id.is_none() || global_read.tabs.is_empty() {
            return div()
                .flex()
                .size_full()
                .items_center()
                .justify_center()
                .bg(cx.theme().colors.background)
                .child(
                    v_flex()
                        .gap(px(16.0))
                        .items_center()
                        .child(
                            Icon::new(IconName::LayoutDashboard)
                                .size(px(64.0))
                                .text_color(cx.theme().colors.border),
                        )
                        .child(
                            Label::new("No Selection")
                                .text_xl()
                                .font_weight(gpui::FontWeight::BOLD)
                                .text_color(cx.theme().colors.muted_foreground),
                        )
                        .child(
                            Label::new("Select a user from the sidebar to view details.")
                                .text_base()
                                .text_color(cx.theme().colors.muted_foreground),
                        ),
                )
                .into_any_element();
        }

        let active_tab = global_read.get_active_tab().unwrap();
        let subject_id = active_tab.subject_id;
        let tab_name = active_tab.name.clone();
        let is_dirty = active_tab.is_dirty;

        // 获取数据快照
        let fields: BTreeMap<String, Value> = active_tab
            .working_attributes
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // [关键修复]: 在这里（主作用域）预先生成所有行组件。
        // 这避免了在 .children(iterator) 的闭包中捕获和使用 &mut cx，从而解决了编译错误。
        let mut field_elements: Vec<AnyElement> = Vec::new();
        for (key, value) in fields {
            field_elements
                .push(Self::render_field_row(subject_id, key, value, cx).into_any_element());
        }

        div()
            .size_full()
            .bg(cx.theme().colors.background)
            .flex()
            .flex_col()
            .child(
                h_flex()
                    .w_full()
                    .h(px(48.0))
                    .px(px(24.0))
                    .border_b_1()
                    .border_color(cx.theme().colors.border)
                    .items_center()
                    .justify_between()
                    .child(
                        h_flex().gap_2().items_center().child(
                            Label::new(tab_name)
                                .text_xl()
                                .font_weight(gpui::FontWeight::BOLD),
                        ),
                    )
                    .child(
                        h_flex().gap(px(12.0)).child(
                            Button::new(SharedString::from("save-btn"))
                                .label("Save Changes")
                                .icon(IconName::Sun)
                                .primary()
                                .disabled(!is_dirty)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.action_save(&SaveActiveTab, window, cx);
                                })),
                        ),
                    ),
            )
            .child(
                div()
                    .id("filed_el")
                    .flex_1()
                    .relative()
                    .overflow_y_scroll()
                    .child(
                        div()
                            .p(px(32.0))
                            .max_w(px(800.0))
                            .mx_auto()
                            .child(v_flex().gap(px(20.0)).children(field_elements)), // 直接传递 Vec
                    ),
            )
            .into_any_element()
    }

    /// 渲染单行字段编辑器
    fn render_field_row(
        subject_id: i32,
        key: String,
        value: Value,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();

        v_flex()
            .w_full()
            .gap(px(6.0))
            .child(
                Label::new(key.clone())
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(theme.colors.muted_foreground),
            )
            .child(match value {
                // Value::String(s) => {
                //     let key_clone = key.clone();
                //     // [修正] 使用 SharedString::from 生成 ID
                //     // [修正] 显式标注闭包参数类型 `new_val: String`
                //     Input::new(SharedString::from(format!("input-{}-{}", subject_id, key)))
                //         .value(s)
                //         .placeholder("Enter value...")
                //         .on_change(move |new_val: String, _window, cx| {
                //             let global = cx.global::<GlobalAppState>().0.clone();
                //             global.update(cx, |model, _| {
                //                 if let Some(tab) = model.get_active_tab_mut() {
                //                     tab.update_field(&key_clone, Value::String(new_val));
                //                 }
                //             });
                //         })
                //         .into_any_element()
                // }
                // Value::Number(n) => {
                //     let key_clone = key.clone();
                //     Input::new(SharedString::from(format!("input-{}-{}", subject_id, key)))
                //         .value(n.to_string())
                //         .on_change(move |new_val: String, _window, cx| {
                //             if let Ok(num) = new_val.parse::<f64>() {
                //                 let global = cx.global::<GlobalAppState>().0.clone();
                //                 global.update(cx, |model, _| {
                //                     if let Some(tab) = model.get_active_tab_mut() {
                //                         if let Some(v) = serde_json::Number::from_f64(num) {
                //                             tab.update_field(&key_clone, Value::Number(v));
                //                         }
                //                     }
                //                 });
                //             }
                //         })
                //         .into_any_element()
                // }

                // [临时回退]: 暂时只展示文本，避免 Input 组件的复杂依赖问题
                // 待引入 InputState 管理机制后，再恢复为 Input
                Value::String(s) => {
                    // let key_clone = key.clone();
                    div()
                        .px(px(8.0))
                        .py(px(6.0))
                        .border_1()
                        .border_color(theme.colors.border)
                        .rounded_md()
                        .bg(theme.colors.background)
                        // .child(Label::new(s).text_sm())
                        // 模拟 Input 的外观
                        .child(s)
                        .into_any_element()
                }
                Value::Number(n) => div()
                    .px(px(8.0))
                    .py(px(6.0))
                    .border_1()
                    .border_color(theme.colors.border)
                    .rounded_md()
                    .bg(theme.colors.background)
                    .child(n.to_string())
                    .into_any_element(),
                Value::Bool(b) => {
                    let key_clone = key.clone();
                    let current_val = b;
                    // [修正] Button 样式：移除 variant()，改用 .when().primary() / .ghost()
                    Button::new(SharedString::from(format!("toggle-{}-{}", subject_id, key)))
                        .label(if b { "TRUE" } else { "FALSE" })
                        .icon(if b { IconName::Check } else { IconName::Close })
                        // 这是一个常见的 GPUI 模式：根据状态应用不同的样式方法
                        .when(b, |btn| btn.primary())
                        .when(!b, |btn| btn.ghost())
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
                Value::Null => div()
                    .p(px(8.0))
                    .bg(theme.colors.secondary)
                    .rounded_md()
                    .child(
                        Label::new("Null")
                            .text_sm()
                            .text_color(theme.colors.muted_foreground),
                    )
                    .into_any_element(),
                _ => div()
                    .p(px(8.0))
                    .bg(theme.colors.secondary)
                    .rounded_md()
                    .child(
                        Label::new(format!("{}", value))
                            .text_sm()
                            .text_color(theme.colors.muted_foreground),
                    )
                    .into_any_element(),
            })
    }
}

impl Render for DetailPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // [修正] 捕获 focus_handle 供闭包使用

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(cx.theme().colors.background)
            .track_focus(&self.focus_handle)
            // --- 注册动作监听器 ---
            .on_action(cx.listener(Self::action_save))
            .on_action(cx.listener(Self::action_close))
            .on_action(cx.listener(Self::action_next_tab))
            .on_action(cx.listener(Self::action_prev_tab))
            .child(self.render_tab_bar(cx))
            .child(self.render_editor(cx))
    }
}

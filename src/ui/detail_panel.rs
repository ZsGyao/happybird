// src/ui/detail_panel.rs

use gpui::{
    AnyElement, App, AppContext, Context, Entity, FocusHandle, IntoElement, KeyBinding, Render,
    SharedString, Subscription, Window, div, prelude::*, px,
};
use gpui_component::{
    ActiveTheme, Disableable, Icon, IconName, Selectable, Sizable,
    button::{Button, ButtonVariants},
    h_flex,
    input::{Input, InputEvent, InputState},
    label::Label,
    v_flex,
};
use serde_json::Value;
use std::collections::BTreeMap;

use crate::ui::{
    history_inspector::HistoryInspector,
    models::{GlobalAppState, HistoryViewMode, TabItem},
};

// =============================================================================
//  1. Actions (定义面板专属动作)
// =============================================================================

gpui::actions!(
    detail_panel,
    [
        SaveActiveTab,       // Save the info detail tab
        CloseActiveTab,      // Close the info detail tab
        NextTab,             // Into next info tab
        PrevTab,             // Into prev info tab
        ToggleEditMode,      // Toggle the edit mode -- edit <-> readonly
        CancelEdit,          // Stop to edit
        ToggleInspector,     // Toggle to open the inspector
        SwitchInspectorMode  // Switch the inspector show mode
    ]
);

// =============================================================================
//  2. Component Struct (组件结构)
// =============================================================================

pub struct DetailPanel {
    /// 焦点句柄，用于接收键盘事件 (Cmd+S, Cmd+W 等)
    focus_handle: FocusHandle,
    /// 本地存储输入框的订阅，防止内存泄漏或重复订阅
    input_subscriptions: BTreeMap<String, Subscription>,
}

impl DetailPanel {
    /// 创建 DetailPanel 实例
    pub fn new(cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let focus_handle = cx.focus_handle();

            // --- 绑定快捷键 ---
            // 这些快捷键只有当焦点在 DetailPanel 内时才生效
            cx.bind_keys([
                KeyBinding::new("ctrl-s", SaveActiveTab, None),
                KeyBinding::new("ctrl-w", CloseActiveTab, None),
                KeyBinding::new("right", NextTab, None),
                KeyBinding::new("left", PrevTab, None),
                KeyBinding::new("ctrl-e", ToggleEditMode, None),
                KeyBinding::new("escape", CancelEdit, None),
                // History panel
                KeyBinding::new("ctrl-h", ToggleInspector, None),
                KeyBinding::new("ctrl-l", SwitchInspectorMode, None),
            ]);

            Self {
                focus_handle,
                input_subscriptions: BTreeMap::new(),
            }
        })
    }

    // ========================================================================
    //  3. Action Handlers (业务逻辑控制器)
    // ========================================================================

    /// 处理保存动作
    fn action_save(&mut self, _: &SaveActiveTab, _: &mut Window, cx: &mut Context<Self>) {
        let global = cx.global::<GlobalAppState>().0.clone();

        // 1. 获取需要保存的数据
        let save_task = global.update(cx, |model, cx| {
            if let Some(tab) = model.get_active_tab_mut() {
                if tab.is_dirty {
                    let id = tab.subject_id;
                    let new_attrs = tab.working_attributes.clone();
                    tab.mark_saved();
                    cx.notify(); // 乐观更新 UI
                    return Some((id, new_attrs, model.get_db_manager()));
                }
            }
            None
        });

        // 2. 异步执行 DB 操作
        if let Some((id, new_attrs, db_manager)) = save_task {
            // 使用 cx.spawn 获取 AsyncWindowContext，它拥有 update 方法
            cx.spawn(async move |_this, cx| {
                let updates: std::collections::HashMap<String, Value> =
                    new_attrs.into_iter().collect();

                // 2.1 这里的代码运行在 Main Thread，但我们把 heavy IO 放到 background
                let result = cx
                    .background_executor()
                    .spawn(async move {
                        if let Ok(mut conn) = db_manager.get_conn() {
                            // 执行更新
                            let _ = crate::backend::db::ops::DataService::update_fields(
                                &mut conn, id, updates,
                            );
                            // [Fix]: 立即拉取最新的 History
                            let new_history =
                                crate::backend::db::ops::DataService::fetch_change_history(
                                    &conn, id,
                                )
                                .ok();
                            return Some(new_history);
                        }
                        None
                    })
                    .await;

                // 2.2 回到 UI 线程更新 Model
                if let Some(new_logs) = result {
                    if let Some(logs) = new_logs {
                        cx.update(|cx| {
                            let global = cx.global::<GlobalAppState>().0.clone();
                            global.update(cx, |model, cx| {
                                if let Some(tab) =
                                    model.tabs.iter_mut().find(|t| t.subject_id == id)
                                {
                                    // [修正] 使用 history_entity 更新
                                    tab.history_entity.update(cx, |store, _| {
                                        store.entries = logs;
                                    });
                                    // Entity update 会自动触发 notify，不需要手动 cx.notify()
                                }
                            });
                        })
                        .ok();
                    }
                }
            })
            .detach();
        }
    }

    /// 切换右侧检查器面板的开关
    fn action_toggle_inspector(
        &mut self,
        _: &ToggleInspector,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let global = cx.global::<GlobalAppState>().0.clone();

        let task = global.update(cx, |model, cx| {
            let db_manager = model.get_db_manager();
            if let Some(tab) = model.get_active_tab_mut() {
                tab.is_inspector_open = !tab.is_inspector_open;

                // [修正] 使用 cx.new 创建实体 (充当 View)
                if tab.inspector_view.is_none() {
                    let subject_id = tab.subject_id;
                    let history_entity = tab.history_entity.clone();

                    let view_entity =
                        cx.new(|cx| HistoryInspector::new(window, cx, subject_id, history_entity));
                    tab.inspector_view = Some(view_entity);
                }

                // 判断是否需要加载数据
                let need_load = tab.history_entity.read(cx).entries.is_empty();

                if tab.is_inspector_open && need_load {
                    return Some((tab.subject_id, db_manager, tab.history_entity.clone()));
                }
                cx.notify();
            }
            None
        });

        // 异步加载
        if let Some((subject_id, db_manager, history_entity)) = task {
            cx.spawn(async move |_, cx| {
                let result = cx
                    .background_executor()
                    .spawn(async move {
                        if let Ok(conn) = db_manager.get_conn() {
                            crate::backend::db::ops::DataService::fetch_change_history(
                                &conn, subject_id,
                            )
                            .ok()
                        } else {
                            None
                        }
                    })
                    .await;

                if let Some(logs) = result {
                    cx.update(|cx| {
                        // [修正] 更新 Entity
                        history_entity.update(cx, |store, _| {
                            store.entries = logs;
                        });
                    })
                    .ok();
                }
            })
            .detach();
        }
    }

    /// 切换历史记录视图模式
    fn action_switch_inspector_mode(
        &mut self,
        _: &SwitchInspectorMode,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let global = cx.global::<GlobalAppState>().0.clone();
        global.update(cx, |model, cx| {
            if let Some(tab) = model.get_active_tab_mut() {
                if let Some(view) = &tab.inspector_view {
                    view.update(cx, |inspector, cx| {
                        inspector.toggle_mode(cx);
                    });
                }
                cx.notify();
            }
        });
    }

    /// 切换编辑模式 (只读 <-> 编辑)
    fn action_toggle_edit(&mut self, _: &ToggleEditMode, _: &mut Window, cx: &mut Context<Self>) {
        let global = cx.global::<GlobalAppState>().0.clone();
        global.update(cx, |model, cx| {
            if let Some(tab) = model.get_active_tab_mut() {
                tab.toggle_edit_mode();
                cx.notify();
            }
        });
    }

    /// 取消编辑 (重置数据并变为只读)
    fn action_cancel_edit(&mut self, _: &CancelEdit, _: &mut Window, cx: &mut Context<Self>) {
        let global = cx.global::<GlobalAppState>().0.clone();
        global.update(cx, |model, cx| {
            if let Some(tab) = model.get_active_tab_mut() {
                // 只有在编辑模式下才响应 ESC
                if tab.is_editing {
                    tab.cancel_edit();
                    cx.notify();
                }
            }
        });
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
    //  4. Logic Helpers
    // ========================================================================

    /// 确保 InputState 存在并已订阅变化
    fn ensure_input_subscription(
        &mut self,
        tab: &mut TabItem,
        key: &str,
        value: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        let unique_key = format!("{}-{}", tab.subject_id, key);

        // 1. 获取或创建 Input Entity (存储在 TabItem 中)
        let input_entity = if let Some(entity) = tab.input_states.get(key) {
            entity.clone()
        } else {
            let entity = cx.new(|cx| {
                let mut state = InputState::new(window, cx);
                state.set_value(value.to_string(), window, cx);
                state
            });
            tab.input_states.insert(key.to_string(), entity.clone());
            entity
        };

        // 2. 确保已订阅事件
        if !self.input_subscriptions.contains_key(&unique_key) {
            let global_handle = cx.global::<GlobalAppState>().0.clone();
            let key_owned = key.to_string();
            let subject_id = tab.subject_id;

            let subscription = cx.subscribe(
                &input_entity,
                move |_view, state: Entity<InputState>, event, cx| {
                    if let InputEvent::Change = event {
                        let new_text = state.read(cx).value();
                        global_handle.update(cx, |model, _| {
                            if let Some(t) =
                                model.tabs.iter_mut().find(|t| t.subject_id == subject_id)
                            {
                                t.update_field(&key_owned, Value::String(new_text.to_string()));
                            }
                        });
                    }
                },
            );

            self.input_subscriptions.insert(unique_key, subscription);
        }

        input_entity
    }

    // ========================================================================
    //  5. UI Renderers
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

    // /// 渲染编辑区域
    // fn render_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    //     let global_handle = cx.global::<GlobalAppState>().0.clone();

    //     // 1. 获取当前 Tab 的数据快照 (Copy-on-write 思想，尽量减少锁的持有时间)
    //     let (active_tab_data, has_tabs) = {
    //         let global = global_handle.read(cx);
    //         if let Some(tab) = global.get_active_tab() {
    //             // 这里我们假设 TabItem 实现了 Clone (Entity 是 cheap clone 的)
    //             (Some(tab.clone()), true)
    //         } else {
    //             (None, !global.tabs.is_empty())
    //         }
    //     };

    //     // 2. 空状态渲染
    //     if !has_tabs || active_tab_data.is_none() {
    //         return div()
    //             .flex()
    //             .size_full()
    //             .items_center()
    //             .justify_center()
    //             .bg(cx.theme().colors.background)
    //             .child(
    //                 v_flex()
    //                     .gap(px(16.0))
    //                     .items_center()
    //                     .child(
    //                         Icon::new(IconName::LayoutDashboard)
    //                             .size(px(64.0))
    //                             .text_color(cx.theme().colors.border),
    //                     )
    //                     .child(
    //                         Label::new("No User Selected")
    //                             .text_xl()
    //                             .font_weight(gpui::FontWeight::BOLD)
    //                             .text_color(cx.theme().colors.muted_foreground),
    //                     )
    //                     .child(
    //                         Label::new("Select a user from the sidebar to view details.")
    //                             .text_base()
    //                             .text_color(cx.theme().colors.muted_foreground),
    //                     ),
    //             )
    //             .into_any_element();
    //     }

    //     // 3. 准备渲染数据
    //     let mut active_tab = active_tab_data.unwrap();
    //     let subject_id = active_tab.subject_id;
    //     let tab_name = active_tab.name.clone();
    //     let is_dirty = active_tab.is_dirty;
    //     let is_editing = active_tab.is_editing; // [关键] 获取编辑模式状态

    //     let fields: BTreeMap<String, Value> = active_tab
    //         .working_attributes
    //         .iter()
    //         .map(|(k, v)| (k.clone(), v.clone()))
    //         .collect();

    //     // 4. 预先生成字段 UI 列表 (解决闭包生命周期问题)
    //     let mut field_elements = Vec::new();

    //     for (key, value) in fields {
    //         // 根据 is_editing 状态决定渲染只读 Label 还是可编辑 Input
    //         let element = if is_editing {
    //             self.render_editable_field(&mut active_tab, &key, value, window, cx)
    //         } else {
    //             self.render_readonly_field(&key, value, cx)
    //         };
    //         field_elements.push(element);
    //     }

    //     // 5. 将新创建的 InputState 同步回 Model (如果有的话)
    //     // 这一步确保我们在 render 期间创建的 Entity 不会丢失
    //     // 注意：这可能导致一次额外的 notify，但在 DetailView 场景下性能是可以接受的
    //     global_handle.update(cx, |m, _| {
    //         if let Some(t) = m.get_active_tab_mut() {
    //             // 简单的合并策略：如果本地有新创建的，就放进去
    //             for (k, v) in active_tab.input_states {
    //                 if !t.input_states.contains_key(&k) {
    //                     t.input_states.insert(k, v);
    //                 }
    //             }
    //         }
    //     });

    //     // 6. 渲染主体布局
    //     div()
    //         .size_full()
    //         .bg(cx.theme().colors.background)
    //         .flex()
    //         .flex_col()
    //         // --- Toolbar ---
    //         .child(
    //             h_flex()
    //                 .w_full()
    //                 .h(px(48.0))
    //                 .px(px(24.0))
    //                 .border_b_1()
    //                 .border_color(cx.theme().colors.border)
    //                 .items_center()
    //                 .justify_between()
    //                 // 标题
    //                 .child(
    //                     h_flex().gap_2().items_center().child(
    //                         Label::new(tab_name)
    //                             .text_xl()
    //                             .font_weight(gpui::FontWeight::BOLD),
    //                     ),
    //                 )
    //                 // 操作区
    //                 .child(
    //                     h_flex()
    //                         .gap(px(12.0))
    //                         // [新增] 锁定/解锁按钮
    //                         .child(
    //                             Button::new(SharedString::from("toggle-edit"))
    //                                 .icon(if is_editing {
    //                                     IconName::Star
    //                                 } else {
    //                                     IconName::StarOff
    //                                 }) // 🔓 / 🔒
    //                                 .label(if is_editing { "Editing" } else { "Read Only" })
    //                                 .when(is_editing, |btn| btn.ghost()) // 编辑态用 Ghost，突出右边的保存
    //                                 .when(!is_editing, |btn| btn.bg(cx.theme().colors.secondary)) // 只读态用 Secondary
    //                                 .on_click(cx.listener(|this, _, window, cx| {
    //                                     this.action_toggle_edit(&ToggleEditMode, window, cx);
    //                                 })),
    //                         )
    //                         // 保存按钮 (仅在编辑且有修改时高亮，或只在编辑模式显示)
    //                         .child(
    //                             Button::new(SharedString::from("save-btn"))
    //                                 .label("Save")
    //                                 .icon(IconName::Sun)
    //                                 .primary()
    //                                 // 逻辑：不在编辑模式下禁用，或者没有脏数据禁用
    //                                 .disabled(!is_editing || !is_dirty)
    //                                 .on_click(cx.listener(|this, _, window, cx| {
    //                                     this.action_save(&SaveActiveTab, window, cx);
    //                                 })),
    //                         )
    //                         // 取消按钮 (仅编辑模式显示)
    //                         .when(is_editing, |div| {
    //                             div.child(
    //                                 Button::new(SharedString::from("cancel-btn"))
    //                                     .label("Cancel")
    //                                     .ghost()
    //                                     .on_click(cx.listener(|this, _, window, cx| {
    //                                         this.action_cancel_edit(&CancelEdit, window, cx);
    //                                     })),
    //                             )
    //                         }),
    //                 ),
    //         )
    //         // --- Form Scroll Area ---
    //         .child(
    //             div()
    //                 .id("tab-scroll-area")
    //                 .flex_1()
    //                 .relative()
    //                 .overflow_y_scroll()
    //                 .child(
    //                     div()
    //                         .p(px(32.0))
    //                         .max_w(px(800.0))
    //                         .mx_auto()
    //                         .child(v_flex().gap(px(20.0)).children(field_elements)),
    //                 ),
    //         )
    //         .into_any_element()
    // }

    /// 渲染左侧表单区域
    fn render_form_area(
        &mut self,
        tab: &mut TabItem,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let tab_name = tab.name.clone();
        let is_dirty = tab.is_dirty;
        let is_editing = tab.is_editing;
        let is_inspector_open = tab.is_inspector_open;

        let fields: BTreeMap<String, Value> = tab
            .working_attributes
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        let mut field_elements = Vec::new();
        for (key, value) in fields {
            let element = if is_editing {
                self.render_editable_field(tab, &key, value, window, cx)
            } else {
                self.render_readonly_field(&key, value, cx)
            };
            field_elements.push(element);
        }

        // 同步 input states
        let global_handle = cx.global::<GlobalAppState>().0.clone();
        global_handle.update(cx, |m, _| {
            if let Some(t) = m.get_active_tab_mut() {
                for (k, v) in tab.input_states.iter() {
                    if !t.input_states.contains_key(k) {
                        t.input_states.insert(k.clone(), v.clone());
                    }
                }
            }
        });

        div()
            .flex_1()
            .size_full()
            .flex()
            .flex_col()
            // Toolbar
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
                        h_flex()
                            .gap(px(12.0))
                            // 检查器开关
                            .child(
                                Button::new(SharedString::from("toggle-history"))
                                    .icon(IconName::Ellipsis)
                                    .selected(is_inspector_open)
                                    .ghost()
                                    .on_click(cx.listener(|this, _, w, c| {
                                        this.action_toggle_inspector(&ToggleInspector, w, c)
                                    })),
                            )
                            .child(
                                Button::new(SharedString::from("toggle-edit"))
                                    .icon(if is_editing {
                                        IconName::Star
                                    } else {
                                        IconName::StarOff
                                    })
                                    .label(if is_editing { "Editing" } else { "Read Only" })
                                    .when(is_editing, |btn| btn.ghost())
                                    .when(!is_editing, |btn| btn.bg(cx.theme().colors.secondary))
                                    .on_click(cx.listener(|this, _, w, c| {
                                        this.action_toggle_edit(&ToggleEditMode, w, c)
                                    })),
                            )
                            .child(
                                Button::new(SharedString::from("save-btn"))
                                    .label("Save")
                                    .icon(IconName::Sun)
                                    .primary()
                                    .disabled(!is_dirty)
                                    .on_click(cx.listener(|this, _, w, c| {
                                        this.action_save(&SaveActiveTab, w, c)
                                    })),
                            )
                            .when(is_editing, |div| {
                                div.child(
                                    Button::new(SharedString::from("cancel-btn"))
                                        .label("Cancel")
                                        .ghost()
                                        .on_click(cx.listener(|this, _, w, c| {
                                            this.action_cancel_edit(&CancelEdit, w, c)
                                        })),
                                )
                            }),
                    ),
            )
            // Form content
            .child(
                div()
                    .id("form-el")
                    .flex_1()
                    .relative()
                    .overflow_y_scroll()
                    .child(
                        div()
                            .p(px(32.0))
                            .max_w(px(800.0))
                            .mx_auto()
                            .child(v_flex().gap(px(20.0)).children(field_elements)),
                    ),
            )
            .into_any_element()
    }

    /// 辅助方法：渲染只读字段 (Label)
    fn render_readonly_field(&self, key: &str, value: Value, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        let display_value = match value {
            Value::String(s) => s,
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => {
                if b {
                    "True".to_string()
                } else {
                    "False".to_string()
                }
            }
            Value::Null => "Null".to_string(),
            _ => format!("{}", value),
        };

        v_flex()
            .w_full()
            .gap(px(6.0))
            .child(
                Label::new(key.to_string())
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(theme.colors.muted_foreground),
            )
            .child(
                div()
                    .px(px(8.0))
                    .py(px(6.0))
                    // 只读模式下，移除边框或者给一个很淡的背景，看起来像文本展示
                    .bg(theme.colors.secondary.opacity(0.3))
                    .rounded_md()
                    .child(Label::new(display_value).text_sm()),
            )
            .into_any_element()
    }

    /// 辅助方法：渲染可编辑字段 (Input/Button)
    fn render_editable_field(
        &mut self,
        tab: &mut TabItem,
        key: &str,
        value: Value,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let subject_id = tab.subject_id;

        // 为了在闭包中使用
        let global_handle = cx.global::<GlobalAppState>().0.clone();
        let key_clone = key.to_string();
        let key_lable = key.to_string();

        let input_component = match value {
            Value::String(s) => {
                let input_entity = self.ensure_input_subscription(tab, key, &s, window, cx);
                Input::new(&input_entity).into_any_element()
            }
            Value::Number(n) => {
                let input_entity =
                    self.ensure_input_subscription(tab, key, &n.to_string(), window, cx);
                Input::new(&input_entity).into_any_element()
            }
            Value::Bool(b) => {
                // Bool 不需要 InputState，直接用 Button Toggle
                Button::new(SharedString::from(format!("toggle-{}-{}", subject_id, key)))
                    .label(if b { "TRUE" } else { "FALSE" })
                    .icon(if b { IconName::Check } else { IconName::Close })
                    .when(b, |btn| btn.primary())
                    .when(!b, |btn| btn.ghost())
                    .on_click(move |_, _, cx| {
                        global_handle.update(cx, |model, _| {
                            if let Some(tab) = model.get_active_tab_mut() {
                                tab.update_field(&key_clone, Value::Bool(!b));
                            }
                        });
                    })
                    .into_any_element()
            }
            // 复杂类型回退到只读显示
            _ => div()
                .px(px(8.0))
                .py(px(6.0))
                .bg(cx.theme().colors.secondary)
                .rounded_md()
                .child(
                    Label::new(format!("{}", value))
                        .text_sm()
                        .text_color(cx.theme().colors.muted_foreground),
                )
                .into_any_element(),
        };

        v_flex()
            .w_full()
            .gap(px(6.0))
            .child(
                Label::new(key_lable)
                    .text_sm()
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(cx.theme().colors.primary),
            ) // 编辑模式下 Label 颜色变深一点提示重点
            .child(input_component)
            .into_any_element()
    }
}

impl Render for DetailPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 获取当前数据快照
        let global_read = cx.global::<GlobalAppState>().0.read(cx);
        let (active_tab_clone, has_active) = if let Some(tab) = global_read.get_active_tab() {
            (Some(tab.clone()), true)
        } else {
            (None, false)
        };

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
            .on_action(cx.listener(Self::action_toggle_edit))
            .on_action(cx.listener(Self::action_cancel_edit))
            .on_action(cx.listener(Self::action_toggle_inspector))
            .on_action(cx.listener(Self::action_switch_inspector_mode))
            .child(self.render_tab_bar(cx))
            // [双栏布局实现]
            .child(div().flex_1().flex().overflow_hidden().children({
                if has_active {
                    let mut tab = active_tab_clone.unwrap();
                    let show_inspector = tab.is_inspector_open;
                    // let inspector_mode = tab.inspector_mode;

                    vec![
                        // 1. 左栏：表单
                        self.render_form_area(&mut tab, window, cx)
                            .into_any_element(),
                        // 2. 右栏：历史检查器 (条件渲染)
                        // 2. 右栏：历史检查器 (条件渲染 - 已修改)
                        if show_inspector {
                            // [关键修改] 直接渲染 View Entity
                            if let Some(view) = tab.inspector_view.clone() {
                                // Entity 在 GPUI 0.2.2 中实现了 IntoElement，可以直接 child()
                                v_flex().h_full().child(view).into_any_element()
                            } else {
                                div().into_any_element()
                            }
                        } else {
                            div().into_any_element()
                        },
                    ]
                } else {
                    vec![
                        div()
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
                                        Label::new("No User Selected")
                                            .text_xl()
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .text_color(cx.theme().colors.muted_foreground),
                                    )
                                    .child(
                                        Label::new(
                                            "Select a user from the sidebar to view details.",
                                        )
                                        .text_base()
                                        .text_color(cx.theme().colors.muted_foreground),
                                    ),
                            )
                            .into_any_element(),
                    ]
                }
            }))
    }
}

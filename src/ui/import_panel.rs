use std::{cmp::Ordering, collections::HashMap};

use gpui::{
    App, AppContext, BorrowAppContext, Context, Entity, InteractiveElement, IntoElement,
    MouseButton, ParentElement, Render, Styled, Subscription, Window, div, px,
};
use gpui_component::{
    ActiveTheme, StyledExt,
    button::Button,
    h_flex,
    input::{Input, InputEvent, InputState},
    table::{Column, Table, TableDelegate, TableState},
    v_flex,
};
use serde_json::Value;

use crate::ui::models::{GlobalAppState, Models};

// =============================================================================
//  1. Table Delegate (Data Adapter)
//  Responsible for telling the table: how much data, headers, sorting, and rendering.
//  It is stateless regarding data; it fetches directly from GlobalAppState.
// =============================================================================

#[derive(Debug, Clone)]
pub struct PreviewDelegate {
    /// Table header define, including the first col "#"
    headers: Vec<String>,
    /// columns conf
    columns: Vec<Column>,
}

impl PreviewDelegate {
    /// Create a new adapter
    ///
    /// * `data_sample` - Used only to extract header keys, does not store data itself
    pub fn new(data_sample: Option<&HashMap<String, Value>>) -> Self {
        // 1. Extract raw keys
        let mut raw_keys: Vec<String> = if let Some(row) = data_sample {
            row.keys().cloned().collect()
        } else {
            vec![]
        };

        // 2.Sort Keys (Name first, others alphabetically)
        raw_keys.sort_by(|a, b| {
            if a == "姓名" {
                Ordering::Less
            } else if b == "姓名" {
                Ordering::Greater
            } else {
                a.cmp(b)
            }
        });

        // 3. Construct header list: ["#", "姓名", "Age", ...]
        let mut headers = vec!["#".to_string()];
        headers.extend(raw_keys);

        // 4. Construct Column configs
        let columns = headers
            .iter()
            .map(|col_name| {
                if col_name == "#" {
                    // === Index Column Config ===
                    Column::new("index", "#")
                        .width(px(50.0)) // 窄一点
                        .fixed_left() // 固定在最左
                        .movable(false) // 不可移动
                } else if col_name == "姓名" {
                    // === Core Data Column Config ===
                    Column::new(col_name, col_name)
                        .width(px(120.0))
                        .fixed_left() // 姓名也固定
                        .movable(false)
                } else {
                    // === Normal Column Config ===
                    Column::new(col_name, col_name)
                        .width(px(120.0))
                        .movable(false)
                }
            })
            .collect();

        Self { headers, columns }
    }

    /// Helper Fun: Format JSON Value to displayable String
    fn format_value(v: Option<&Value>) -> String {
        match v {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Number(n)) => n.to_string(),
            Some(Value::Bool(b)) => b.to_string(),
            Some(Value::Null) | None => "-".to_string(), // 空值显示横杠
            Some(v) => v.to_string(),
        }
    }
}

impl TableDelegate for PreviewDelegate {
    fn columns_count(&self, _cx: &App) -> usize {
        self.headers.len()
    }

    fn rows_count(&self, cx: &App) -> usize {
        let global = cx.global::<GlobalAppState>().0.read(cx);
        global
            .import_preview_state
            .import_preview_data
            .as_ref()
            .map(|d| d.len())
            .unwrap_or(0)
    }

    fn column(&self, col_ix: usize, _cx: &App) -> &gpui_component::table::Column {
        &self.columns[col_ix]
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        window: &mut gpui::Window,
        cx: &mut Context<gpui_component::table::TableState<Self>>,
    ) -> impl IntoElement {
        // Define header_names
        let header_name = &self.headers[col_ix];

        // === Scenario 1: Index Column (#) ===
        if col_ix == 0 {
            return div()
                .size_full()
                .px(px(8.0))
                .flex()
                .items_center()
                .justify_center()
                .bg(cx.theme().colors.secondary) // 稍微灰一点的背景
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().colors.muted)
                        .child((row_ix + 1).to_string()),
                )
                .into_any_element();
        }

        // === Scenario 2: Data Column ===
        // 2.1 Fetch Data directly from Global
        let global_handle = cx.global::<GlobalAppState>().0.clone();
        let global_read = global_handle.read(cx);

        let data_opt = global_read
            .import_preview_state
            .import_preview_data
            .as_ref();
        if data_opt.is_none() {
            return div().into_any_element();
        }
        let row_data = &data_opt.unwrap()[row_ix];

        // Get current cell value string
        let cell_value = Self::format_value(row_data.get(header_name));

        // 2.2 Check if currently editing
        let is_editing = match &global_read.import_preview_state.editing_cell {
            Some((r, k)) => *r == row_ix && k == header_name,
            None => false,
        };

        // 2.3 Rendering Logic Branch
        if is_editing {
            // ============ 编辑模式 ============
            // 此时 Global 中应该已经有 active_input 了（由 ImportPanel 创建）
            if let Some(input_entity) = &global_read.import_preview_state.active_input {
                div()
                    .size_full()
                    // 严格遵循文档：View 渲染时不处理事件，只传入 InputState 实体
                    .child(Input::new(input_entity))
                    .into_any_element()
            } else {
                div().child("Loading input...").into_any_element()
            }
        } else {
            // ============ 查看模式 ============
            let header_clone = header_name.clone();

            div()
                .size_full()
                .px(px(8.0))
                .flex()
                .items_center()
                .cursor(gpui::CursorStyle::IBeam)
                // 双击触发：只修改 Global 状态，不创建 View
                .on_mouse_down(MouseButton::Left, move |e, _win, cx| {
                    if e.click_count >= 2 {
                        cx.stop_propagation();
                        cx.update_global::<GlobalAppState, _>(|app, cx| {
                            app.0.update(cx, |model: &mut Models, cx| {
                                // 1. 设置编辑坐标
                                model.import_preview_state.editing_cell =
                                    Some((row_ix, header_clone.clone()));
                                // 2. 通知 ImportPanel (观察者)，让它去创建 InputState 并订阅事件
                                cx.notify();
                            });
                        });
                    }
                })
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().colors.foreground)
                        .whitespace_nowrap()
                        .overflow_hidden()
                        .text_ellipsis()
                        .child(cell_value),
                )
                .into_any_element()
        }
    }
}

pub struct ImportPanel {
    table_state: Entity<TableState<PreviewDelegate>>,
    // 保存 InputState 的事件订阅，防止被 Drop
    input_subscription: Option<Subscription>,
    subscribed_input_id: Option<gpui::EntityId>,
}

// =============================================================================
//  2. ImportPanel View
// =============================================================================

impl ImportPanel {
    pub fn new(window: &mut Window, cx: &mut App) -> Entity<Self> {
        let global_handle = cx.global::<GlobalAppState>().0.clone();
        let global_read = global_handle.read(cx);
        let first_row = global_read
            .import_preview_state
            .import_preview_data
            .as_ref()
            .and_then(|d| d.first());

        let delegate = PreviewDelegate::new(first_row);
        let table_state = cx.new(|cx| TableState::new(delegate, window, cx));

        cx.new(|cx| {
            // [Fix]: 这个闭包只捕获 global_handle，绝不捕获 window
            cx.observe(&global_handle, move |this: &mut Self, model, cx| {
                this.check_edit_state(model, cx);
                cx.notify();
            })
            .detach();

            Self {
                table_state,
                input_subscription: None,
                subscribed_input_id: None,
            }
        })
    }

    // 辅助方法：检查编辑状态并创建 Input
    fn check_edit_state(&mut self, model: Entity<GlobalAppState>, cx: &mut Context<Self>) {
        let model = model.read(cx);
        let state = &model.0.import_preview_state;

        if let Some((row_ix, key)) = &state.editing_cell {
            if state.active_input.is_none() {
                // 1. 获取值
                let data = state.import_preview_data.as_ref().unwrap();
                let initial_val = PreviewDelegate::format_value(data[*row_ix].get(key));
                let key_clone = key.clone();
                let row_ix_clone = *row_ix;

                // 2. 创建 InputState (使用 ViewContext)
                // [Fix]: 这里的 cx 是 ViewContext<ImportPanel>，它可以转化为 AppContext 用于 InputState::new
                let input_state = cx.new(|cx| {
                    let mut s = InputState::new(cx); // [Fix]: 移除了 window 参数，InputState 应该支持只传 cx
                    s.set_value(initial_val);
                    // s.focus_handle(cx).focus(cx); // [注意]: ViewContext下可能无法直接 focus，或者需要 window。
                    // 如果 InputState::new 必须 window，那我们在 observe 里确实做不到。
                    // 幸好 gpui-component 0.5 的 InputState::new(cx) 是存在的。
                    s
                });

                // 3. 订阅事件
                let sub = cx.subscribe(&input_state, move |this, state, event, cx| match event {
                    InputEvent::Change => {
                        let new_val = state.read(cx).value().to_string();
                        this.update_global_cell(cx, row_ix_clone, &key_clone, new_val);
                    }
                    InputEvent::PressEnter { .. } => this.finish_editing(cx),
                    InputEvent::Blur => this.finish_editing(cx),
                    _ => {}
                });

                self.input_subscription = Some(sub);
                self.subscribed_input_id = Some(input_state.entity_id());

                // 4. 更新 Global
                let input_entity = input_state.clone();
                cx.update_global::<GlobalAppState, _>(|app, cx| {
                    app.0.update(cx, |m: &mut Models, cx| {
                        m.import_preview_state.active_input = Some(input_entity);
                        // 注意：这里可能会触发递归 observe，但因为 active_input 有值了，会跳过创建逻辑
                        cx.notify();
                    });
                });
            }
        } else {
            // 清理订阅
            if this.input_subscription.is_some() {
                this.input_subscription = None;
                this.subscribed_input_id = None;
            }
        }
    }

    fn update_global_cell(&mut self, cx: &mut Context<Self>, row: usize, key: &str, val: String) {
        let key_string = key.to_string();
        cx.update_global::<GlobalAppState, _>(|app, cx| {
            app.0.update(cx, |model: &mut Models, _| {
                model.update_cell_value(row, &key_string, val);
            });
        });
    }

    fn finish_editing(&mut self, cx: &mut Context<Self>) {
        cx.update_global::<GlobalAppState, _>(|app, cx| {
            app.0.update(cx, |model: &mut Models, cx| {
                model.import_preview_state.editing_cell = None;
                model.import_preview_state.active_input = None;
                cx.notify();
            });
        });
    }
}

impl Render for ImportPanel {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) -> impl gpui::IntoElement {
        let global = cx.global::<GlobalAppState>().0.read(cx);
        let row_count = global
            .import_preview_state
            .import_preview_data
            .as_ref()
            .map(|d| d.len())
            .unwrap_or(0);

        div()
            .id("import-panel-overlay")
            .absolute()
            .size_full()
            .bg(gpui::black().opacity(0.5))
            .flex()
            .items_center()
            .justify_center()
            // 在卡片层拦截点击，防止穿透
            // 点击遮罩层退出编辑
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                v_flex()
                    .w(px(900.0))
                    .h(px(600.0))
                    .bg(cx.theme().colors.background)
                    .border_1()
                    .border_color(cx.theme().colors.border)
                    .rounded_xl()
                    .shadow_xl()
                    .overflow_hidden()
                    // --- Header ---
                    .child(
                        h_flex()
                            .flex_shrink_0()
                            .justify_between()
                            .items_center()
                            .px(px(20.0))
                            .py(px(16.0))
                            .border_b_1()
                            .border_color(cx.theme().colors.border)
                            .child(
                                h_flex()
                                    .gap(px(8.0))
                                    .items_baseline()
                                    .child(div().text_xl().font_bold().child("Preview Data"))
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().colors.secondary_foreground)
                                            .child(format!("{} rows ready", row_count)),
                                    ),
                            ),
                    )
                    // --- Table Area ---
                    .child(
                        div()
                            .flex_1()
                            .size_full()
                            // 🔥 强制 flex 子元素不溢出，防止覆盖 Footer
                            .min_h(px(0.0))
                            .child(
                                Table::new(&self.table_state)
                                    .stripe(true)
                                    .bordered(true)
                                    .scrollbar_visible(true, true),
                            ),
                    )
                    // --- Footer Buttons ---
                    .child(
                        h_flex()
                            .flex_shrink_0()
                            .justify_end()
                            .items_center()
                            .px(px(20.0))
                            .py(px(16.0))
                            .gap(px(12.0))
                            .border_t_1()
                            .border_color(cx.theme().colors.border)
                            .bg(cx.theme().colors.background)
                            .child(Button::new("cancel-import").label("Cancel").on_click(
                                |_, _, cx| {
                                    let global = cx.global::<GlobalAppState>().0.clone();
                                    global.update(cx, |m, cx| m.cancel_import(cx));
                                },
                            ))
                            .child(
                                Button::new("confirm-import")
                                    .label("Confirm & Import")
                                    .on_click(|_, _, cx| {
                                        let global = cx.global::<GlobalAppState>().0.clone();
                                        global.update(cx, |m, cx| m.confirm_import(cx));
                                    }),
                            ),
                    ),
            )
    }
}

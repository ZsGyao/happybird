use std::{cmp::Ordering, collections::HashMap};

use gpui::{
    App, AppContext, BorrowAppContext, Context, Entity, Focusable, InteractiveElement, IntoElement,
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
// =============================================================================

#[derive(Debug, Clone)]
pub struct PreviewDelegate {
    headers: Vec<String>,
    columns: Vec<Column>,
}

impl PreviewDelegate {
    pub fn new(data_sample: Option<&HashMap<String, Value>>) -> Self {
        let mut raw_keys: Vec<String> = if let Some(row) = data_sample {
            row.keys().cloned().collect()
        } else {
            vec![]
        };

        raw_keys.sort_by(|a, b| {
            if a == "姓名" {
                Ordering::Less
            } else if b == "姓名" {
                Ordering::Greater
            } else {
                a.cmp(b)
            }
        });

        let mut headers = vec!["#".to_string()];
        headers.extend(raw_keys);

        let columns = headers
            .iter()
            .map(|col_name| {
                if col_name == "#" {
                    Column::new("index", "#")
                        .width(px(50.0))
                        .fixed_left()
                        .movable(false)
                } else if col_name == "姓名" {
                    Column::new(col_name, col_name)
                        .width(px(120.0))
                        .fixed_left()
                        .movable(false)
                } else {
                    Column::new(col_name, col_name)
                        .width(px(120.0))
                        .movable(false)
                }
            })
            .collect();

        Self { headers, columns }
    }

    fn format_value(v: Option<&Value>) -> String {
        match v {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Number(n)) => n.to_string(),
            Some(Value::Bool(b)) => b.to_string(),
            Some(Value::Null) | None => "-".to_string(),
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
        _window: &mut gpui::Window, // Note: We use window in mouse handler, not directly here
        cx: &mut Context<gpui_component::table::TableState<Self>>,
    ) -> impl IntoElement {
        let header_name = &self.headers[col_ix];

        // === Scenario 1: Index Column (#) ===
        if col_ix == 0 {
            return div()
                .size_full()
                .px(px(8.0))
                .flex()
                .items_center()
                .justify_center()
                .bg(cx.theme().colors.secondary)
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().colors.muted)
                        .child((row_ix + 1).to_string()),
                )
                .into_any_element();
        }

        // === Scenario 2: Data Column ===
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
        let cell_value = Self::format_value(row_data.get(header_name));

        // Check editing state
        let is_editing = match &global_read.import_preview_state.editing_cell {
            Some((r, k)) => *r == row_ix && k == header_name,
            None => false,
        };

        if is_editing {
            // === Edit Mode ===
            // Display Input if it exists in Global
            if let Some(input_entity) = &global_read.import_preview_state.active_input {
                div()
                    .size_full()
                    .child(Input::new(input_entity))
                    .into_any_element()
            } else {
                div().child("Initializing...").into_any_element()
            }
        } else {
            // === View Mode ===
            let header_clone = header_name.clone();
            let cell_value_clone = cell_value.clone();

            div()
                .size_full()
                .px(px(8.0))
                .flex()
                .items_center()
                .cursor(gpui::CursorStyle::IBeam)
                // Double Click to Edit
                .on_mouse_down(MouseButton::Left, move |e, window, cx| {
                    if e.click_count >= 2 {
                        cx.stop_propagation();

                        // 1. Create InputState (requires Window access)
                        let input_entity = cx.new(|cx| {
                            let mut s = InputState::new(window, cx);
                            s.set_value(cell_value_clone.clone(), window, cx);
                            s.focus_handle(cx).focus(window); // Auto focus
                            s
                        });

                        // 2. Update Global State
                        // We use the captured global_handle
                        global_handle.update(cx, |models, cx| {
                            models.import_preview_state.editing_cell =
                                Some((row_ix, header_clone.clone()));
                            models.import_preview_state.active_input = Some(input_entity);
                            cx.notify();
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
            // Observe global changes to handle input subscriptions
            cx.observe(&global_handle, move |this: &mut Self, model, cx| {
                this.ensure_subscription(model, cx);
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

    /// Checks if there is an active input and ensures we are subscribed to its events.
    fn ensure_subscription(&mut self, model: Entity<Models>, cx: &mut Context<Self>) {
        // 1. 第一步：在一个独立的作用域内读取数据并克隆所需的实体
        // 这样做是为了让 model.read(cx) 产生的不可变借用在这里就结束
        let (active_input, edit_info) = {
            let model_read = model.read(cx);
            let state = &model_read.import_preview_state;

            if let Some(input_entity) = &state.active_input {
                let info = state.editing_cell.as_ref().map(|(r, k)| (*r, k.clone()));
                (Some(input_entity.clone()), info)
            } else {
                (None, None)
            }
        }; // <--- model_read 在这里被 Drop，cx 的借用被释放

        // 2. 第二步：使用克隆的数据进行订阅操作，此时 cx 可以被 mut 借用
        if let Some(input_entity) = active_input {
            let input_id = input_entity.entity_id();

            // 如果订阅的 ID 变了，或者之前没有订阅
            if self.subscribed_input_id != Some(input_id) {
                if let Some((row_ix, key)) = edit_info {
                    // 这里的 cx 是可用的
                    let sub =
                        cx.subscribe(&input_entity, move |this, state, event, cx| match event {
                            InputEvent::Change => {
                                let new_val = state.read(cx).value();
                                this.update_global_cell(cx, row_ix, &key, new_val.to_string());
                            }
                            InputEvent::PressEnter { .. } | InputEvent::Blur => {
                                this.finish_editing(cx);
                            }
                            _ => {}
                        });

                    self.input_subscription = Some(sub);
                    self.subscribed_input_id = Some(input_id);
                }
            }
        } else {
            // 没有活动的输入框，清理订阅
            if self.input_subscription.is_some() {
                self.input_subscription = None;
                self.subscribed_input_id = None;
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
            // Click mask to close/cancel (optional, currently just stops propagation)
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
                        div().flex_1().size_full().min_h(px(0.0)).child(
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

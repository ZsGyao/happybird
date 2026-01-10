use std::{cmp::Ordering, collections::HashMap};

use gpui::{
    App, AppContext, BorrowAppContext, ClickEvent, Context, Entity, Focusable, InteractiveElement,
    IntoElement, MouseButton, ParentElement, Render, SharedString, StatefulInteractiveElement,
    Styled, Subscription, Window, div, px,
};
use gpui_component::{
    ActiveTheme, Disableable, StyledExt,
    button::Button,
    checkbox::Checkbox,
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
    /// Whether in selection mode (show checkboxes)
    is_selection_mode: bool,
    /// Whether in edit mode (allow double click)
    is_edit_mode: bool,
}

impl PreviewDelegate {
    pub fn new(
        data_sample: Option<&HashMap<String, Value>>,
        is_selection_mode: bool,
        is_edit_mode: bool,
    ) -> Self {
        let mut raw_keys: Vec<String> = if let Some(row) = data_sample {
            row.keys().cloned().collect()
        } else {
            vec![]
        };

        // Sort: Name first, then alphabetical
        raw_keys.sort_by(|a, b| {
            if a == "姓名" {
                Ordering::Less
            } else if b == "姓名" {
                Ordering::Greater
            } else {
                a.cmp(b)
            }
        });

        let mut delegate = Self {
            headers: vec![],
            columns: vec![],
            is_selection_mode,
            is_edit_mode,
        };

        // Reuse the logic to build headers and columns
        delegate.rebuild_columns(raw_keys);
        delegate
    }

    /// Internal helper to rebuild columns based on keys and current mode
    fn rebuild_columns(&mut self, raw_keys: Vec<String>) {
        let mut headers = vec![];
        if self.is_selection_mode {
            headers.push("select".to_string());
        }
        headers.push("#".to_string());
        headers.extend(raw_keys);

        self.columns = headers
            .iter()
            .map(|col_name| {
                if col_name == "select" {
                    Column::new("select", "")
                        .width(px(40.0))
                        .fixed_left()
                        .movable(false)
                } else if col_name == "#" {
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
        self.headers = headers;
    }

    /// Update modes and refresh columns if necessary
    pub fn update_mode(&mut self, is_selection: bool, is_edit: bool) {
        self.is_edit_mode = is_edit;

        if self.is_selection_mode != is_selection {
            self.is_selection_mode = is_selection;

            // Extract raw keys from existing headers to rebuild
            // Filter out system columns "select" and "#"
            let raw_keys: Vec<String> = self
                .headers
                .iter()
                .filter(|h| *h != "select" && *h != "#")
                .cloned()
                .collect();

            self.rebuild_columns(raw_keys);
        }
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
        _window: &mut gpui::Window,
        cx: &mut Context<gpui_component::table::TableState<Self>>,
    ) -> impl IntoElement {
        let header_name = &self.headers[col_ix];
        let global_handle = cx.global::<GlobalAppState>().0.clone();

        // === Scenario 1: Selection Column (Checkbox) ===
        if self.is_selection_mode && col_ix == 0 {
            let is_selected = cx
                .global::<GlobalAppState>()
                .0
                .read(cx)
                .import_preview_state
                .selected_rows
                .contains(&row_ix);

            return div()
                .id(SharedString::from(format!(
                    "preview-chk-{}-{}",
                    row_ix, col_ix
                )))
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .id("row-checkbox")
                        .size(px(16.0))
                        .border_1()
                        .border_color(if is_selected {
                            cx.theme().colors.primary
                        } else {
                            cx.theme().colors.border
                        })
                        .bg(if is_selected {
                            cx.theme().colors.primary
                        } else {
                            gpui::transparent_black()
                        })
                        .rounded_sm()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(if is_selected {
                            div()
                                .text_size(px(10.0))
                                .text_color(gpui::white())
                                .child("✓")
                        } else {
                            div()
                        }),
                )
                // Closure signature to match gpui requirement: &mut App
                .on_click(move |_e, _window, cx| {
                    cx.stop_propagation();
                    global_handle.update(cx, |model, _| {
                        model.import_preview_state.toggle_row_selection(row_ix);
                    });
                })
                .into_any_element();
        }

        // === Scenario 2: Index Column (#) ===
        // Adjust index column position based on selection mode
        let is_index_col = if self.is_selection_mode {
            col_ix == 1
        } else {
            col_ix == 0
        };

        if is_index_col {
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

        // === Scenario 3: Data Column ===
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

        let is_editing = match &global_read.import_preview_state.editing_cell {
            Some((r, k)) => *r == row_ix && k == header_name,
            None => false,
        };

        if is_editing {
            if let Some(input_entity) = &global_read.import_preview_state.active_input {
                div()
                    .size_full()
                    .child(Input::new(&input_entity))
                    .into_any_element()
            } else {
                div().into_any_element()
            }
        } else {
            let header_clone = header_name.clone();
            let cell_value_clone = cell_value.clone();
            let can_edit = self.is_edit_mode;

            let mut cell = div().size_full().px(px(8.0)).flex().items_center().child(
                div()
                    .text_sm()
                    .text_color(cx.theme().colors.foreground)
                    .whitespace_nowrap()
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(cell_value),
            );

            // [Important]: This block attaches the event handler.
            // When `can_edit` becomes true, we need the Table to re-render this cell
            // to attach the listener.
            if can_edit {
                cell = cell.cursor(gpui::CursorStyle::IBeam).on_mouse_down(
                    MouseButton::Left,
                    move |e, window, cx| {
                        if e.click_count >= 2 {
                            cx.stop_propagation();
                            let input_entity = cx.new(|cx| {
                                let mut s = InputState::new(window, cx);
                                s.set_value(cell_value_clone.clone(), window, cx);
                                s.focus_handle(cx).focus(window);
                                s
                            });

                            global_handle.update(cx, |models, cx| {
                                models.import_preview_state.editing_cell =
                                    Some((row_ix, header_clone.clone()));
                                models.import_preview_state.active_input = Some(input_entity);
                                cx.notify();
                            });
                        }
                    },
                );
            }

            cell.into_any_element()
        }
    }
}

pub struct ImportPanel {
    table_state: Entity<TableState<PreviewDelegate>>,
    input_subscription: Option<Subscription>,
    subscribed_input_id: Option<gpui::EntityId>,

    last_selection_mode: bool,
    last_edit_mode: bool,
}

// =============================================================================
//  2. ImportPanel View
// =============================================================================

impl ImportPanel {
    pub fn new(window: &mut Window, cx: &mut App) -> Entity<Self> {
        let global_handle = cx.global::<GlobalAppState>().0.clone();

        let (first_row, is_sel, is_edit) = {
            let m = global_handle.read(cx);
            let s = &m.import_preview_state;
            (
                s.import_preview_data
                    .as_ref()
                    .and_then(|d| d.first())
                    .cloned(),
                s.is_selection_mode_enabled,
                s.is_edit_mode_enabled,
            )
        };

        let delegate = PreviewDelegate::new(first_row.as_ref(), is_sel, is_edit);
        let table_state = cx.new(|cx| TableState::new(delegate, window, cx));

        cx.new(|cx| {
            let global_obs = global_handle.clone();
            cx.observe(&global_obs, move |this: &mut Self, model, cx| {
                this.ensure_subscription(&model, cx);
                cx.notify();
            })
            .detach();

            Self {
                table_state,
                input_subscription: None,
                subscribed_input_id: None,
                last_selection_mode: is_sel,
                last_edit_mode: is_edit,
            }
        })
    }

    fn ensure_subscription(&mut self, model: &Entity<Models>, cx: &mut Context<Self>) {
        let (active_input, edit_info) = {
            let model_read = model.read(cx);
            let state = &model_read.import_preview_state;

            if let Some(input_entity) = &state.active_input {
                let info = state.editing_cell.as_ref().map(|(r, k)| (*r, k.clone()));
                (Some(input_entity.clone()), info)
            } else {
                (None, None)
            }
        };

        if let Some(input_entity) = active_input {
            let input_id = input_entity.entity_id();

            if self.subscribed_input_id != Some(input_id) {
                if let Some((row_ix, key)) = edit_info {
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

    fn render_toolbar_button(
        &self,
        id: &str, // Add explicit ID parameter
        label: &str,
        active: bool,
        icon_char: &str,
        cx: &Context<Self>,
        // Correct signature for button click handler: &mut App
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static + Send + Sync,
    ) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .id(SharedString::from(id.to_string())) // Explicitly set ID here
            .flex()
            .items_center()
            .gap(px(6.0))
            .px(px(10.0))
            .py(px(6.0))
            .rounded_md()
            .border_1()
            .cursor_pointer()
            .border_color(if active {
                theme.colors.primary
            } else {
                theme.colors.border
            })
            .bg(if active {
                theme.colors.primary.opacity(0.1)
            } else {
                theme.colors.background
            })
            .text_color(if active {
                theme.colors.primary
            } else {
                theme.colors.foreground
            })
            .hover(|s| s.bg(theme.colors.secondary))
            .on_click(on_click)
            .child(div().child(icon_char.to_string()))
            .child(div().text_sm().font_medium().child(label.to_string()))
    }
}

impl Render for ImportPanel {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) -> impl gpui::IntoElement {
        let global = cx.global::<GlobalAppState>().0.read(cx);
        let state = &global.import_preview_state;

        let row_count = state
            .import_preview_data
            .as_ref()
            .map(|d| d.len())
            .unwrap_or(0);
        let selected_count = state.selected_rows.len();
        let is_edit = state.is_edit_mode_enabled;
        let is_select = state.is_selection_mode_enabled;

        // [新增] 关键逻辑：检测模式是否改变，如果改变则重建 TableState
        if self.last_selection_mode != is_select || self.last_edit_mode != is_edit {
            let first_row = state.import_preview_data.as_ref().and_then(|d| d.first());

            // 1. 创建新的 Delegate
            let new_delegate = PreviewDelegate::new(first_row, is_select, is_edit);

            // 2. 创建新的 TableState 并替换旧的 (需要 window 参数)
            self.table_state = cx.new(|cx| TableState::new(new_delegate, window, cx));

            // 3. 更新缓存记录
            self.last_selection_mode = is_select;
            self.last_edit_mode = is_edit;
        }

        let import_label = if is_select && selected_count > 0 {
            format!("Import Selected ({})", selected_count)
        } else if is_select {
            "Select Rows...".to_string()
        } else {
            format!("Import All ({})", row_count)
        };

        let can_import = !is_select || selected_count > 0;

        div()
            .id("import-panel-overlay")
            .absolute()
            .size_full()
            .bg(gpui::black().opacity(0.5))
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                v_flex()
                    .w(px(950.0))
                    .h(px(650.0))
                    .bg(cx.theme().colors.background)
                    .border_1()
                    .border_color(cx.theme().colors.border)
                    .rounded_xl()
                    .shadow_xl()
                    .overflow_hidden()
                    // --- Header & Toolbar ---
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
                                    .child(div().text_xl().font_bold().child("Data Import"))
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().colors.secondary_foreground)
                                            .child(format!("{} records found", row_count)),
                                    ),
                            )
                            // --- Right Toolbar ---
                            .child(
                                h_flex()
                                    .gap(px(12.0))
                                    .items_center()
                                    .child(self.render_toolbar_button(
                                        "toggle-edit-mode-btn", // Unique ID
                                        if is_edit { "Editing" } else { "Read Only" },
                                        is_edit,
                                        if is_edit { "🔓" } else { "🔒" },
                                        cx,
                                        // Match signature &mut App
                                        |_, _, cx: &mut App| {
                                            cx.update_global::<GlobalAppState, _>(|app, cx| {
                                                app.0.update(cx, |m, _| {
                                                    m.import_preview_state.toggle_edit_mode()
                                                });
                                            });
                                        },
                                    ))
                                    .child(self.render_toolbar_button(
                                        "toggle-select-mode-btn", // Unique ID
                                        if is_select {
                                            "Selection On"
                                        } else {
                                            "Select Rows"
                                        },
                                        is_select,
                                        "☑️",
                                        cx,
                                        |_, _, cx: &mut App| {
                                            cx.update_global::<GlobalAppState, _>(|app, cx| {
                                                app.0.update(cx, |m, _| {
                                                    m.import_preview_state.toggle_selection_mode()
                                                });
                                            });
                                        },
                                    )),
                            ),
                    )
                    // --- Table Area ---
                    .child(
                        div()
                            .flex_1()
                            .size_full()
                            .min_h(px(0.0))
                            // [FIX]: Wrap Table in a div with a dynamic ID.
                            // This replaces which isn't available on Table struct.
                            // Changing the ID forces gpui to replace the element, triggering a full re-render.
                            .child(
                                div()
                                    .size_full()
                                    .id(SharedString::from(format!(
                                        "table-mode-{}-{}",
                                        is_select, is_edit
                                    )))
                                    .child(
                                        Table::new(&self.table_state)
                                            .stripe(true)
                                            .bordered(true)
                                            .scrollbar_visible(true, true),
                                    ),
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
                            .child(if is_select {
                                div().mr(px(12.0)).child(
                                    Button::new("select-all").label("Toggle All").on_click(
                                        move |_, _, cx: &mut App| {
                                            cx.update_global::<GlobalAppState, _>(|app, cx| {
                                                app.0.update(cx, |m, _| {
                                                    m.import_preview_state
                                                        .toggle_select_all(row_count)
                                                });
                                            });
                                        },
                                    ),
                                )
                            } else {
                                div()
                            })
                            .child(Button::new("cancel-import").label("Cancel").on_click(
                                |_, _, cx: &mut App| {
                                    let global = cx.global::<GlobalAppState>().0.clone();
                                    global.update(cx, |m, cx| m.cancel_import(cx));
                                },
                            ))
                            .child(
                                Button::new("confirm-import")
                                    .label(import_label)
                                    .disabled(!can_import)
                                    .on_click(|_, _, cx: &mut App| {
                                        let global = cx.global::<GlobalAppState>().0.clone();
                                        global.update(cx, |m, cx| m.confirm_import(cx));
                                    }),
                            ),
                    ),
            )
    }
}

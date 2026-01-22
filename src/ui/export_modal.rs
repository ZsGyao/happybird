// src/ui/export_modal.rs

use crate::ui::{
    models::{ExportScope, GlobalAppState},
    theme::infra::hb_icons::HappyBirdIcons,
};
use gpui::{prelude::FluentBuilder, *};
use gpui_component::{
    ActiveTheme, Disableable, Icon, IconName, Sizable,
    button::{Button, ButtonVariants},
    checkbox::Checkbox,
    h_flex,
    label::Label,
    v_flex,
};
use std::path::Path;

pub struct ExportModal;

impl ExportModal {
    /// 打开导出弹窗
    /// 如果是从批量操作栏进来的，会自动选中 SelectedItems 模式
    pub fn toggle(cx: &mut App) {
        // 安全检查：确保 GlobalAppState 存在
        if !cx.has_global::<GlobalAppState>() {
            return;
        }
        let global = cx.global::<GlobalAppState>().0.clone();

        global.update(cx, |model, cx| {
            if model.export_state.is_open {
                model.export_state.close();
            } else {
                let fields = model.dynamic_headers.clone();
                let has_selection = model.multi_selection.is_selection_mode();
                model.export_state.open(fields, has_selection);
            }
            cx.notify();
        });
    }

    /// 执行导出流程
    fn confirm_export(_window: &mut Window, cx: &mut App) {
        let global = cx.global::<GlobalAppState>().0.clone();

        // 1. 提取当前快照数据 (Snapshot)
        let (scope, fields, db, search_query, selected_ids) = global.update(cx, |m, _| {
            let f: Vec<String> = m
                .dynamic_headers
                .iter()
                .filter(|k| m.export_state.selected_fields.contains(*k))
                .cloned()
                .collect();
            let ids: Vec<i32> = m.multi_selection.selected_ids.iter().cloned().collect();
            (
                m.export_state.scope,
                f,
                m.get_db_manager(),
                m.search_query.clone(),
                ids,
            )
        });

        if fields.is_empty() {
            // TODO: Toast "Please select columns"
            return;
        }

        // 2. 调用原生系统保存对话框 (Save As)
        // 参数1: 初始目录 (这里设为当前目录 ".")
        // 参数2: 建议的文件名
        let save_future = cx.prompt_for_new_path(Path::new("."), Some("export_data.xlsx"));

        cx.spawn(async move |cx| {
            // 等待用户在系统弹窗中操作
            // save_future.await 返回 Result<Result<Option<PathBuf>>>
            if let Ok(Ok(Some(path))) = save_future.await {
                // 3. 用户确认保存，设置 Loading 状态
                let _ = cx.update(|cx| {
                    if !cx.has_global::<GlobalAppState>() {
                        return;
                    }
                    let global = cx.global::<GlobalAppState>().0.clone();
                    global.update(cx, |m, cx| {
                        m.export_state.is_exporting = true;
                        cx.notify();
                    })
                });

                // 4. 后台执行导出任务
                let result = cx
                    .background_executor()
                    .spawn(async move {
                        // 根据 scope 获取数据
                        let subjects = if let Ok(conn) = db.get_conn() {
                            match scope {
                                ExportScope::AllData => {
                                    crate::backend::db::ops::DataService::get_all_subjects(&conn)
                                        .unwrap_or_default()
                                }
                                ExportScope::CurrentSearch => {
                                    crate::backend::db::ops::DataService::search_subjects(
                                        &conn,
                                        Some(&search_query),
                                        1,
                                        100000, // this is assume the search data will not beyond 100000, like get all search
                                    )
                                    .unwrap_or_default()
                                }
                                ExportScope::SelectedItems => {
                                    crate::backend::db::ops::DataService::get_subjects_by_ids(
                                        &conn,
                                        &selected_ids,
                                    )
                                    .unwrap_or_default()
                                }
                            }
                        } else {
                            vec![]
                        };

                        // 写入 Excel 到用户选择的 path
                        crate::backend::file::exporter::ExcelExporter::export(
                            path, subjects, fields,
                        )
                    })
                    .await;

                // 5. 完成回调
                let _ = cx.update(|cx| {
                    if !cx.has_global::<GlobalAppState>() {
                        return;
                    }
                    let global = cx.global::<GlobalAppState>().0.clone();
                    global.update(cx, |m, cx| {
                        m.export_state.is_exporting = false;
                        if result.is_ok() {
                            m.export_state.close();
                        } else {
                            eprintln!("Export failed: {:?}", result.err());
                        }
                        cx.notify();
                    })
                });
            }
            // 如果用户点了取消 (Ok(Ok(None)))，什么都不做
        })
        .detach();
    }
}

// 辅助：渲染单选框选项
fn render_scope_option(
    scope: ExportScope,
    title: &str,
    desc: &str,
    current: ExportScope,
    global: Entity<crate::ui::models::Models>,
    cx: &mut App,
) -> impl IntoElement {
    let selected = scope == current;
    let theme = cx.theme();
    let global_click = global.clone();
    let title_owned = SharedString::from(title.to_string());
    let desc_owned = SharedString::from(desc.to_string());

    let element_id = match scope {
        ExportScope::AllData => "scope-all",
        ExportScope::CurrentSearch => "scope-search",
        ExportScope::SelectedItems => "scope-selected",
    };

    div()
        .id(element_id)
        .flex()
        .gap(px(8.0))
        .cursor_pointer()
        .p(px(8.0))
        .rounded_md()
        .bg(if selected {
            theme.colors.secondary.opacity(0.2)
        } else {
            gpui::transparent_black()
        })
        // 修复：on_click 闭包需要接收 event, window, app
        .on_click(move |_, _, cx| {
            global_click.update(cx, |m, _| m.export_state.scope = scope);
        })
        .child(
            div()
                .mt(px(2.0))
                .size(px(16.0))
                .rounded_full()
                .border_1()
                .border_color(if selected {
                    theme.colors.primary
                } else {
                    theme.colors.muted_foreground
                })
                .flex()
                .items_center()
                .justify_center()
                .child(if selected {
                    div()
                        .size(px(8.0))
                        .rounded_full()
                        .bg(theme.colors.primary)
                        .into_any_element()
                } else {
                    div().into_any_element()
                }),
        )
        .child(
            v_flex()
                .child(Label::new(title_owned).font_weight(if selected {
                    FontWeight::BOLD
                } else {
                    FontWeight::NORMAL
                }))
                .child(
                    Label::new(desc_owned)
                        .text_xs()
                        .text_color(theme.colors.muted_foreground),
                ),
        )
}

/// 渲染模态框主体
pub fn render_export_modal(cx: &mut App) -> Option<AnyElement> {
    if !cx.has_global::<GlobalAppState>() {
        return None;
    }

    // [Phase 1: Data Extraction]
    // 在这个块中获取数据并 Clone，块结束后自动释放 borrow
    let (_is_open, scope, all_fields, selected_fields, is_exporting, total_count, selected_count) = {
        let global_read = cx.global::<GlobalAppState>().0.read(cx);
        if !global_read.export_state.is_open {
            return None;
        }
        let state = &global_read.export_state;
        (
            state.is_open,
            state.scope,
            state.all_fields.clone(),
            state.selected_fields.clone(),
            state.is_exporting,
            global_read.total_count,
            global_read.multi_selection.selected_ids.len(),
        )
    };

    // [Phase 2: UI Building]
    // 此时 cx 没有被借用，可以安全传入 render_scope_option

    let global_handle = cx.global::<GlobalAppState>().0.clone();

    Some(
        div()
            .absolute()
            .size_full()
            .bg(gpui::black().opacity(0.5))
            .flex()
            .items_center()
            .justify_center()
            .child(
                v_flex()
                    .w(px(500.0))
                    .max_h(px(700.0))
                    .bg(cx.theme().colors.background)
                    .border_1()
                    .border_color(cx.theme().colors.border)
                    .rounded_xl()
                    .shadow_lg()
                    // Header
                    .child(
                        h_flex()
                            .p(px(20.0))
                            .border_b_1()
                            .border_color(cx.theme().colors.border)
                            .justify_between()
                            .child(
                                Label::new("Export Data")
                                    .font_weight(FontWeight::BOLD)
                                    .text_lg(),
                            )
                            .child(
                                Button::new("close-btn")
                                    .icon(Icon::new(HappyBirdIcons::SquareX.load(cx)))
                                    .cursor_pointer()
                                    .on_click(|_, _, cx| ExportModal::toggle(cx)),
                            ),
                    )
                    // Body
                    .child(
                        v_flex()
                            .id("exp-body")
                            .p(px(20.0))
                            .gap(px(24.0))
                            .overflow_y_scroll()
                            // 1. Data Range
                            .child(
                                v_flex()
                                    .gap(px(8.0))
                                    .child(
                                        Label::new("1. Data Range").font_weight(FontWeight::BOLD),
                                    )
                                    .child(
                                        v_flex()
                                            .gap(px(8.0))
                                            .pl(px(8.0))
                                            .child(render_scope_option(
                                                ExportScope::AllData,
                                                "All Data",
                                                "Export everything in the database.",
                                                scope, // Use cloned scope
                                                global_handle.clone(),
                                                cx,
                                            ))
                                            .child(render_scope_option(
                                                ExportScope::CurrentSearch,
                                                "Current Search",
                                                &format!(
                                                    "{} records match your filters.",
                                                    total_count
                                                ),
                                                scope, // Use cloned scope
                                                global_handle.clone(),
                                                cx,
                                            ))
                                            .child(
                                                div()
                                                    .id("select-it")
                                                    .map(|this| {
                                                        if selected_count == 0 {
                                                            this.opacity(0.5).cursor_not_allowed()
                                                        } else {
                                                            this
                                                        }
                                                    })
                                                    .child(render_scope_option(
                                                        ExportScope::SelectedItems,
                                                        "Selected Items",
                                                        &format!(
                                                            "{} records selected",
                                                            selected_count
                                                        ),
                                                        scope, // Use cloned scope
                                                        global_handle.clone(),
                                                        cx,
                                                    ))
                                                    .on_click(move |_, _, _| {
                                                        if selected_count == 0 { /* Block click */ }
                                                    }),
                                            ),
                                    ),
                            )
                            // 2. Columns
                            .child(
                                v_flex()
                                    .gap(px(8.0))
                                    .child(
                                        h_flex()
                                            .justify_between()
                                            .child(
                                                Label::new("2. Columns")
                                                    .font_weight(FontWeight::BOLD),
                                            )
                                            .child(
                                                Button::new("select-all")
                                                    .label("Select All")
                                                    .small()
                                                    .ghost()
                                                    .on_click(|_, _, cx| {
                                                        if !cx.has_global::<GlobalAppState>() {
                                                            return;
                                                        }
                                                        let global =
                                                            cx.global::<GlobalAppState>().0.clone();
                                                        global.update(cx, |m, _| {
                                                            m.export_state.select_all()
                                                        });
                                                    }),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .id("chk-box")
                                            .border_1()
                                            .border_color(cx.theme().colors.border)
                                            .rounded_md()
                                            .p(px(12.0))
                                            .h(px(200.0))
                                            .overflow_y_scroll()
                                            .bg(cx.theme().colors.secondary.opacity(0.1))
                                            .child(v_flex().gap(px(8.0)).children(
                                                all_fields.iter().map(|f| {
                                                    // Use cloned all_fields
                                                    let f_clone = f.clone();
                                                    let is_checked = selected_fields.contains(f); // Use cloned selected_fields
                                                    let handle = global_handle.clone();
                                                    h_flex()
                                                        .gap(px(8.0))
                                                        .items_center()
                                                        .child(
                                                            Checkbox::new(SharedString::from(
                                                                f.clone(),
                                                            ))
                                                            .checked(is_checked)
                                                            .on_click(move |_, _, cx| {
                                                                handle.update(cx, |m, _| {
                                                                    m.export_state
                                                                        .toggle_field(&f_clone)
                                                                });
                                                            }),
                                                        )
                                                        .child(Label::new(f.clone()).text_sm())
                                                }),
                                            )),
                                    ),
                            ),
                    )
                    // Footer
                    .child(
                        h_flex()
                            .p(px(20.0))
                            .border_t_1()
                            .border_color(cx.theme().colors.border)
                            .justify_end()
                            .gap(px(12.0))
                            .child(
                                Button::new("cancel")
                                    .label("Cancel")
                                    .ghost()
                                    .on_click(|_, _, cx| ExportModal::toggle(cx)),
                            )
                            .child(
                                Button::new("confirm")
                                    .label(if is_exporting {
                                        // Use extracted bool
                                        "Exporting..."
                                    } else {
                                        "Export"
                                    })
                                    .icon(if is_exporting {
                                        IconName::Loader
                                    } else {
                                        IconName::File
                                    })
                                    .primary()
                                    .disabled(
                                        is_exporting || selected_fields.is_empty(), // Use extracted data
                                    )
                                    .on_click(|_, w, cx| ExportModal::confirm_export(w, cx)),
                            ),
                    ),
            )
            .into_any_element(),
    )
}

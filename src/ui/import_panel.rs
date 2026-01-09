use std::{cmp::Ordering, collections::HashMap};

use gpui::{
    App, AppContext, Context, Entity, InteractiveElement, IntoElement, MouseButton, ParentElement,
    Render, Styled, Window, div, px,
};
use gpui_component::{
    ActiveTheme, StyledExt,
    button::Button,
    h_flex,
    scroll::ScrollableElement,
    table::{Column, ColumnSort, Table, TableDelegate, TableState},
    v_flex,
};
use serde_json::Value;

use crate::ui::models::GlobalAppState;

// =============================================================================
//  1. Table Delegate (数据适配器)
//  负责告诉表格：有多少数据、表头是什么、怎么排序、每个格怎么画
// =============================================================================

#[derive(Debug)]
pub struct PreviewDelegate {
    /// 缓存排好序的表头（列定义）
    headers: Vec<String>,
    /// 存储 UI 所需的 Column 对象
    columns: Vec<Column>,
    /// 数据源（本地持有一份拷贝，以便支持前端排序而不影响原始数据）
    data: Vec<HashMap<String, Value>>,
}

impl PreviewDelegate {
    pub fn new(data: Vec<HashMap<String, Value>>) -> Self {
        // 1. 提取表头 (取第一行的 keys)
        let mut headers: Vec<String> = if let Some(first) = data.first() {
            first.keys().cloned().collect()
        } else {
            vec![]
        };

        // 2. 默认表头排序策略：强制 "姓名" 排第一，其他按字典序
        headers.sort_by(|a, b| {
            if a == "姓名" {
                Ordering::Less
            } else if b == "姓名" {
                Ordering::Greater
            } else {
                a.cmp(b)
            }
        });

        // 3. 构建 Column 对象
        let columns = headers
            .iter()
            .map(|key| {
                if key == "姓名" {
                    Column::new(key.clone(), key.clone()) // 使用 key 作为 ID 和 Title
                        .width(px(120.0))
                        .fixed_left() // 姓名列固定在左侧
                } else {
                    Column::new(key.clone(), key.clone()) // 使用 key 作为 ID 和 Title
                        .width(px(120.0)) // 这里定义列宽
                }
            })
            .collect();

        Self {
            headers,
            columns,
            data,
        }
    }

    /// 辅助函数：将 JSON Value 转为适合显示的 String
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

    fn rows_count(&self, _cx: &App) -> usize {
        self.data.len()
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
        // 1. 根据列索引获取对应的 Key (例如 "姓名", "年龄")
        let key = &self.headers[col_ix];

        // 2. 根据行索引获取对应的数据行 (HashMap)
        let row = &self.data[row_ix];

        // 3. 提取值并格式化为字符串
        // (调用我们在 PreviewDelegate 中定义的 format_value 辅助函数)
        let text = Self::format_value(row.get(key));

        // 4. 构建单元格 UI
        div()
            .size_full() // 撑满整个单元格空间
            .px(px(8.0)) // 左右内边距
            .flex()
            .items_center() // 垂直居中
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().colors.foreground)
                    .whitespace_nowrap() // 核心：单行显示
                    .overflow_hidden() // 核心：防止撑破表格
                    .text_ellipsis() // 核心：超长自动显示省略号 "..."
                    .child(text),
            )
    }

    // 实现列排序
    fn perform_sort(
        &mut self,
        col_ix: usize,
        sort: ColumnSort,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) {
        let key = &self.headers[col_ix];
        let cmp = |a: &HashMap<String, Value>, b: &HashMap<String, Value>| {
            let va = Self::format_value(a.get(key));
            let vb = Self::format_value(b.get(key));
            if let (Ok(na), Ok(nb)) = (va.parse::<f64>(), vb.parse::<f64>()) {
                na.partial_cmp(&nb).unwrap_or(Ordering::Equal)
            } else {
                va.cmp(&vb)
            }
        };

        match sort {
            ColumnSort::Ascending => self.data.sort_by(cmp),
            ColumnSort::Descending => self.data.sort_by(|a, b| cmp(b, a)),
            ColumnSort::Default => {}
        }
    }
}

pub struct ImportPanel {
    table_state: Entity<TableState<PreviewDelegate>>,
    data_count: usize,
}

impl ImportPanel {
    pub fn new(window: &mut Window, cx: &mut App) -> Entity<Self> {
        // 1. 从 Models 读取原始数据
        let global = cx.global::<GlobalAppState>().0.read(cx);
        // 这里 Clone 一份数据给 UI 用于展示和排序，原始数据保留在 Models 里用于入库
        let raw_data = global.import_preview_data.clone().unwrap_or_default();
        let count = raw_data.len();

        // 2. 创建适配器
        let delegate = PreviewDelegate::new(raw_data);

        // 3. 创建 Table 状态
        let table_state = cx.new(|cx| TableState::new(delegate, window, cx));

        cx.new(|_cx| Self {
            table_state,
            data_count: count,
        })
    }
}

impl Render for ImportPanel {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) -> impl gpui::IntoElement {
        // --- 构建 UI ---
        div()
            .id("import-panel-overlay")
            .absolute()
            .size_full()
            .bg(gpui::black().opacity(0.5))
            .flex()
            .items_center()
            .justify_center()
            // ❌ 移除最外层的 stop_propagation，因为它可能误杀内部按钮的事件状态
            // .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                // 弹窗卡片主体
                v_flex()
                    .w(px(900.0))
                    .h(px(600.0))
                    .bg(cx.theme().colors.background)
                    .border_1()
                    .border_color(cx.theme().colors.border)
                    .rounded_xl()
                    .shadow_xl()
                    // .overflow_hidden()
                    // === A. Header (顶部固定) ===
                    .child(
                        h_flex()
                            .flex_shrink_0()
                            .justify_between()
                            .items_center()
                            .px(px(20.0))
                            .py(px(16.0))
                            .border_b_1()
                            .border_color(cx.theme().colors.border)
                            .p(px(20.0))
                            .gap(px(16.0))
                            .child(
                                h_flex()
                                    .justify_between()
                                    .items_baseline()
                                    .child(div().text_xl().font_bold().child("Preview Data"))
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().colors.secondary_foreground)
                                            .child(format!("{} rows ready", self.data_count)),
                                    ),
                            ),
                    )
                    // === B. Table Area (中间弹性伸缩) ===
                    .child(
                        div()
                            .flex_1() // 占据剩余空间
                            .size_full() // 宽/高撑满
                            .bg(gpui::red())
                            .min_h(px(0.0)) // 🔥 核心修复：强制限制 flex 内部高度，防止表格溢出覆盖底部
                            .child(
                                // Table 组件
                                Table::new(&self.table_state),
                            ),
                    )
                    // === C. Footer Buttons (底部固定) ===
                    .child(
                        h_flex()
                            .flex_shrink_0() // 防止被压缩
                            .justify_end()
                            .items_center()
                            .px(px(20.0))
                            .py(px(16.0)) // 给足够的点击区域
                            .gap(px(12.0))
                            .border_t_1()
                            .border_color(cx.theme().colors.border)
                            .bg(cx.theme().colors.background) // 确保背景不透明
                            .child(Button::new("cancel-import").label("Cancel").on_click(
                                |_, _, cx| {
                                    println!("***** Cancel Clicked!"); // 现在这里应该会打印了
                                    let global = cx.global::<GlobalAppState>().0.clone();
                                    global.update(cx, |m, cx| m.cancel_import(cx));
                                },
                            ))
                            .child(
                                Button::new("confirm-import")
                                    .label("Confirm & Import")
                                    .on_click(|_, _, cx| {
                                        println!("@@@@@ Confirm Clicked!");
                                        let global = cx.global::<GlobalAppState>().0.clone();
                                        global.update(cx, |m, cx| m.confirm_import(cx));
                                    }),
                            ),
                    ),
            )
    }
}

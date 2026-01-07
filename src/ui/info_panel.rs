use chrono::{DateTime, Local};
use gpui::*;

use gpui_component::{
    ActiveTheme, StyledExt,
    button::Button,
    h_flex,
    resizable::{resizable_panel, v_resizable},
    scroll::ScrollableElement,
    v_flex,
};
use rand::Rng;
use smallvec::SmallVec;

use crate::ui::{
    indent_guides::{IndentGuideColors, RenderedIndentGuide, indent_guides},
    search::SearchPanel,
};

#[derive(Clone, Debug)]
pub struct FileEntry {
    pub id: usize,
    pub name: String,
    pub depth: usize,
    pub is_dir: bool,
    pub is_selected: bool,
}

pub struct InfoPanel {
    entries: Vec<FileEntry>,
    scroll_handle: UniformListScrollHandle,
    focus_handle: FocusHandle,
    selected_idx: Option<usize>,
    search: Entity<SearchPanel>,
    import_history: Vec<Entity<HistoryImportItem>>,
}

#[derive(Clone)]
pub struct HistoryImportItem {
    import_name: String,
    import_status: bool,
    import_time: DateTime<Local>,
    import_desc: String,
}

impl HistoryImportItem {
    fn generate_dummy_data(cx: &mut App) -> Vec<Entity<Self>> {
        let mut rng = rand::rng();
        (0..20)
            .map(|i| {
                let item = HistoryImportItem {
                    import_name: format!("Import {}", i + 1),
                    import_status: if i % 2 == 0 { true } else { false },
                    import_time: Local::now() - chrono::Duration::days(rng.gen_range(0..365)),
                    import_desc: format!("Description for import {}.", i + 1),
                };
                cx.new(|_cx| item)
            })
            .collect()
    }
}

impl Render for HistoryImportItem {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 定义颜色（根据你的图片取色，或者使用 theme）
        let pink_color = rgb(0xd946ef); // 类似图中的粉紫色
        let green_color = rgb(0x22c55e); // 绿色
        let dot_color = if self.import_status {
            green_color
        } else {
            pink_color
        };

        let line_color = gpui::hsla(0.0, 0.0, 1.0, 0.1);
        let tag_bg_color = gpui::rgba(0x8b5cf6);
        let tag_border_color = rgb(0x8b5cf6);

        div()
            .flex()
            .flex_row() // 整体是水平布局：左边是线/点，右边是文字
            .w_full()
            .gap_3() // 点和文字之间的间距
            .child(
                // === 左侧：时间轴 (点 + 线) ===
                v_flex()
                    .h_full() // 撑满高度，让线能连起来
                    .items_center() // 居中对齐
                    .child(
                        // 1. 圆点
                        div()
                            .mt(px(6.0)) // 稍微往下顶一点，为了对齐第一行文字的中心
                            .size(px(8.0)) // 圆点大小
                            .rounded_full()
                            .bg(dot_color),
                    )
                    .child(
                        // 2. 垂直连线
                        div()
                            .w(px(1.0)) // 线宽 1px
                            .flex_1() // 占据剩余高度，形成连接下个item的效果
                            .mt(px(4.0)) // 点和线之间留一点点缝隙
                            .bg(line_color), // 淡淡的灰色线条
                    ),
            )
            .child(
                // === 右侧：主要内容 ===
                v_flex()
                    .flex_1()
                    .pb(px(16.0)) // 每个 Item 底部留白
                    .gap_1() // 行间距
                    // 第一行：标题 + 时间
                    .child(
                        h_flex()
                            .justify_between()
                            .items_start()
                            .child(
                                div()
                                    .font_bold()
                                    .text_sm()
                                    .text_color(gpui::white())
                                    .child(self.import_name.clone()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_bold() // 时间通常用等宽字体好看点
                                    .text_color(cx.theme().colors.blue)
                                    .child(self.import_time.format("%D-%H:%M").to_string()),
                            ),
                    )
                    // 第二行：描述
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().colors.blue)
                            .child(self.import_desc.clone()),
                    )
                    // 第三行：Tag (例如 "+ Hobby")
                    .child(
                        div()
                            .flex() // 包裹一层 flex 为了不让 tag 占满整行宽度
                            .child(
                                div()
                                    .mt(px(4.0))
                                    .px(px(8.0))
                                    .py(px(2.0))
                                    .rounded_md()
                                    .border_1()
                                    // 这里模拟图中那个紫色的 tag 样式
                                    .border_color(rgb(0x8b5cf6))
                                    .bg(rgb(0x8b5cf6))
                                    .text_xs()
                                    .font_medium()
                                    .text_color(rgb(0xc4b5fd))
                                    .child("+ Hobby"), // 如果这是动态的，请换成 self.tag_name
                            ),
                    ),
            )
    }
}

impl Focusable for InfoPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl InfoPanel {
    pub fn new(window: &mut Window, cx: &mut App) -> Entity<Self> {
        // 生成测试数据
        let entries = Self::generate_dummy_data();

        cx.new(|cx| InfoPanel {
            entries,
            scroll_handle: UniformListScrollHandle::new(),
            focus_handle: cx.focus_handle(),
            selected_idx: None,
            search: SearchPanel::new(window, cx),
            import_history: HistoryImportItem::generate_dummy_data(cx),
        })
    }

    // 生成一些看起来像树结构的扁平数据
    fn generate_dummy_data() -> Vec<FileEntry> {
        vec![
            FileEntry {
                id: 1,
                name: "src".into(),
                depth: 0,
                is_dir: true,
                is_selected: false,
            },
            FileEntry {
                id: 2,
                name: "components".into(),
                depth: 1,
                is_dir: true,
                is_selected: false,
            },
            FileEntry {
                id: 3,
                name: "button.rs".into(),
                depth: 2,
                is_dir: false,
                is_selected: false,
            },
            FileEntry {
                id: 4,
                name: "list.rs".into(),
                depth: 2,
                is_dir: false,
                is_selected: false,
            },
            FileEntry {
                id: 5,
                name: "panel.rs".into(),
                depth: 2,
                is_dir: false,
                is_selected: false,
            },
            FileEntry {
                id: 6,
                name: "utils".into(),
                depth: 1,
                is_dir: true,
                is_selected: false,
            },
            FileEntry {
                id: 7,
                name: "format.rs".into(),
                depth: 2,
                is_dir: false,
                is_selected: false,
            },
            FileEntry {
                id: 8,
                name: "lib.rs".into(),
                depth: 1,
                is_dir: false,
                is_selected: false,
            },
            FileEntry {
                id: 9,
                name: "main.rs".into(),
                depth: 1,
                is_dir: false,
                is_selected: false,
            },
            FileEntry {
                id: 10,
                name: "Cargo.toml".into(),
                depth: 0,
                is_dir: false,
                is_selected: false,
            },
            FileEntry {
                id: 11,
                name: "README.md".into(),
                depth: 0,
                is_dir: false,
                is_selected: false,
            },
            FileEntry {
                id: 12,
                name: "target".into(),
                depth: 0,
                is_dir: true,
                is_selected: false,
            },
            FileEntry {
                id: 13,
                name: "debug".into(),
                depth: 1,
                is_dir: true,
                is_selected: false,
            },
            FileEntry {
                id: 14,
                name: "deps".into(),
                depth: 2,
                is_dir: true,
                is_selected: false,
            },
            FileEntry {
                id: 15,
                name: "lib-123.rlib".into(),
                depth: 3,
                is_dir: false,
                is_selected: false,
            },
        ]
    }

    fn select_item(&mut self, idx: usize, cx: &mut Context<Self>) {
        self.selected_idx = Some(idx);
        for (i, entry) in self.entries.iter_mut().enumerate() {
            entry.is_selected = i == idx;
        }
        cx.notify();
    }
}

impl Render for FileEntry {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .text_center()
            .child(format!("{} -- {}", self.id, self.name))
    }
}

impl Render for InfoPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entries = self.entries.clone();
        let item_count = entries.len();

        div()
            .v_flex() // Correct root element
            .id("info-panel")
            .p(px(13.0))
            .size_full()
            .relative()
            .track_focus(&self.focus_handle(cx))
            .gap(px(8.0)) // Use gaps for spacing
            .child(div().w_full().child(self.search.clone())) // Search bar at top
            .child(div().font_semibold().child("EXPLORER")) // Explorer label
            .child(
                // Main content area
                div().flex_1().w_full().overflow_hidden().child(
                    // Use flex_1 to fill space, h_full to prevent collapse
                    v_resizable("left-dock")
                        .child(
                            resizable_panel().child(
                                uniform_list("entries", item_count, move |range, _, cx| {
                                    range
                                        .map(|ix| {
                                            let item = &entries[ix];
                                            div()
                                                .h(px(24.0))
                                                .flex()
                                                .items_center()
                                                .pl(px(20.0) * item.depth as f32)
                                                .bg(if item.is_selected {
                                                    cx.theme().colors.selection
                                                } else {
                                                    cx.theme().colors.list
                                                })
                                                .child(format!(
                                                    "{} {}",
                                                    if item.is_dir { "📁" } else { "📄" },
                                                    item.name
                                                ))
                                        })
                                        .collect::<Vec<_>>()
                                })
                                .size_full()
                                .with_decoration(
                                    indent_guides(px(14.0), IndentGuideColors::panel(cx))
                                        .with_compute_indents_fn(
                                            cx.entity(),
                                            |this, range, _window, _cx| {
                                                let mut depths = SmallVec::with_capacity(
                                                    range.end - range.start,
                                                );
                                                for i in range {
                                                    if let Some(entry) = this.entries.get(i) {
                                                        depths.push(entry.depth);
                                                    }
                                                }
                                                depths
                                            },
                                        )
                                        .with_render_fn(
                                            cx.entity(),
                                            move |_this, params, _, _cx| {
                                                const LEFT_OFFSET: Pixels = px(14.);
                                                const PADDING_Y: Pixels = px(4.);
                                                const HITBOX_OVERDRAW: Pixels = px(3.);

                                                let indent_size = params.indent_size;
                                                let item_height = params.item_height;
                                                let active_indent_guide_index = None;

                                                params
                                                    .indent_guides
                                                    .into_iter()
                                                    .enumerate()
                                                    .map(|(idx, layout)| {
                                                        let offset = if layout.continues_offscreen {
                                                            px(0.)
                                                        } else {
                                                            PADDING_Y
                                                        };
                                                        let bounds = Bounds::new(
                                                            point(
                                                                layout.offset.x * indent_size
                                                                    + LEFT_OFFSET,
                                                                layout.offset.y * item_height
                                                                    + offset,
                                                            ),
                                                            size(
                                                                px(1.),
                                                                layout.length * item_height
                                                                    - offset * 2.,
                                                            ),
                                                        );
                                                        RenderedIndentGuide {
                                                            bounds,
                                                            layout,
                                                            is_active: active_indent_guide_index
                                                                == Some(idx),
                                                            hitbox: Some(Bounds::new(
                                                                point(
                                                                    bounds.origin.x
                                                                        - HITBOX_OVERDRAW,
                                                                    bounds.origin.y,
                                                                ),
                                                                size(
                                                                    bounds.size.width
                                                                        + HITBOX_OVERDRAW * 2.,
                                                                    bounds.size.height,
                                                                ),
                                                            )),
                                                        }
                                                    })
                                                    .collect()
                                            },
                                        ),
                                ),
                            ),
                        )
                        .child(
                            resizable_panel()
                                .size(px(160.0))
                                .size_range(px(100.0)..px(500.0))
                                .child(
                                    v_flex()
                                        .size_full()
                                        .gap_2() // 标题和列表之间的间距
                                        // 1. 添加缺失的标题栏
                                        .child(
                                            div()
                                                .flex()
                                                .justify_between()
                                                .items_center()
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .font_bold()
                                                        .text_color(cx.theme().colors.blue_light) // 灰色文字
                                                        .child("Import Activity"),
                                                )
                                                // 如果需要右侧那个刷新/历史图标，可以在这里加
                                                .child(
                                                    div()
                                                        .child("↺")
                                                        .text_color(cx.theme().colors.blue_light),
                                                ),
                                        )
                                        // 2. 列表区域
                                        .child(
                                            v_flex()
                                                .flex_1() // 列表占据剩余空间
                                                .overflow_y_scrollbar()
                                                .pb(px(16.0)) // <--- 【关键修改】添加底部内边距，防止最后一个 Item 贴底或被切断
                                                .children(self.import_history.clone()),
                                        ),
                                ),
                        ),
                ),
            )
            .child(
                // Bottom panel
                div()
                    // 不需要固定高度 h(px(120.0))，让内容撑开即可，或者用 mt_auto 推到底部
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap_2() // 上下两行按钮之间的间距
                    .pt(px(8.0)) // 给上面留一点呼吸空间
                    .border_t_1() // 可选：顶部加一条分割线
                    .border_color(cx.theme().colors.blue_light)
                    // 第一行：Import 大按钮
                    .child(
                        Button::new("Import-button")
                            .w_full() // <--- 关键：填满宽度
                            .label("Import New Data")
                            .on_click(|_, _, _| println!("Import New Data")),
                    )
                    // 第二行：Export 和 Config 并排
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .w_full()
                            .h(px(75.0))
                            .gap_2() // 两个按钮中间的缝隙
                            // 左侧：Export (包裹在 flex_1 中以占据 50%)
                            .child(
                                div()
                                    .flex_1() // <--- 关键：均分空间
                                    .child(
                                        Button::new("Export-button")
                                            .w_full() // 按钮填满这个 flex_1 容器
                                            .label("Export")
                                            .on_click(|_, _, _| println!("Export")),
                                    ),
                            )
                            // 右侧：Config (包裹在 flex_1 中以占据 50%)
                            .child(
                                div()
                                    .flex_1() // <--- 关键：均分空间
                                    .child(
                                        Button::new("Config-button")
                                            .w_full() // 按钮填满这个 flex_1 容器
                                            .label("Config")
                                            .on_click(|_, _, _| println!("Config")),
                                    ),
                            ),
                    ),
            )
    }
}

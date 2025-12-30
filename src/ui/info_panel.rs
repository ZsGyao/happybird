use std::ops::Range;

use chrono::format::Item;
use gpui::*;
use gpui_component::{h_flex, v_flex};
use smallvec::SmallVec;

use crate::ui::indent_guides::{IndentGuideColors, RenderedIndentGuide, indent_guides};

#[derive(Clone, Debug)]
pub struct FileEntry {
    pub id: usize,
    pub name: String,
    pub depth: usize,
    pub is_dir: bool,
    pub is_selected: bool,
}

impl Render for FileEntry {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .text_center()
            .child(format!("{} -- {}", self.id, self.name))
    }
}

pub struct InfoPanel {
    entries: Vec<FileEntry>,
    scroll_handle: UniformListScrollHandle,
    focus_handle: FocusHandle,
    selected_idx: Option<usize>,
}

impl Focusable for InfoPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl InfoPanel {
    pub fn new(cx: &mut App) -> Entity<Self> {
        // 生成测试数据
        let entries = Self::generate_dummy_data();

        cx.new(|cx| InfoPanel {
            entries,
            scroll_handle: UniformListScrollHandle::new(),
            focus_handle: cx.focus_handle(),
            selected_idx: None,
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

impl Render for InfoPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entries = self.entries.clone();
        let item_count = entries.len();

        h_flex()
            .id("info-panel")
            .size_full()
            .relative()
            .track_focus(&self.focus_handle(cx))
            .child(
                v_flex().size_full().child(
                    uniform_list("entries", item_count, move |range, _, cx| {
                        // range 是系统计算出的当前可见行 (例如 0..15)
                        range
                            .map(|ix| {
                                let item = &entries[ix];

                                // 直接构建 UI 元素
                                div()
                                    .h(px(28.0)) // 必须给固定高度，因为是 uniform_list
                                    .flex()
                                    .items_center()
                                    .pl(px(20.0) * item.depth as f32) // 缩进
                                    .bg(if item.is_selected {
                                        hsla(0.0, 0.0, 0.2, 1.0)
                                    } else {
                                        hsla(0.0, 0.0, 0.0, 0.0)
                                    })
                                    .child(format!(
                                        "{} {}",
                                        if item.is_dir { "📁" } else { "📄" },
                                        item.name
                                    ))
                            })
                            .collect::<Vec<_>>() // 返回 Vec<Div>
                    })
                    .size_full()
                    .with_decoration(
                        indent_guides(px(14.0), IndentGuideColors::panel(cx))
                            .with_compute_indents_fn(cx.entity(), |this, range, window, cx| {
                                let mut depths = SmallVec::with_capacity(range.end - range.start);

                                // 直接遍历当前可见范围内的 entries
                                // range 是系统传进来的，比如 0..20
                                for i in range {
                                    if let Some(entry) = this.entries.get(i) {
                                        // 直接获取深度，不需要 calculate_depth_and_difference
                                        depths.push(entry.depth);
                                    }
                                }
                                depths
                            })
                            .with_render_fn(cx.entity(), move |this, params, _, cx| {
                                const LEFT_OFFSET: Pixels = px(14.); // 文字左侧的 Padding
                                const PADDING_Y: Pixels = px(4.); // 垂直方向微调
                                const HITBOX_OVERDRAW: Pixels = px(3.); // 增加点击热区

                                let indent_size = params.indent_size;
                                let item_height = params.item_height;

                                // 暂时移除复杂的 find_active_indent_guide 逻辑
                                // 如果你想实现高亮，需要监听 MouseMove 并计算坐标，这里先设为 None
                                let active_indent_guide_index = None;

                                params
                                    .indent_guides
                                    .into_iter()
                                    .enumerate()
                                    .map(|(idx, layout)| {
                                        // 处理线条延伸到屏幕外的情况
                                        let offset = if layout.continues_offscreen {
                                            px(0.)
                                        } else {
                                            PADDING_Y
                                        };

                                        // 计算线条的几何位置
                                        let bounds = Bounds::new(
                                            point(
                                                layout.offset.x * indent_size + LEFT_OFFSET,
                                                layout.offset.y * item_height + offset,
                                            ),
                                            size(
                                                px(1.), // 线宽 1px
                                                layout.length * item_height - offset * 2.,
                                            ),
                                        );

                                        // 返回渲染对象
                                        RenderedIndentGuide {
                                            bounds,
                                            layout,
                                            // 判断是否激活（高亮）
                                            is_active: active_indent_guide_index == Some(idx),
                                            // 设置交互热区（Hitbox）
                                            hitbox: Some(Bounds::new(
                                                point(
                                                    bounds.origin.x - HITBOX_OVERDRAW,
                                                    bounds.origin.y,
                                                ),
                                                size(
                                                    bounds.size.width + HITBOX_OVERDRAW * 2.,
                                                    bounds.size.height,
                                                ),
                                            )),
                                        }
                                    })
                                    .collect()
                            }),
                    ),
                ),
            )
    }
}

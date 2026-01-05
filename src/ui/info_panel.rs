use std::ops::Range;

use chrono::{DateTime, Local, format::Item};
use gpui::{prelude::FluentBuilder, *};
use gpui_component::{
    ActiveTheme, ColorName, Sizable, StyledExt, h_flex,
    resizable::{h_resizable, resizable_panel, v_resizable},
    scroll::ScrollableElement,
    tag::Tag,
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
        let mut rng = rand::thread_rng();
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
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .h_flex()
            .w_full()
            .justify_between()
            .items_center()
            .p_1()
            .child(
                v_flex()
                    .gap(px(2.0))
                    .child(div().font_bold().child(self.import_name.clone()).truncate())
                    .child(div().font_thin().child(self.import_desc.clone()).truncate()),
            )
            .child(
                v_flex()
                    .gap(px(2.0))
                    .items_end()
                    .child(div().child(format!("{}", self.import_time.format("%Y-%m-%d %H:%M"))))
                    .child(Tag::color(ColorName::Green).small().when_else(
                        self.import_status,
                        |this| this.child("success"),
                        |this| this.child("failure"),
                    )),
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .text_center()
            .child(format!("{} -- {}", self.id, self.name))
    }
}

impl Render for InfoPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                                            |this, range, window, cx| {
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
                                        .with_render_fn(cx.entity(), move |this, params, _, cx| {
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
                                                            layout.offset.y * item_height + offset,
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
                                                                bounds.origin.x - HITBOX_OVERDRAW,
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
                                        }),
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
                                        .overflow_y_scrollbar()
                                        .children(self.import_history.clone()),
                                ),
                        ),
                ),
            )
            .child(
                // Bottom panel
                div().h(px(120.0)).w_full().child("Import New Data").child(
                    div()
                        .h_flex()
                        .justify_around()
                        .child("Export")
                        .child("Config"),
                ),
            )
    }
}

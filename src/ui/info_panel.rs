// =============================================================================
//  1. ViewModel (视图模型)
// =============================================================================

use std::{collections::HashSet, ops::Range};

use gpui::{
    App, AppContext, Bounds, ClickEvent, Context, Div, Entity, FocusHandle, InteractiveElement,
    IntoElement, ParentElement, Pixels, Render, SharedString, Stateful, StatefulInteractiveElement,
    Styled, UniformListScrollHandle, Window, div, point, px, size, uniform_list,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, StyledExt, button::Button, h_flex, label::Label, list::ListItem,
};
use smallvec::SmallVec;

use crate::ui::{
    indent_guides::{IndentGuideColors, RenderedIndentGuide, indent_guides},
    models::{GlobalAppState, Models},
    search::SearchPanel,
};

/// 用于 UI 渲染的树节点结构。
///
/// 这个结构体是“扁平化”后的树形数据。每一行 UI 对应一个 `TreeItem`。
/// 它解耦了后端数据模型 (`Subject`) 和前端显示逻辑。
#[derive(Clone, Debug)]
struct TreeItem {
    /// 节点的唯一标识符（例如："root", "subject-1"）。
    /// 用于在 `expanded_ids` 中追踪展开状态。
    id: String,
    /// 显示在界面上的文本。
    text: String,
    /// 缩进深度（0 表示根节点，1 表示第一层子节点，以此类推）。
    depth: usize,
    /// 标记该节点是否为容器（文件夹）。
    is_folder: bool,
    /// 如果是文件夹，当前是否处于展开状态。
    is_open: bool,
    /// 关联的后端数据 ID。如果是虚拟节点（如 Root），则为 None。
    subject_id: Option<i32>,
}

// =============================================================================
//  2. Component (主组件)
// =============================================================================

/// 侧边信息面板组件。
///
/// 负责展示资源树（Subjects）和历史记录。
/// 实现了 `Render` trait 以绘制 UI，并处理所有相关的鼠标键盘交互。
pub struct InfoPanel {
    // --- 状态数据 ---
    /// 用于渲染的扁平化树节点列表。
    tree_items: Vec<TreeItem>,
    /// 记录当前已展开的文件夹 ID 集合。
    expanded_ids: HashSet<String>,
    /// 当前选中项在 `tree_items` 中的索引。
    selected_idx: Option<usize>,

    // --- UI 句柄 ---
    /// 列表的滚动状态句柄。
    scroll_handle: UniformListScrollHandle,
    /// 焦点控制句柄，用于处理键盘导航。
    focus_handle: FocusHandle,

    /// 顶部的搜索框组件。
    search: Entity<SearchPanel>,
}

impl InfoPanel {
    /// 创建一个新的 `InfoPanel` 实例。
    ///
    /// # Arguments
    /// * `window` - 当前窗口句柄。
    /// * `cx` - 应用上下文。
    pub fn new(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let focus_handle = cx.focus_handle();
            let search = SearchPanel::new(window, cx);

            // 初始化时默认展开根节点
            let mut expanded_ids = HashSet::new();
            expanded_ids.insert("root".to_string());

            let mut panel = InfoPanel {
                tree_items: vec![],
                expanded_ids,
                selected_idx: None,
                scroll_handle: UniformListScrollHandle::new(),
                focus_handle,
                search,
            };

            // 1. 获取全局状态
            let global_model = cx.global::<GlobalAppState>().0.clone();

            // 2. 初始构建树
            panel.rebuild_tree(global_model.read(cx));

            // 3. 订阅数据变化：当后端数据更新时，自动重绘树
            cx.observe(&global_model, |this: &mut Self, model, cx| {
                this.rebuild_tree(model.read(cx));
                cx.notify();
            })
            .detach();

            panel
        })
    }

    /// 根据后端模型 (`Models`) 和当前展开状态 (`expanded_ids`) 重建 `tree_items`。
    ///
    /// 这是一个核心逻辑，它将层级数据“拍平”为列表，供 `uniform_list` 渲染。
    fn rebuild_tree(&mut self, model: &Models) {
        self.tree_items.clear();

        // --- 1. 添加虚拟根节点 ---
        let root_id = "root".to_string();
        let is_root_open = self.expanded_ids.contains(&root_id);

        self.tree_items.push(TreeItem {
            id: root_id.clone(),
            text: format!("All Subjects ({})", model.total_count), // 显示总数,
            depth: 0,
            is_folder: true,
            is_open: is_root_open,
            subject_id: None,
        });

        // --- 2. 添加子节点 (仅当根节点展开时) ---
        // 目前 Subject 结构是扁平的，这里将其作为根节点的直接子级。
        // 如果未来 Subject 有层级关系，这里可以使用递归函数来生成。
        if is_root_open {
            for sub in &model.subjects {
                self.tree_items.push(TreeItem {
                    id: sub.id.to_string(),
                    text: sub.name.clone(),
                    depth: 1,         // 根节点是 0，子节点是 1
                    is_folder: false, // 暂时假设 Subject 是叶子节点（文件）
                    is_open: false,
                    subject_id: Some(sub.id),
                });
            }
        }
    }

    // --- 交互逻辑 (Actions) ---

    /// 切换指定索引项的展开/折叠状态。
    fn toggle_expanded(&mut self, ix: usize, cx: &mut Context<Self>) {
        if let Some(item) = self.tree_items.get(ix) {
            if item.is_folder {
                if self.expanded_ids.contains(&item.id) {
                    self.expanded_ids.remove(&item.id);
                } else {
                    self.expanded_ids.insert(item.id.clone());
                }

                // 状态变更后，必须重新计算树结构
                let store = cx.global::<GlobalAppState>().0.read(cx);
                self.rebuild_tree(store);
                cx.notify();
            }
        }
    }

    /// 选中指定索引的项。
    fn select_item(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.selected_idx = Some(ix);
        cx.notify();

        // 可选：触发其他全局事件，例如通知右侧面板显示详情
        if let Some(item) = self.tree_items.get(ix) {
            if let Some(subject_id) = item.subject_id {
                println!("Selected Subject ID: {}", subject_id);
                // cx.emit(SelectionChangedEvent(subject_id));
            }
        }
    }

    /// 渲染单个列表项 (Atom Renderer)。
    ///
    /// 仿照 Zed 的模式，将每一行的渲染逻辑独立出来，保持 `render` 函数整洁。
    ///
    /// # Arguments
    /// * `ix` - 当前项在列表中的索引。
    /// * `view` - 组件自身的 View 句柄，用于在闭包中回调组件方法。
    /// * `cx` - 窗口上下文。
    fn render_tree_item(
        &self,
        ix: usize,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        // 定义布局常量，确保对齐
        const INDENT_SIZE: Pixels = px(16.0);

        // 安全检查：防止渲染越界
        if ix >= self.tree_items.len() {
            return div().id("empty");
        }

        let item = &self.tree_items[ix];

        // --- 逻辑：检测是否需要加载更多 (Infinite Scroll) ---
        // 如果渲染到了倒数第 10 个元素，触发加载下一页
        if ix + 10 >= self.tree_items.len() {
            // 使用 window.defer 避免在 render 循环中直接 update
            cx.defer(move |cx| {
                // 访问全局状态
                let global_model = cx.global::<GlobalAppState>().0.clone();
                global_model.update(cx, |model, cx| {
                    if model.has_more && !model.is_loading_more && !model.is_loading {
                        println!(">>> Triggering fetch_page (next page)");
                        model.fetch_page(cx, false); // false = 加载下一页
                    }
                });
            });
        }

        let is_selected = self.selected_idx == Some(ix);
        let is_folder = item.is_folder;
        let is_open = item.is_open;
        let depth = item.depth;

        // 计算背景色
        let bg_color = if is_selected {
            cx.theme().colors.selection
        } else {
            gpui::transparent_black()
        };

        div()
            .id(SharedString::from(format!(
                "tree-item-{}",
                self.tree_items[ix].id
            )))
            .h(px(24.0))
            .relative()
            .items_center()
            .rounded_none()
            .pl(INDENT_SIZE * depth as f32)
            .pr(px(8.0))
            .cursor_pointer()
            .bg(bg_color)
            .on_click(cx.listener(move |this, event: &ClickEvent, _window, cx| {
                if event.is_right_click() || event.first_focus() {
                    return;
                }
                cx.stop_propagation();

                // this.select_item(ix, cx);
                if is_folder {
                    this.toggle_expanded(ix, cx);
                } else {
                    println!("Open tree item");
                }
            }))
            .child(
                ListItem::new(SharedString::from(format!("tree-list-item-{}", item.id))).child(
                    h_flex()
                        .items_center()
                        .gap_2()
                        .text_sm()
                        .child(if is_folder {
                            Icon::new(IconName::Folder)
                        } else {
                            Icon::new(IconName::File)
                        })
                        .child(Label::new(item.text.clone())),
                ),
            )
    }
}

impl Render for InfoPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let item_count = self.tree_items.len();

        // 样式常量
        const INDENT_SIZE: Pixels = px(16.0);
        const GUIDE_OFFSET: Pixels = px(16.0); // 缩进线应该在图标容器(16px)的中间(8px)

        div()
            .v_flex()
            .id("info-panel")
            .size_full()
            .p(px(13.0))
            .bg(cx.theme().colors.background)
            .relative()
            .track_focus(&self.focus_handle)
            .gap(px(8.0))
            // ------ search -------
            .child(div().w_full().child(self.search.clone()))
            // ------ sider center source tree
            .child(
                div().flex_1().size_full().relative().child(
                    uniform_list("entries", item_count, {
                        cx.processor(|this, range: Range<usize>, window, cx| {
                            range
                                .map(|ix| this.render_tree_item(ix, window, cx).into_any_element())
                                .collect()
                        })
                    })
                    .size_full()
                    // --------- Visual Guides ----------
                    .with_decoration(
                        indent_guides(INDENT_SIZE, IndentGuideColors::panel(cx))
                            .with_compute_indents_fn(cx.entity(), |this, range, _, _| {
                                let mut depths = SmallVec::with_capacity(range.len());
                                for i in range {
                                    if let Some(entry) = this.tree_items.get(i) {
                                        depths.push(entry.depth);
                                    }
                                }
                                depths
                            })
                            .with_render_fn(cx.entity(), |_, params, _, _| {
                                const PADDING_Y: Pixels = px(4.);
                                let indent_size = params.indent_size;
                                let item_height = params.item_height;

                                params
                                    .indent_guides
                                    .into_iter()
                                    .map(|layout| {
                                        let offset = if layout.continues_offscreen {
                                            px(0.)
                                        } else {
                                            PADDING_Y
                                        };
                                        let x_pos = layout.offset.x * indent_size + GUIDE_OFFSET;

                                        RenderedIndentGuide {
                                            bounds: Bounds::new(
                                                point(
                                                    x_pos,
                                                    layout.offset.y * item_height + offset,
                                                ),
                                                size(
                                                    px(1.),
                                                    layout.length * item_height - offset * 2.,
                                                ),
                                            ),
                                            layout,
                                            is_active: false,
                                            hitbox: None,
                                        }
                                    })
                                    .collect()
                            }),
                    )
                    .track_scroll(self.scroll_handle.clone()),
                ),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .border_t_1()
                    .border_color(cx.theme().colors.border)
                    .pt(px(8.0))
                    .child(
                        Button::new("Import-button")
                            .w_full()
                            .label("Import New Data")
                            .on_click(|_, _, _| println!("Import clicked")),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap_2()
                            .h(px(32.0))
                            .child(
                                div()
                                    .flex_1()
                                    .child(Button::new("Export").w_full().label("Export")),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .child(Button::new("Config").w_full().label("Config")),
                            ),
                    ),
            )
    }
}

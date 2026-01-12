// =============================================================================
//  1. ViewModel (视图模型)
// =============================================================================

use std::{
    collections::{HashMap, HashSet},
    ops::Range,
};

use gpui::{
    App, AppContext, AsyncApp, Bounds, ClickEvent, Context, Div, Entity, FocusHandle,
    InteractiveElement, IntoElement, ParentElement, PathPromptOptions, Pixels, Render,
    SharedString, Stateful, StatefulInteractiveElement, Styled, UniformListScrollHandle, Window,
    div, point, prelude::FluentBuilder, px, size, uniform_list,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable, StyledExt,
    button::{Button, ButtonVariants},
    h_flex,
    label::Label,
    list::ListItem,
    menu::{DropdownMenu, PopupMenuItem},
};
use smallvec::SmallVec;

use crate::{
    backend::db::models::Subject,
    debug, error,
    ui::{
        indent_guides::{IndentGuideColors, RenderedIndentGuide, indent_guides},
        models::{GlobalAppState, Models},
        search::SearchPanel,
    },
    warn,
};

/// 用于 UI 渲染的树节点结构。
#[derive(Clone, Debug)]
struct TreeItem {
    /// 节点的唯一标识符（例如："root", "group:department:RD", "subject:1"）。
    /// 这里的 ID 生成策略对于正确折叠/展开至关重要。
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
    /// 如果是分组节点，记录它是按哪个字段分的（用于图标或样式区分）。
    group_key: Option<String>,
}

// =============================================================================
//  2. Component (主组件)
// =============================================================================

/// 侧边信息面板组件。
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
            group_key: None,
        });

        if !is_root_open {
            return;
        }

        // 2. 开始递归分组处理
        // 获取当前激活的分组 keys (例如 ["department", "role"])
        let group_keys = &model.grouping_state.active_grouping_keys;
        // 准备所有 subjects 的引用
        let subjects: Vec<&Subject> = model.subjects.iter().collect();
        // 调用递归函数
        self.build_recursive_groups(subjects, group_keys, 0, &root_id, 1);
    }

    /// 递归构建分组树
    ///
    /// # Arguments
    /// * `subjects` - 当前层级待处理的用户列表
    /// * `group_keys` - 剩余的分组依据字段列表
    /// * `key_idx` - 当前正在处理 group_keys 中的第几个字段
    /// * `parent_id` - 父节点的 ID（用于生成唯一 ID）
    /// * `depth` - 当前深度
    fn build_recursive_groups(
        &mut self,
        subjects: Vec<&Subject>,
        group_keys: &Vec<String>,
        key_idx: usize,
        parent_id: &str,
        depth: usize,
    ) {
        // 如果没有更多的分组字段了，直接渲染剩余的用户为叶子节点
        if key_idx >= group_keys.len() {
            for sub in subjects {
                self.tree_items.push(TreeItem {
                    id: format!("{}:sub:{}", parent_id, sub.id),
                    text: sub.name.clone(),
                    depth,
                    is_folder: false,
                    is_open: false,
                    subject_id: Some(sub.id),
                    group_key: None,
                });
            }
            return;
        }

        // 获取当前用于分组的字段名 (例如 "department")
        let current_key = &group_keys[key_idx];

        // 1. 分组逻辑：将 subjects 按照 current_key 的值归类
        let mut groups: HashMap<String, Vec<&Subject>> = HashMap::new();
        let mut uncategorized: Vec<&Subject> = Vec::new();

        for sub in subjects {
            // 尝试从 JSON attributes 中获取值
            if let Some(val) = sub.attributes.get(current_key) {
                // 将 Value 转换为可显示的字符串 key
                let group_name = match val {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    _ => "Other".to_string(), // 数组或对象暂归为 Other
                };
                groups.entry(group_name).or_default().push(sub);
            } else {
                uncategorized.push(sub);
            }
        }

        // 2. 对分组名进行排序，保证 UI 稳定
        let mut sorted_group_names: Vec<String> = groups.keys().cloned().collect();
        sorted_group_names.sort();

        // 3. 渲染每一个分组文件夹
        for group_name in sorted_group_names {
            let group_subjects = groups.remove(&group_name).unwrap();
            let count = group_subjects.len();

            // 生成该分组的唯一 ID
            // 格式建议: "parent_id:key:value" -> "root:department:RD"
            let node_id = format!("{}:{}:{}", parent_id, current_key, group_name);
            let is_open = self.expanded_ids.contains(&node_id);

            // 添加分组节点
            self.tree_items.push(TreeItem {
                id: node_id.clone(),
                text: format!("{} ({})", group_name, count),
                depth,
                is_folder: true,
                is_open,
                subject_id: None,
                group_key: Some(current_key.clone()),
            });

            // 如果展开，递归处理下一级
            if is_open {
                self.build_recursive_groups(
                    group_subjects,
                    group_keys,
                    key_idx + 1, // 移动到下一个分组字段
                    &node_id,
                    depth + 1,
                );
            }
        }

        // 4. 处理未分类的项 (Uncategorized)
        // 这些项通常放在最后，或者如果不分组就不显示文件夹直接显示
        if !uncategorized.is_empty() {
            let uncat_node_id = format!("{}:uncategorized", parent_id);
            // 策略：如果还有下一级分组，或者这是第一级，我们把未分类的单独放一个文件夹
            // 如果这是最后一级，直接展示用户

            let is_open = self.expanded_ids.contains(&uncat_node_id);
            self.tree_items.push(TreeItem {
                id: uncat_node_id.clone(),
                text: format!("Uncategorized ({})", uncategorized.len()),
                depth,
                is_folder: true,
                is_open,
                subject_id: None,
                group_key: None,
            });

            if is_open {
                // 对于未分类的，我们仍然尝试对其进行下一级分组 (key_idx + 1)
                // 这样即使用户没有 "department"，但可能有 "interest"
                self.build_recursive_groups(
                    uncategorized,
                    group_keys,
                    key_idx + 1,
                    &uncat_node_id,
                    depth + 1,
                );
            }
        }
    }

    // ------------------------------------ Actions ---------------------------------

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

    // [新增] 添加分组条件
    fn action_add_grouping(&mut self, key: String, cx: &mut Context<Self>) {
        let global_handle = cx.global::<GlobalAppState>().0.clone();
        global_handle.update(cx, |model, cx| {
            model.grouping_state.add_grouping(key);
            cx.notify(); // 通知 Model 更新
        });
        // rebuild_tree 会通过 observe 自动触发
    }

    // [新增] 移除分组条件
    fn action_remove_grouping(&mut self, key: String, cx: &mut Context<Self>) {
        let global_handle = cx.global::<GlobalAppState>().0.clone();
        global_handle.update(cx, |model, cx| {
            model.grouping_state.remove_grouping(&key);
            cx.notify();
        });
    }

    // --------------------------------- Renderers ----------------------------

    /// 渲染顶部的分组配置条
    fn render_grouping_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let global = cx.global::<GlobalAppState>().0.read(cx);
        let active_keys = &global.grouping_state.active_grouping_keys;
        let available_headers = &global.dynamic_headers;

        let global_model = cx.global::<GlobalAppState>().0.clone();

        h_flex()
            .w_full()
            .gap(px(4.0))
            .flex_wrap()
            .items_center()
            .child(
                Label::new("Group by:")
                    .text_xs()
                    .text_color(cx.theme().colors.muted_foreground),
            )
            // 1. 渲染已激活的分组 Tag (这部分代码很好，保持不变)
            .children(active_keys.iter().map(|key| {
                let key_clone = key.clone();
                div()
                    .flex()
                    .items_center()
                    .gap(px(2.0))
                    .bg(cx.theme().colors.secondary)
                    .rounded_md()
                    .px(px(4.0))
                    .py(px(2.0))
                    .border_1()
                    .border_color(cx.theme().colors.border)
                    .child(Label::new(key.clone()).text_xs())
                    .child(
                        div()
                            .id(SharedString::from(format!("del-{}", key)))
                            .cursor_pointer()
                            .child(Icon::new(IconName::Close).size(px(10.0)))
                            .hover(|s| s.text_color(cx.theme().colors.link_hover))
                            .on_click(cx.listener(move |this, _, _window, cx| {
                                this.action_remove_grouping(key_clone.clone(), cx);
                            })),
                    )
            }))
            // 2. 添加按钮 (+) - 使用官方推荐的 dropdown_menu
            .child(
                Button::new("add-group")
                    .icon(IconName::Plus)
                    .small()
                    .ghost()
                    // [关键修改] 使用 dropdown_menu 构建原生风格菜单
                    .dropdown_menu({
                        let headers = available_headers.clone();
                        let active_set: HashSet<_> = active_keys.iter().cloned().collect();

                        let global_model = global_model.clone();

                        move |mut menu, _window, cx| {
                            // 场景 A: 没有可选属性
                            if headers.is_empty() {
                                return menu.item(
                                    PopupMenuItem::new("No attributes found").disabled(true),
                                );
                            }

                            // 场景 B: 循环添加可选属性
                            // 使用 fold (或循环赋值) 来动态链式添加 item
                            for h in &headers {
                                if !active_set.contains(h) {
                                    let h_clone = h.clone();
                                    let model = global_model.clone();

                                    menu = menu.item(PopupMenuItem::new(h.clone()).on_click(
                                        move |_, window, cx| {
                                            // 直接更新全局状态
                                            model.update(cx, |m, cx| {
                                                m.grouping_state.add_grouping(h_clone.clone());
                                                cx.notify();
                                            });
                                        },
                                    ));
                                }
                            }

                            // 场景 C: 添加分隔线和清除按钮
                            if !active_set.is_empty() {
                                let model = global_model.clone();
                                menu = menu.separator().item(
                                    PopupMenuItem::new("Clear Grouping")
                                        // 注意：PopupMenuItem 可能暂时不支持直接 set color，
                                        // 如果需要红色警告色，可能需要查看文档是否支持 style 或 icon，
                                        // 或者暂时用普通文本，这里用 Trash 图标增强语义。
                                        .icon(IconName::Close)
                                        .on_click(move |_, window, cx| {
                                            model.update(cx, |m, cx| {
                                                m.grouping_state.clear();
                                                cx.notify();
                                            });
                                        }),
                                );
                            }

                            menu
                        }
                    }),
            )
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
        let group_key = item.group_key.clone();

        let bg_color = if is_selected {
            cx.theme().colors.selection
        } else {
            gpui::transparent_black()
        };

        div()
            .id(SharedString::from(format!("tree-item-{}", item.id)))
            .h(px(24.0))
            .relative()
            .items_center()
            .pl(INDENT_SIZE * depth as f32)
            .pr(px(8.0))
            .cursor_pointer()
            .bg(bg_color)
            .hover(|s| s.bg(cx.theme().colors.info_hover))
            .on_click(cx.listener(move |this, event: &ClickEvent, _window, cx| {
                if event.is_right_click() || event.first_focus() {
                    return;
                }
                cx.stop_propagation();

                // 如果是文件夹，点击即切换
                if is_folder {
                    this.toggle_expanded(ix, cx);
                } else {
                    this.select_item(ix, cx);
                }
            }))
            .child(
                ListItem::new(SharedString::from(format!("li-{}", item.id))).child(
                    h_flex()
                        .items_center()
                        .gap_2()
                        .text_sm()
                        .child(
                            // 图标逻辑：
                            // Root -> Globe/Database
                            // Group -> Folder (如果是部门可以用 Building, 兴趣用 Heart 等，这里暂统用 Folder)
                            // User -> User/File
                            if item.id == "root" {
                                Icon::new(IconName::Dash).text_color(cx.theme().colors.primary)
                            } else if is_folder {
                                let icon = if is_open {
                                    IconName::FolderOpen
                                } else {
                                    IconName::Folder
                                };
                                // 不同的分组层级可以用不同颜色
                                let color = match group_key.as_deref() {
                                    Some("department") | Some("部门") => cx.theme().colors.info,
                                    Some("role") | Some("职位") => cx.theme().colors.warning,
                                    _ => cx.theme().colors.blue,
                                };
                                Icon::new(icon).text_color(color)
                            } else {
                                Icon::new(IconName::User)
                                    .text_color(cx.theme().colors.muted_foreground)
                            },
                        )
                        .child(
                            Label::new(item.text.clone())
                                // 如果是分组节点，加粗显示
                                .when(is_folder && item.id != "root", |s| {
                                    s.font_weight(gpui::FontWeight::SEMIBOLD)
                                }),
                        ),
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
            // ------ search
            .child(div().w_full().child(self.search.clone()))
            // ------ group control
            .child(self.render_grouping_bar(cx))
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
                            .on_click(|_, _, cx| {
                                // let directories = cx.can_select_mixed_files_and_dirs();
                                let task = cx.prompt_for_paths(PathPromptOptions {
                                    files: true,
                                    directories: false,
                                    multiple: false,
                                    prompt: None,
                                });

                                cx.spawn(|cx: &mut AsyncApp| {
                                    // 【关键修复步骤】
                                    // 在进入 async 块之前，我们把 cx 克隆一份。
                                    // AsyncWindowContext 是一个轻量级句柄，克隆它是廉价且必须的。
                                    // 这样 async 块捕获的就是一个“拥有所有权”的 cx，而不是临时的引用。
                                    let cx = cx.clone();

                                    async move {
                                        match task.await {
                                            Ok(Ok(Some(paths))) => {
                                                if let Some(path) = paths.first() {
                                                    let p = path.clone();
                                                    debug!("path -> {:?}", p);

                                                    // 3. 使用克隆后的 cx 更新全局状态
                                                    // 这里的 cx 是 AsyncWindowContext，它可以在后台存活
                                                    cx.update(|cx| {
                                                        if cx.has_global::<GlobalAppState>() {
                                                            let global = cx
                                                                .global::<GlobalAppState>()
                                                                .0
                                                                .clone();
                                                            global.update(cx, |model, cx| {
                                                                model.preview_file(cx, p);
                                                            });
                                                        }
                                                    })
                                                    .ok();
                                                }
                                            }
                                            Ok(Ok(None)) => {
                                                warn!("Cancel file select");
                                            }
                                            Err(e) => {
                                                error!("System dialog error: {}", e);
                                            }
                                            Ok(Err(e)) => {
                                                error!("Task error: {}", e);
                                            }
                                        }
                                    }
                                })
                                .detach(); // detach 表示让这个任务独立运行
                            }),
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
                            )
                            .child(
                                div().flex_1().child(
                                    Button::new("Test Button")
                                        .w_full()
                                        .label("Test Button")
                                        .on_click(|e, window: &mut Window, cx| {
                                            println!("Test Button click");
                                            let model = cx.global::<GlobalAppState>().0.clone();
                                            model.update(cx, |val, _| {
                                                val.show_test = !val.show_test;
                                            })
                                        }),
                                ),
                            ),
                    ),
            )
    }
}

// =============================================================================
//  1. ViewModel (视图模型)
// =============================================================================

use std::{
    collections::{HashMap, HashSet},
    ops::Range,
};

use gpui::{
    App, AppContext, AsyncApp, Bounds, ClickEvent, Context, Div, Entity, FocusHandle, FontWeight,
    InteractiveElement, IntoElement, ParentElement, PathPromptOptions, Pixels, Render,
    ScrollStrategy, SharedString, Stateful, StatefulInteractiveElement, Styled,
    UniformListScrollHandle, Window, div, point, prelude::FluentBuilder, px, size, uniform_list,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable,
    button::{Button, ButtonCustomVariant, ButtonVariants},
    checkbox::Checkbox,
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
        export_modal::ExportModal,
        hb_icons::HappyBirdIcons,
        indent_guides::{IndentGuideColors, RenderedIndentGuide, indent_guides},
        models::{GlobalAppState, Models},
        search::SearchPanel,
    },
    warn,
};

gpui::actions!(
    info_panel,
    [
        SelectPrev,
        SelectNext,
        PerformPrimaryAction, // Enter/Space
    ]
);

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
    selected_id: Option<String>,

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
                selected_id: None,
                scroll_handle: UniformListScrollHandle::new(),
                focus_handle,
                search,
            };

            // 1. 获取全局状态
            let global_model = cx.global::<GlobalAppState>().0.clone();

            // 2. 初始构建树
            panel.rebuild_tree(global_model.read(cx));

            // =========================================================
            // [新增] 1. 绑定键盘快捷键到 Action
            // =========================================================
            // 当焦点在这个 View 内时，这些快捷键生效
            cx.bind_keys([
                gpui::KeyBinding::new("up", SelectPrev, Some("InfoList")),
                gpui::KeyBinding::new("down", SelectNext, Some("InfoList")),
                gpui::KeyBinding::new("enter", PerformPrimaryAction, Some("InfoList")),
                gpui::KeyBinding::new("space", PerformPrimaryAction, Some("InfoList")),
            ]);

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
        let root_id = "root".to_string();
        let is_root_open = self.expanded_ids.contains(&root_id);

        // 1. 检视模式过滤逻辑 (Review Mode)
        // 如果开启了检视模式，我们只显示选中的人
        let all_subjects: Vec<&Subject> = if model.multi_selection.is_viewing_selected {
            model
                .subjects
                .iter()
                .filter(|s| model.multi_selection.selected_ids.contains(&s.id))
                .collect()
        } else {
            model.subjects.iter().collect()
        };

        // 标题动态变化
        let root_text = if model.multi_selection.is_viewing_selected {
            format!("Selected Items ({})", all_subjects.len())
        } else {
            format!("All Subjects ({})", model.total_count)
        };

        self.tree_items.push(TreeItem {
            id: root_id.clone(),
            text: root_text,
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
        // 调用递归函数
        self.build_recursive_groups(all_subjects, group_keys, 0, &root_id, 1);
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

    // --- Keyboard Action Handlers ---

    /// 处理 "上键"：选中上一个
    fn action_select_prev(&mut self, _: &SelectPrev, _: &mut Window, cx: &mut Context<Self>) {
        if self.tree_items.is_empty() {
            return;
        }

        let current_index = self
            .tree_items
            .iter()
            .position(|item| Some(&item.id) == self.selected_id.as_ref());

        let new_index = match current_index {
            Some(i) => {
                if i > 0 {
                    i - 1
                } else {
                    0
                }
            }
            None => 0, // 如果当前没选中，按下键默认选第一个
        };

        self.select_item_by_index(new_index, cx);
    }

    /// 处理 "下键"：选中下一个
    fn action_select_next(&mut self, _: &SelectNext, _: &mut Window, cx: &mut Context<Self>) {
        if self.tree_items.is_empty() {
            return;
        }

        let current_index = self
            .tree_items
            .iter()
            .position(|item| Some(&item.id) == self.selected_id.as_ref());

        let new_index = match current_index {
            Some(i) => {
                if i < self.tree_items.len() - 1 {
                    i + 1
                } else {
                    self.tree_items.len() - 1
                }
            }
            None => 0,
        };

        self.select_item_by_index(new_index, cx);
    }

    /// 处理 "Enter" 或 "Space"：切换折叠 或 点击 Item
    fn action_perform_primary(
        &mut self,
        _: &PerformPrimaryAction,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // 找到当前选中的项
        if let Some(selected_id) = &self.selected_id {
            // 我们需要找到对应的 tree_item，因为 selected_id 只是个 ID string
            // 这里我们需要 clone 一下 item 以避免借用冲突，或者只取我们需要的数据
            let item_opt = self
                .tree_items
                .iter()
                .find(|i| &i.id == selected_id)
                .cloned();

            if let Some(item) = item_opt {
                if item.is_folder {
                    self.toggle_expanded(&item.id, cx);
                } else {
                    // 如果是文件，执行点击逻辑
                    println!("Keyboard triggered action on: {}", item.text);
                    // 如果点击还有其他副作用（比如打开右侧详情），在这里调用
                    self.select_item(item.id, item.subject_id, cx);
                }
            }
        }
    }

    /// 辅助函数：通过索引选中并滚动可见
    fn select_item_by_index(&mut self, ix: usize, cx: &mut Context<Self>) {
        if let Some(item) = self.tree_items.get(ix) {
            self.selected_id = Some(item.id.clone());
            self.scroll_handle
                .scroll_to_item(ix, ScrollStrategy::Center); // [重要] 自动滚动到该位置
            cx.notify();
        }
    }

    // ------------------------------------ Actions ---------------------------------

    /// 切换指定索引项的展开/折叠状态。
    fn toggle_expanded(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.expanded_ids.contains(id) {
            self.expanded_ids.remove(id);
        } else {
            self.expanded_ids.insert(id.to_string());
        }

        let store = cx.global::<GlobalAppState>().0.read(cx);
        self.rebuild_tree(store);
        cx.notify();
    }

    /// 选中指定索引的项。
    fn select_item(&mut self, id: String, subject_id: Option<i32>, cx: &mut Context<Self>) {
        // 1. 设置当前 UI 的选中状态（高亮显示）
        self.selected_id = Some(id);
        cx.notify();

        // 2. 如果是具体的用户（subject_id 存在），则打开标签页
        if let Some(sid) = subject_id {
            let global_handle = cx.global::<GlobalAppState>().0.clone();

            global_handle.update(cx, |model, cx| {
                // 为了避免 Rust 的借用检查错误（同时对 model 进行不可变借用查找和可变借用修改），
                // 我们先找到并克隆出 Subject 数据
                if let Some(subject) = model.subjects.iter().find(|s| s.id == sid).cloned() {
                    // 调用 models.rs 中定义的 open_tab
                    model.open_tab(&subject, cx);
                }
            });
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
                    .text_sm()
                    .text_center()
                    .text_color(cx.theme().colors.foreground),
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
                    .px(px(6.0))
                    .py(px(2.0))
                    .border_1()
                    .border_color(cx.theme().colors.border)
                    .child(Label::new(key.clone()).text_xs())
                    .child(
                        div()
                            .id(SharedString::from(format!("del-{}", key)))
                            .cursor_pointer()
                            .child(Icon::new(IconName::Close).size(px(12.0)))
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

                        move |mut menu, _window, _cx| {
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
                                        move |_, _window, cx| {
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
                                        .on_click(move |_, _window, cx| {
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

        // Infinite Scroll
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

        let item = &self.tree_items[ix];
        let item_id = item.id.clone();
        let item_subject_id = item.subject_id;

        // 1. 获取全局多选状态
        let global = cx.global::<GlobalAppState>().0.read(cx);
        // 判断当前行是否被选中
        let is_checked = item_subject_id.map_or(false, |sid| {
            global.multi_selection.selected_ids.contains(&sid)
        });
        // 判断是否处于“批量模式”（即列表中只要有一项被选中，就视为批量模式）
        let is_selection_mode = global.multi_selection.is_selection_mode();
        let global_handle = cx.global::<GlobalAppState>().0.clone();

        let is_selected = self.selected_id.as_ref() == Some(&item_id);
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
            .group("row")
            .h(px(28.0))
            .relative()
            .items_center()
            .pl(INDENT_SIZE * depth as f32)
            .pr(px(8.0))
            .cursor_pointer()
            .bg(bg_color)
            .hover(|s| s.bg(cx.theme().colors.info_hover))
            // [交互区域 A] 行点击：只负责打开详情或折叠，绝不处理勾选
            .on_click(cx.listener(move |this, event: &ClickEvent, _window, cx| {
                if event.is_right_click() || event.first_focus() {
                    return;
                }
                cx.stop_propagation();

                // 如果是文件夹 -> 切换折叠
                if is_folder {
                    this.toggle_expanded(&item_id, cx);
                } else {
                    // 如果是文件 -> 选中
                    this.select_item(item_id.clone(), item_subject_id, cx);
                }
            }))
            .child(
                ListItem::new(SharedString::from(format!("li-{}", item.id))).child(
                    h_flex()
                        .items_center()
                        .gap_2()
                        .text_sm()
                        // [交互区域 B] 左侧图标/复选框区域 (固定宽度 20px)
                        .child(
                            div()
                                .w(px(20.0))
                                .flex()
                                .justify_center()
                                .items_center()
                                .child(
                                    // 逻辑分流：是用户节点还是文件夹节点？
                                    if let Some(sid) = item_subject_id {
                                        // === 用户节点逻辑 ===
                                        if is_checked || is_selection_mode {
                                            // 场景 1: 批量模式或已选中 -> 常驻显示 Checkbox
                                            self.render_checkbox(
                                                sid,
                                                is_checked,
                                                global_handle.clone(),
                                                cx,
                                            )
                                            .into_any_element()
                                        } else {
                                            // 场景 2: 普通模式 -> 默认显示 Icon，Hover 时“变身”为 Checkbox
                                            div()
                                                .size_full()
                                                .relative() // 用于绝对定位叠加
                                                // 层级 1: 默认图标 (User Icon) -> Hover 时隐藏
                                                .child(
                                                    div()
                                                        .absolute()
                                                        .inset_0()
                                                        .flex()
                                                        .items_center()
                                                        .justify_center()
                                                        .group_hover("row", |s| s.invisible()) // 关键 CSS
                                                        .child(
                                                            Icon::new(IconName::User).text_color(
                                                                cx.theme().colors.muted_foreground,
                                                            ),
                                                        ),
                                                )
                                                // 层级 2: 悬停复选框 -> Hover 时显示
                                                .child(
                                                    div()
                                                        .absolute()
                                                        .inset_0()
                                                        .flex()
                                                        .items_center()
                                                        .justify_center()
                                                        .invisible() // 默认隐藏
                                                        .group_hover("row", |s| s.visible()) // 关键 CSS
                                                        .child(self.render_checkbox(
                                                            sid,
                                                            false,
                                                            global_handle.clone(),
                                                            cx,
                                                        )),
                                                )
                                                .into_any_element()
                                        }
                                    } else {
                                        // === 文件夹节点逻辑 (保持不变) ===
                                        if item.id == "root" {
                                            Icon::new(IconName::Dash)
                                                .text_color(cx.theme().colors.primary)
                                                .into_any_element()
                                        } else {
                                            let icon = if is_open {
                                                IconName::FolderOpen
                                            } else {
                                                IconName::Folder
                                            };
                                            let color = match group_key.as_deref() {
                                                Some("department") | Some("部门") => {
                                                    cx.theme().colors.info
                                                }
                                                Some("role") | Some("职位") => {
                                                    cx.theme().colors.warning
                                                }
                                                _ => cx.theme().colors.blue,
                                            };
                                            Icon::new(icon).text_color(color).into_any_element()
                                        }
                                    },
                                ),
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

    /// 辅助函数：渲染带有点击拦截的 Checkbox
    ///
    /// 这个函数生成的 Checkbox 会拦截点击事件，防止冒泡到行点击处理器。
    fn render_checkbox(
        &self,
        sid: i32,
        checked: bool,
        global: Entity<Models>,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        Checkbox::new(SharedString::from(format!("chk-{}", sid)))
            .checked(checked) // 使用 bool
            .on_click(move |_, _, cx| {
                cx.stop_propagation();
                global.update(cx, |m, _| m.multi_selection.toggle(sid));
            })
    }

    /// [新增] 渲染底部悬浮操作栏
    fn render_selection_bar(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let global = cx.global::<GlobalAppState>().0.read(cx);
        let selection = &global.multi_selection;
        let count = selection.selected_ids.len();

        if count == 0 {
            return None;
        } // 没选中不显示

        let is_viewing = selection.is_viewing_selected;
        let global_handle = cx.global::<GlobalAppState>().0.clone();

        Some(
            div()
                .absolute()
                .bottom(px(90.0))
                .left(px(0.0))
                .right(px(0.0)) // 底部居中
                .flex()
                .justify_center()
                // 浮在列表之上
                .child(
                    h_flex()
                        .gap(px(12.0))
                        .p(px(8.0))
                        .rounded_xl()
                        .shadow_lg()
                        .border_1()
                        .bg(cx.theme().colors.popover) // 使用 Popover 背景
                        .border_color(cx.theme().colors.border)
                        .items_center()
                        // 1. 计数
                        .child(
                            Label::new(format!("{} Selected", count))
                                .font_weight(FontWeight::BOLD)
                                .text_sm()
                                .pl(px(8.0)),
                        )
                        .child(div().w(px(1.0)).h(px(16.0)).bg(cx.theme().colors.border))
                        // 2. 检视模式按钮
                        .child(
                            Button::new("review-btn")
                                .label(if is_viewing { "Show All" } else { "Review" })
                                .icon(if is_viewing {
                                    HappyBirdIcons::List.load(cx)
                                } else {
                                    HappyBirdIcons::View.load(cx)
                                })
                                .when_else(is_viewing, |this| this.primary(), |this| this.ghost())
                                .on_click({
                                    let g = global_handle.clone();
                                    move |_, _, cx| {
                                        cx.stop_propagation();
                                        g.update(cx, |m, cx| {
                                            m.multi_selection.toggle_view_mode();
                                            cx.notify();
                                        })
                                    }
                                }),
                        )
                        // 3. 导出按钮
                        .child(
                            Button::new("export-sel-btn")
                                .label("Export")
                                .icon(HappyBirdIcons::Download.load(cx))
                                .ghost()
                                .on_click(|_, _, cx| ExportModal::toggle(cx)), // 打开导出框，会自动识别选中项
                        )
                        // 4. 清空按钮
                        .child(
                            Button::new("clear-btn")
                                .icon(IconName::Close)
                                .ghost()
                                .on_click(move |_, _, cx| {
                                    global_handle.update(cx, |m, _| m.multi_selection.clear())
                                }),
                        ),
                ),
        )
    }
}

impl Render for InfoPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let item_count = self.tree_items.len();
        const INDENT_SIZE: Pixels = px(16.0);
        const GUIDE_OFFSET: Pixels = px(16.0);

        let custom_bnt = ButtonCustomVariant::new(cx)
            .color(cx.theme().background)
            .foreground(cx.theme().foreground)
            .border(cx.theme().border)
            .active(cx.theme().secondary_active)
            .hover(cx.theme().background.opacity(0.1));

        div()
            .id("info-panel")
            .size_full()
            .flex()
            .flex_col()
            .p_4()
            .overflow_hidden()
            .bg(cx.theme().colors.background)
            // ------ search
            .child(div().flex_shrink_0().w_full().child(self.search.clone()))
            // ------ group control
            .child(div().flex_shrink_0().child(self.render_grouping_bar(cx)))
            // ------ sider center source tree
            .child(
                div()
                    .flex_1()
                    .w_full()
                    .min_h(px(0.0))
                    .overflow_hidden()
                    .relative()
                    .track_focus(&self.focus_handle)
                    .key_context("InfoList")
                    // 当焦点在这个 div 或其子元素上时，这些 Action 会被捕获并处理
                    .on_action(cx.listener(Self::action_select_prev))
                    .on_action(cx.listener(Self::action_select_next))
                    .on_action(cx.listener(Self::action_perform_primary))
                    .child(
                        uniform_list("entries", item_count, {
                            cx.processor(|this, range: Range<usize>, window, cx| {
                                range
                                    .map(|ix| {
                                        this.render_tree_item(ix, window, cx).into_any_element()
                                    })
                                    .collect()
                            })
                        })
                        .size_full()
                        .track_scroll(self.scroll_handle.clone())
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
                                            let x_pos =
                                                layout.offset.x * indent_size + GUIDE_OFFSET;

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
            // 挂载悬浮栏
            .children(self.render_selection_bar(cx))
            .child(
                div()
                    .flex_shrink_0()
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
                            .rounded_none()
                            .custom(custom_bnt)
                            .label("Import New Data")
                            .text_sm()
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
                                div().flex_1().child(
                                    Button::new("export")
                                        .w_full()
                                        .rounded_none()
                                        .custom(custom_bnt)
                                        .label("Export")
                                        .text_xl(),
                                ),
                            )
                            .child(
                                div().flex_1().child(
                                    Button::new("config")
                                        .w_full()
                                        .rounded_none()
                                        .custom(custom_bnt)
                                        .label("Config")
                                        .text_xl(),
                                ),
                            ), // .child(
                               //     div().flex_1().child(
                               //         Button::new("Test Button")
                               //             .w_full()
                               //             .label("Test Button")
                               //             .on_click(|e, window: &mut Window, cx| {
                               //                 println!("Test Button click");
                               //                 let model = cx.global::<GlobalAppState>().0.clone();
                               //                 model.update(cx, |val, _| {
                               //                     val.show_test = !val.show_test;
                               //                 })
                               //             }),
                               //     ),
                               // ),
                    ),
            )
    }
}

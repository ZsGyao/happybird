use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
};
// 【关键修正1】引入 prelude 和 AppContext trait，确保 cx.new 和 update 可用
use anyhow::Result;
use gpui::{App, Context, Entity, Global, prelude::*};
use gpui_component::input::InputState;
use serde_json::{Map, Value, json};

use crate::backend::db::{config::DbManager, models::Subject, ops::DataService};

/// The global state
pub struct Models {
    // ------------ data ------------------
    /// Database manager
    db_manager: Arc<DbManager>,
    /// The data in the database
    pub subjects: Vec<Subject>,
    /// The data headers
    pub dynamic_headers: Vec<String>,
    /// Total count in the database
    pub total_count: usize,

    // --------- INFO PANEL: page split state ----------
    pub is_loading: bool,
    pub is_loading_more: bool, // 防止滚动到底部时重复出发
    pub has_more: bool,        // 数据库中是否还有更多数据
    pub page: usize,
    pub page_size: usize,
    pub error_msg: Option<String>,
    pub selected_subject_id: Option<i32>,
    pub search_query: String,
    pub grouping_state: GroupingState,

    // ---------- IMPORT PANEL: import preview ----------------
    pub import_preview_state: ImportPreviewState,

    // ---------- DETAIL PANEL: Tab Management ----------
    /// 已打开的标签页列表
    pub tabs: Vec<TabItem>,
    /// 当前激活的标签页 ID (Subject ID)
    pub active_tab_id: Option<i32>,

    // ---------- SHOW ABOUT --------------------------------
    pub show_about: bool,

    // -------- just for test ---------------
    pub show_test: bool,
}

impl Models {
    pub fn new() -> Self {
        let db_manager = Arc::new(DbManager::new().expect("DB Init Failed"));

        Self {
            db_manager,
            subjects: vec![],
            dynamic_headers: vec![],
            is_loading: false,
            is_loading_more: false,
            has_more: true, // 初始假设有数据
            page: 1,
            page_size: 50, // 每页加载 50 条
            error_msg: None,
            selected_subject_id: None,
            search_query: String::new(),
            show_about: false,
            total_count: 0,
            import_preview_state: ImportPreviewState::new(),
            show_test: false,
            grouping_state: GroupingState::default(),
            tabs: vec![],
            active_tab_id: None,
        }
    }

    /// 核心动作：加载数据（支持分页）
    /// is_reload: true = 重新搜索/刷新（清空列表）；false = 滚动加载下一页（追加列表）
    pub fn fetch_page(&mut self, cx: &mut Context<Self>, is_reload: bool) {
        if self.is_loading || self.is_loading_more {
            return;
        }
        if !is_reload && !self.has_more {
            return;
        }

        if is_reload {
            self.is_loading = true;
            self.page = 1;
            self.has_more = true;
            self.subjects.clear();
            // 注意：不要在这里重置 total_count，否则 UI 会闪烁
        } else {
            self.is_loading_more = true;
            self.page += 1;
        }

        cx.notify();

        let db = self.db_manager.clone();
        let query = self.search_query.clone();
        let page = self.page;
        let page_size = self.page_size;

        cx.spawn(async move |this, cx| {
            // --- 后台线程 ---
            // 返回值类型改为 Option<usize>，表示 total_count 是可选更新的
            let result: Result<(Vec<Subject>, Option<usize>)> = cx
                .background_executor()
                .spawn(async move {
                    let conn = db.get_conn()?;

                    // 1. 获取分页数据 (总是执行)
                    let data = DataService::search_subjects(&conn, Some(&query), page, page_size)?;

                    // 2. 优化：仅在 is_reload 为 true 时查询总数
                    let count = if is_reload {
                        Some(DataService::count_subjects(&conn, Some(&query))?)
                    } else {
                        None // 加载下一页时，不更新总数
                    };

                    Ok((data, count))
                })
                .await;

            // --- UI 线程 ---
            let _ = this.update(cx, |store, cx| {
                store.is_loading = false;
                store.is_loading_more = false;

                match result {
                    Ok((new_data, total_opt)) => {
                        // 只有当 total_opt 有值时才更新 store.total_count
                        if let Some(total) = total_opt {
                            store.total_count = total;
                        }

                        // 判断是否还有更多页
                        if new_data.len() < page_size {
                            store.has_more = false;
                        }

                        if is_reload {
                            store.subjects = new_data;
                        } else {
                            store.subjects.extend(new_data);
                            println!("{:?}", store.subjects);
                        }

                        store.recalc_headers();
                    }
                    Err(e) => {
                        store.error_msg = Some(e.to_string());
                        if !is_reload && store.page > 1 {
                            store.page -= 1;
                        }
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn recalc_headers(&mut self) {
        let mut set = BTreeSet::new();
        for sub in &self.subjects {
            if let Some(map) = sub.attributes.as_object() {
                for k in map.keys() {
                    set.insert(k.clone());
                }
            }
        }
        self.dynamic_headers = set.into_iter().collect();
    }

    /// Update specific cell data in the import preview
    ///
    /// * `row_ix` - Row index
    /// * `key` - Column Key (Header name)
    /// * `value` - New value string
    pub fn update_cell_value(&mut self, row_ix: usize, key: &str, value: String) {
        if let Some(data) = &mut self.import_preview_state.import_preview_data {
            if let Some(row) = data.get_mut(row_ix) {
                // 这里默认将所有输入视为 String，实际场景可尝试 parse 为 number/bool
                row.insert(key.to_string(), serde_json::json!(value));
            }
        }
    }

    // --- Action 1: 解析文件 (UI -> Backend) ---
    pub fn preview_file(&mut self, cx: &mut Context<Self>, path: std::path::PathBuf) {
        self.import_preview_state.is_importing = true; // 复用全局 loading 或使用 is_importing
        cx.notify();

        cx.spawn(async move |this, cx| {
            // 在后台线程解析
            let result = cx
                .background_executor()
                .spawn(async move { crate::backend::file::importer::parse_file(&path) })
                .await;

            this.update(cx, |model, cx| {
                model.import_preview_state.is_importing = false;
                match result {
                    Ok(data) => {
                        model.import_preview_state.import_preview_data = Some(data);
                        // debug!("{:?}", model.import_preview_state.import_preview_data.clone().unwrap().first());
                        model.import_preview_state.show_import_modal = true; // 打开预览弹窗
                        model.import_preview_state.import_error = None;
                    }
                    Err(e) => {
                        model.import_preview_state.import_error =
                            Some(format!("Parse failed: {}", e));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    // --- Action 2: 确认导入 (Preview -> DB) ---
    pub fn confirm_import(&mut self, cx: &mut Context<Self>) {
        // 获取数据所有权
        if let Some(mut data) = self.import_preview_state.import_preview_data.take() {
            // 【关键修改】根据选择模式过滤数据
            let final_data = if self.import_preview_state.is_selection_mode_enabled {
                // 如果是选择模式，只保留被选中的行
                // 技巧：使用 filter_map + index
                let selected = &self.import_preview_state.selected_rows;
                if selected.is_empty() {
                    // 如果开启了选择模式但没选数据，这里可以选择报错或者什么都不做
                    // 恢复数据的所有权以便重试
                    self.import_preview_state.import_preview_data = Some(data);
                    self.error_msg = Some("Please select at least one row".to_string());
                    cx.notify();
                    return;
                }

                data.into_iter()
                    .enumerate()
                    .filter(|(i, _)| selected.contains(i))
                    .map(|(_, row)| row)
                    .collect()
            } else {
                // 默认全量导入
                data
            };

            self.import_preview_state.is_importing = true;
            self.import_preview_state.show_import_modal = false;
            // 重置状态
            self.import_preview_state.is_edit_mode_enabled = false;
            self.import_preview_state.is_selection_mode_enabled = false;
            self.import_preview_state.selected_rows.clear();
            cx.notify();

            let db = self.db_manager.clone();

            cx.spawn(async move |this, cx| {
                let result = cx
                    .background_executor()
                    .spawn(async move {
                        let mut conn = db.get_conn()?;
                        crate::backend::db::ops::DataService::batch_import(
                            &mut conn, final_data, "姓名",
                        )
                    })
                    .await;

                // ... (回调处理保持不变)
                this.update(cx, |model, cx| {
                    model.import_preview_state.is_importing = false;
                    match result {
                        Ok(_count) => {
                            // ... 成功逻辑
                            model.fetch_page(cx, true);
                        }
                        Err(e) => {
                            // ... 失败逻辑
                            model.import_preview_state.import_error = Some(e.to_string());
                        }
                    }
                })
                .ok();
            })
            .detach();
        }
    }

    // --- Action 3: 取消导入 ---
    pub fn cancel_import(&mut self, cx: &mut Context<Self>) {
        self.import_preview_state.import_preview_data = None;
        self.import_preview_state.show_import_modal = false;
        self.import_preview_state.import_error = None;
        cx.notify();
    }

    // ----------------------------- Tab Management Logic --------------------------

    /// 打开一个用户标签页。如果已存在则切换过去，否则新建。
    pub fn open_tab(&mut self, subject: &Subject, cx: &mut App) {
        // 注意: 这里通常用 &mut AppContext 来 notify
        if !self.tabs.iter().any(|t| t.subject_id == subject.id) {
            self.tabs.push(TabItem::new(subject));
        }
        self.active_tab_id = Some(subject.id);
        // cx.notify(); // 如果你的架构是在 update 回调外 notify，这里不需要，否则需要
    }

    /// 关闭标签页
    pub fn close_tab(&mut self, subject_id: i32) {
        if let Some(index) = self.tabs.iter().position(|t| t.subject_id == subject_id) {
            self.tabs.remove(index);

            // 如果关闭的是当前激活的，需要切换激活状态
            if self.active_tab_id == Some(subject_id) {
                if let Some(last) = self.tabs.last() {
                    self.active_tab_id = Some(last.subject_id);
                } else {
                    self.active_tab_id = None;
                }
            }
        }
    }

    /// 激活标签页
    pub fn activate_tab(&mut self, subject_id: i32) {
        self.active_tab_id = Some(subject_id);
    }

    /// 获取当前激活的 Tab 数据引用
    pub fn get_active_tab(&self) -> Option<&TabItem> {
        self.active_tab_id
            .and_then(|id| self.tabs.iter().find(|t| t.subject_id == id))
    }

    pub fn get_active_tab_mut(&mut self) -> Option<&mut TabItem> {
        let id = self.active_tab_id?;
        self.tabs.iter_mut().find(|t| t.subject_id == id)
    }

    /// 🧪 测试专用：生成 Dummy 数据注入数据库
    pub fn seed_dummy_data(&self) {
        let mut conn = self.db_manager.get_conn().expect("Failed to connect DB");

        // 1. 检查是否已存在数据
        let count: i32 = conn
            .query_row("SELECT count(*) FROM subjects", [], |r| r.get(0))
            .unwrap_or(0);

        if count > 0 {
            println!(
                ">>> [DB Check] Database already has {} rows. Skipping seed.",
                count
            );
            return;
        }

        println!(">>> [Seeding] Starting to insert dummy data...");

        // 2. 插入 120 条数据
        for i in 1..=120 {
            let name = format!("Test User {:03}", i);
            let mut row = HashMap::new();
            // 模拟多样化数据
            row.insert("age".to_string(), json!(20 + (i % 30)));
            row.insert("email".to_string(), json!(format!("user_{}@hb.com", i)));
            row.insert(
                "role".to_string(),
                json!(if i % 3 == 0 { "Admin" } else { "User" }),
            );
            row.insert(
                "city".to_string(),
                json!(match i % 4 {
                    0 => "Shanghai",
                    1 => "Beijing",
                    2 => "New York",
                    _ => "Tokyo",
                }),
            );

            if let Err(e) = DataService::import_row(&mut conn, &name, row) {
                eprintln!(">>> [Error] Failed to seed row {}: {}", i, e);
            }
        }

        // 3. 【验证点 1】: 打印数据库中的实际数据
        println!(">>> [DB Check] Seeding complete. Verifying DB content...");

        let final_count: i32 = conn
            .query_row("SELECT count(*) FROM subjects", [], |r| r.get(0))
            .unwrap_or(0);
        println!(">>> [DB Check] Total rows in SQLite: {}", final_count);

        // 打印第一条数据来看看长什么样
        let first_user: Option<String> = conn
            .query_row(
                "SELECT name || ' : ' || attributes FROM subjects ORDER BY id ASC LIMIT 1",
                [],
                |r| r.get(0),
            )
            .ok();

        if let Some(info) = first_user {
            println!(">>> [DB Check] First Row Sample: {}", info);
        }
    }
}

pub struct GlobalAppState(pub Entity<Models>);

impl Global for GlobalAppState {}

pub fn build_models(cx: &mut App) {
    let models = cx.new(|cx| {
        let mut models = Models::new();

        // just for test
        // models.seed_dummy_data();

        models.fetch_page(cx, true);
        models
    });

    cx.set_global(GlobalAppState(models));
}

// ------------------------------------------------------ Sub Item ------------------------------------------

/// Import preview global data and state
#[derive(Debug)]
pub struct ImportPreviewState {
    /// Temp store the prase data from import file
    pub import_preview_data: Option<Vec<HashMap<String, Value>>>,
    /// Contorl the import preview ui whether show
    pub show_import_modal: bool,
    /// Check the the file is importing
    pub is_importing: bool,
    /// Store the error msg during import
    pub import_error: Option<String>,
    // ---------------------- Edit ----------------------------
    /// The unit point in table that is editing
    pub editing_cell: Option<(usize, String)>,
    /// Store current active `Input` state entity,
    /// if not store, the new `Input` will create in every render, which cause user cannot input
    pub active_input: Option<Entity<InputState>>,
    /// Whether open the edit mode, true means the table item can modify
    pub is_edit_mode_enabled: bool,
    // ---------------------- Select --------------------------
    /// Whether open the selection mode, true means the table item can select by user, default `false`
    pub is_selection_mode_enabled: bool,
    pub selected_rows: BTreeSet<usize>,
}

impl ImportPreviewState {
    pub fn new() -> Self {
        Self {
            import_preview_data: None,
            show_import_modal: false,
            is_importing: false,
            import_error: None,
            editing_cell: None,
            active_input: None,
            is_edit_mode_enabled: false,
            is_selection_mode_enabled: false,
            selected_rows: BTreeSet::new(),
        }
    }

    /// 切换编辑模式
    pub fn toggle_edit_mode(&mut self) {
        self.is_edit_mode_enabled = !self.is_edit_mode_enabled;
        // 如果关闭编辑模式，强行退出当前的编辑状态
        if !self.is_edit_mode_enabled {
            self.editing_cell = None;
            self.active_input = None;
        }
    }

    /// 切换选择模式
    pub fn toggle_selection_mode(&mut self) {
        self.is_selection_mode_enabled = !self.is_selection_mode_enabled;
        println!("##### toggle_selection_mode click");
        // 如果开启选择模式且当前没选任何行，可以根据需求决定是否全选，或者留空
        // 这里策略：保持 selected_rows 不变，或者清空
    }

    /// 切换某一行的选中状态
    pub fn toggle_row_selection(&mut self, row_ix: usize) {
        if self.selected_rows.contains(&row_ix) {
            self.selected_rows.remove(&row_ix);
        } else {
            self.selected_rows.insert(row_ix);
        }
    }

    /// 全选或取消全选
    pub fn toggle_select_all(&mut self, total_rows: usize) {
        if self.selected_rows.len() == total_rows {
            self.selected_rows.clear();
        } else {
            self.selected_rows = (0..total_rows).collect();
        }
    }
}

/// The info panel group config state
#[derive(Debug, Clone, Default)]
pub struct GroupingState {
    /// 当前启用的分组字段列表，按顺序排列。
    /// 例如：vec!["department", "role"] 表示先按部门分，再按角色分。
    pub active_grouping_keys: Vec<String>,
}

impl GroupingState {
    pub fn add_grouping(&mut self, key: String) {
        if !self.active_grouping_keys.contains(&key) {
            self.active_grouping_keys.push(key);
        }
    }

    pub fn remove_grouping(&mut self, key: &str) {
        self.active_grouping_keys.retain(|k| k != key);
    }

    pub fn clear(&mut self) {
        self.active_grouping_keys.clear();
    }
}

/// 单个标签页的状态
#[derive(Clone, Debug)]
pub struct TabItem {
    pub subject_id: i32,
    pub name: String,
    /// 原始数据（用于对比脏状态和重置）
    pub original_attributes: Map<String, Value>,
    /// 当前正在编辑的数据
    pub working_attributes: Map<String, Value>,
    /// 是否有未保存的更改
    pub is_dirty: bool,
}

impl TabItem {
    pub fn new(subject: &Subject) -> Self {
        // 假设 subject.attributes 是 Value::Object
        let attrs = subject.attributes.as_object().cloned().unwrap_or_default();
        Self {
            subject_id: subject.id,
            name: subject.name.clone(),
            original_attributes: attrs.clone(),
            working_attributes: attrs,
            is_dirty: false,
        }
    }

    pub fn update_field(&mut self, key: &str, value: Value) {
        self.working_attributes.insert(key.to_string(), value);
        self.is_dirty = self.working_attributes != self.original_attributes;
    }

    pub fn mark_saved(&mut self) {
        self.original_attributes = self.working_attributes.clone();
        self.is_dirty = false;
    }
}

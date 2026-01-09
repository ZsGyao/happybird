use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
};
// 【关键修正1】引入 prelude 和 AppContext trait，确保 cx.new 和 update 可用
use anyhow::Result;
use gpui::{App, Context, Entity, Global, prelude::*};
use serde_json::json;

use crate::backend::db::{config::DbManager, models::Subject, ops::DataService};

pub struct Models {
    db_manager: Arc<DbManager>,
    pub subjects: Vec<Subject>,
    pub dynamic_headers: Vec<String>,
    pub total_count: usize, // 数据库中的总记录数

    // ----- 分页状态 -------
    pub is_loading: bool,
    pub is_loading_more: bool, // 防止滚动到底部时重复出发
    pub has_more: bool,        // 数据库中是否还有更多数据
    pub page: usize,
    pub page_size: usize,

    pub error_msg: Option<String>,
    pub selected_subject_id: Option<i32>,
    pub search_query: String,

    pub show_about: bool,
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
        models.seed_dummy_data();

        models.fetch_page(cx, true);
        models
    });

    cx.set_global(GlobalAppState(models));
}

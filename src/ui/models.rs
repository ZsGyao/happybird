use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
};
// 【关键修正1】引入 prelude 和 AppContext trait，确保 cx.new 和 update 可用
use anyhow::Result;
use gpui::{App, Context, Entity, Global, prelude::*};
use serde_json::json;

use crate::backend::{config::DbManager, models::Subject, ops::DataService};

pub struct Models {
    db_manager: Arc<DbManager>,
    pub subjects: Vec<Subject>,
    pub dynamic_headers: Vec<String>,
    pub is_loading: bool,
    pub error_msg: Option<String>,
    pub selected_subject_id: Option<i32>,
    pub search_query: String,
    pub page: usize,
    pub page_size: usize,
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
            error_msg: None,
            selected_subject_id: None,
            search_query: String::new(),
            page: 1,
            page_size: 50,
            show_about: false,
        }
    }

    /// 核心动作：从数据库拉取数据更新到内存
    pub fn fetch_data(&mut self, cx: &mut Context<Self>) {
        // 1. 设置状态
        self.is_loading = true;
        self.error_msg = None;
        cx.notify();

        let db = self.db_manager.clone();
        let query = self.search_query.clone();
        let page = self.page;
        let page_size = self.page_size;

        // 2. 派发异步任务
        // 【关键修正2】：去掉显式的闭包参数类型标注（因为 WeakModel 可能叫 WeakModelHandle），
        // 转而通过标注内部 result 和 store 的类型，让编译器反向推导。
        cx.spawn(async move |this, cx| {
            // --- 后台线程 ---
            let result: Result<Vec<Subject>> = cx
                .background_executor()
                .spawn(async move {
                    let conn = db.get_conn()?;
                    DataService::search_subjects(&conn, Some(&query), page, page_size)
                })
                .await;

            // --- UI 线程 ---
            let _ = this.update(cx, |store, cx| {
                store.is_loading = false;

                match result {
                    Ok(data) => {
                        println!(">>> [UI Check] fetch_data success!");
                        println!(
                            ">>> [UI Check] Loaded {} subjects into Memory (Store).",
                            data.len()
                        );
                        if let Some(first) = data.first() {
                            println!(
                                ">>> [UI Check] First Subject in Memory: ID={} Name={}",
                                first.id, first.name
                            );
                        }

                        store.subjects = data;
                        store.recalc_headers();
                    }
                    Err(e) => {
                        store.error_msg = Some(e.to_string());
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

        // 2. 插入 60 条数据
        for i in 1..=60 {
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

        models.fetch_data(cx);
        models
    });

    cx.set_global(GlobalAppState(models));
}

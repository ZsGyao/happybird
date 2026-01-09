// src/backend/config.rs

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::fs;
use std::path::PathBuf;

/// Database Manager, which manage .db file path, init connection
pub struct DbManager {
    db_path: PathBuf,
}

impl DbManager {
    /// Create a new DbManager instance,
    /// it will detect os system documentpath and create 'HappyBird' dir
    pub fn new() -> Result<Self> {
        let path = Self::resolve_db_path()?;
        println!(">> [Database] Path: {:?}", path);

        let manager = Self { db_path: path };
        manager.init_system()?; // 确保表结构存在
        Ok(manager)
    }

    /// Get a new database conn, Note that Rusqlite conn is not thread safe,
    /// but create single conn in single thread is perfect practice
    pub fn get_conn(&self) -> Result<Connection> {
        let conn = Connection::open(&self.db_path)?;

        // 关键性能优化：开启 WAL (Write-Ahead Logging) 模式
        // 这允许并发读写，防止 GUI 界面因为数据库写入而卡顿。
        conn.pragma_update(None, "journal_mode", "WAL")?;

        // 开启外键约束支持
        conn.pragma_update(None, "foreign_keys", "ON")?;

        Ok(conn)
    }

    /// Prase .db file store path
    fn resolve_db_path() -> Result<PathBuf> {
        // 优先存放在用户的文档目录下，更符合桌面应用规范
        let mut path = dirs::document_dir().context("无法获取系统文档目录")?;

        path.push("HappyBird");
        if !path.exists() {
            fs::create_dir_all(&path)?;
        }
        path.push("happy_bird.db");
        Ok(path)
    }

    /// Init database schema
    fn init_system(&self) -> Result<()> {
        let conn = self.get_conn()?;
        crate::backend::db::schema::create_tables(&conn)?;
        Ok(())
    }
}

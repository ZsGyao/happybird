// src/backend/config.rs

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::fs;
use std::path::PathBuf;

/// 数据库管理器
/// 负责处理文件路径、初始化连接和性能配置。
pub struct DbManager {
    db_path: PathBuf,
}

impl DbManager {
    /// 创建一个新的管理器实例。
    /// 这会自动检测系统文档路径，并创建 'HappyBird' 文件夹。
    pub fn new() -> Result<Self> {
        let path = Self::resolve_db_path()?;
        println!(">> [Database] Path: {:?}", path);

        let manager = Self { db_path: path };
        manager.init_system()?; // 确保表结构存在
        Ok(manager)
    }

    /// 获取一个新的数据库连接。
    /// 注意：Rusqlite 连接不是线程安全的，但在多线程中每个线程创建一个连接是最佳实践。
    pub fn get_conn(&self) -> Result<Connection> {
        let conn = Connection::open(&self.db_path)?;

        // 关键性能优化：开启 WAL (Write-Ahead Logging) 模式
        // 这允许并发读写，防止 GUI 界面因为数据库写入而卡顿。
        conn.pragma_update(None, "journal_mode", "WAL")?;

        // 开启外键约束支持
        conn.pragma_update(None, "foreign_keys", "ON")?;

        Ok(conn)
    }

    /// 内部逻辑：解析存储路径
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

    /// 内部逻辑：初始化表结构
    fn init_system(&self) -> Result<()> {
        let conn = self.get_conn()?;
        crate::backend::schema::create_tables(&conn)?;
        Ok(())
    }
}

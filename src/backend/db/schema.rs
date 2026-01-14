// src/backend/schema.rs

use anyhow::Result;
use rusqlite::Connection;

/// 创建应用所需的所有数据表。
/// 使用 IF NOT EXISTS 确保幂等性。
pub fn create_tables(conn: &Connection) -> Result<()> {
    // 1. 主数据表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS subjects (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT NOT NULL UNIQUE,
            pinyin      TEXT NOT NULL DEFAULT '', -- 存储全拼 (如: zhangsan)
            py_abbr     TEXT NOT NULL DEFAULT '', -- 存储首字母 (如: zs)
            attributes  TEXT NOT NULL DEFAULT '{}',
            created_at  DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at  DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    // 2. 字段元数据表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS field_definitions (
            key         TEXT PRIMARY KEY,
            label       TEXT NOT NULL,
            created_at  DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    // 3. 变更审计日志表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS change_log (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            subject_id  INTEGER, -- 允许为 NULL (全局操作)
            action_type TEXT NOT NULL,
            field_key   TEXT,
            old_value   TEXT,
            new_value   TEXT,
            created_at  DATETIME DEFAULT CURRENT_TIMESTAMP,
            remark      TEXT,  -- 备注字段
            FOREIGN KEY(subject_id) REFERENCES subjects(id) ON DELETE CASCADE
        )",
        [],
    )?;

    Ok(())
}

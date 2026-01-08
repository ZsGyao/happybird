// src/backend/ops.rs

use crate::backend::models::Subject;
use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, ToSql, Transaction, params};
use serde_json::{Map, Value};
use std::collections::HashMap;

/// 数据服务层：封装所有具体的数据库 CRUD 操作。
pub struct DataService;

impl DataService {
    // ========================================================================
    //  Public Interfaces (外部调用的接口)
    // ========================================================================

    /// 导入一行 Excel 数据。
    /// 自动处理：新字段注册、用户创建或更新、审计日志记录。
    pub fn import_row(
        conn: &mut Connection,
        name: &str,
        row_data: HashMap<String, Value>,
    ) -> Result<()> {
        let tx = conn.transaction()?; // 开启事务

        // 1. 同步字段定义
        Self::sync_definitions(&tx, row_data.keys())?;

        // 2. 查找用户
        let existing = Self::find_subject_id_and_attrs(&tx, name)?;

        match existing {
            Some((id, old_json)) => {
                // Update 逻辑
                Self::perform_update(&tx, id, name, old_json, row_data, "IMPORT_UPDATE")?;
            }
            None => {
                // Insert 逻辑
                Self::perform_insert(&tx, name, row_data)?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    /// 更新指定用户的字段 (Patch 操作)。
    pub fn update_fields(
        conn: &mut Connection,
        subject_id: i32,
        updates: HashMap<String, Value>,
    ) -> Result<()> {
        let tx = conn.transaction()?;

        Self::sync_definitions(&tx, updates.keys())?;

        // 获取当前数据
        let current_json: String = tx
            .query_row(
                "SELECT attributes FROM subjects WHERE id = ?",
                params![subject_id],
                |r| r.get(0),
            )
            .context("User not found")?;

        Self::perform_update(
            &tx,
            subject_id,
            "Unknown",
            current_json,
            updates,
            "MANUAL_UPDATE",
        )?;

        tx.commit()?;
        Ok(())
    }

    /// 核心：构建动态 SQL WHERE 子句 (增强版)。
    ///
    /// 该函数负责解析用户输入的自然语言查询字符串，并将其转换为 SQLite 的 `WHERE` 子句及对应的参数列表。
    /// 支持多条件组合（AND 逻辑）、智能类型转换（数字 vs 字符串）以及多种高级操作符。
    ///
    /// # 🔍 支持的搜索语法
    ///
    /// ## 1. 范围查询 (Range)
    /// 用于查找介于两个值之间的数据。
    /// - **语法**: `key:min..max` (**推荐**，通用性强，支持日期和数字)
    /// - **语法**: `key:min-max` (兼容性写法，**仅支持纯数字**，不可用于日期)
    /// - **示例**:
    ///     - `age:18..30` → 查找 `age` 在 18 到 30 岁之间的用户。
    ///     - `created_at:2023-01-01..2023-12-31` → 查找 2023 年创建的所有记录。
    ///
    /// ## 2. 比较查询 (Comparison)
    /// 用于查找大于、小于或等于某值的数据。自动处理数字类型的比较。
    /// - **语法**: `key:>val`, `key:>=val`, `key:<val`, `key:<=val`
    /// - **示例**:
    ///     - `score:>90` → 查找分数大于 90 的记录。
    ///     - `age:>=18` → 查找成年人。
    ///     - `updated_at:>=2024-01-01` → 查找 2024 年及以后更新的记录。
    ///
    /// ## 3. 指定字段匹配 (Key-Value)
    /// 指定特定字段进行查找。对于文本字段默认使用 `LIKE` 模糊匹配。
    /// - **语法**: `key:value`
    /// - **示例**:
    ///     - `name:Alice` → 查找名字中包含 "Alice" 的记录。
    ///     - `city:Shanghai` → 查找 JSON 属性中 `city` 为 "Shanghai" 的记录。
    ///     - `role:Admin` → 查找角色为 "Admin" 的记录。
    ///
    /// ## 4. 全局关键字搜索 (Global Keyword)
    /// 不指定 Key 时，将在 `name` 和所有 `attributes` (JSON) 中进行广撒网式模糊搜索。
    /// - **语法**: `keyword`
    /// - **示例**:
    ///     - `Bob` → 查找名字含 "Bob" 或任意属性（如备注、地址）含 "Bob" 的记录。
    ///
    /// # 💡 组合使用示例
    /// 多个条件可以用空格分隔，它们之间是 **AND (且)** 的关系。
    ///
    /// ```text
    /// // 查找：北京的、20到30岁的、管理员
    /// city:Beijing age:20..30 role:Admin
    /// ```
    ///
    /// # ⚙️ 实现细节
    /// - 对 `name`, `created_at`, `updated_at` 等一级字段直接查询。
    /// - 对其他字段自动使用 `json_extract(attributes, '$.key')` 提取 JSON 属性。
    /// - 针对数字比较，自动添加 `CAST(... AS REAL)` 以修复 SQLite 字符串与数字比较的陷阱。
    fn build_search_query(query: Option<&str>) -> (String, Vec<String>) {
        let raw_query = match query {
            Some(q) if !q.trim().is_empty() => q.trim(),
            _ => return (String::new(), vec![]),
        };

        let mut conditions = Vec::new();
        let mut params = Vec::new();

        for term in raw_query.split_whitespace() {
            if let Some((key, val)) = term.split_once(':') {
                // 1. 确定字段表达式
                // created_at 和 updated_at 是真实列，name 也是
                let column_expr = match key {
                    "name" => "name".to_string(),
                    "created_at" => "created_at".to_string(),
                    "updated_at" => "updated_at".to_string(),
                    _ => format!("json_extract(attributes, '$.{}')", key),
                };

                // 辅助闭包：判断是否需要数字转换
                // 如果输入值能解析为数字，且 key 不是时间字段，则强制转为 REAL 比较
                let wrap_cast = |expr: &str, val: &str| -> String {
                    if val.parse::<f64>().is_ok() && key != "created_at" && key != "updated_at" {
                        format!("CAST({} AS REAL)", expr)
                    } else {
                        expr.to_string()
                    }
                };

                // 2. 解析操作符
                if let Some((min, max)) = val.split_once("..") {
                    // --- Range: ".." (优先支持，兼容日期) ---
                    // date:2023-01-01..2023-12-31
                    let col_left = wrap_cast(&column_expr, min);
                    let col_right = wrap_cast(&column_expr, max);

                    // 注意：这里的 CAST(? AS REAL) 是为了让 SQLite 把传入的字符串参数也当数字处理
                    let val_placeholder_min = if min.parse::<f64>().is_ok() && key != "created_at" {
                        "CAST(? AS REAL)"
                    } else {
                        "?"
                    };
                    let val_placeholder_max = if max.parse::<f64>().is_ok() && key != "created_at" {
                        "CAST(? AS REAL)"
                    } else {
                        "?"
                    };

                    conditions.push(format!(
                        "({} >= {} AND {} <= {})",
                        col_left, val_placeholder_min, col_right, val_placeholder_max
                    ));
                    params.push(min.to_string());
                    params.push(max.to_string());
                } else if let Some(stripped) = val.strip_prefix(">=") {
                    // --- Compare: ">=" ---
                    let col = wrap_cast(&column_expr, stripped);
                    let placeholder = if stripped.parse::<f64>().is_ok() && key != "created_at" {
                        "CAST(? AS REAL)"
                    } else {
                        "?"
                    };
                    conditions.push(format!("{} >= {}", col, placeholder));
                    params.push(stripped.to_string());
                } else if let Some(stripped) = val.strip_prefix("<=") {
                    // --- Compare: "<=" ---
                    let col = wrap_cast(&column_expr, stripped);
                    let placeholder = if stripped.parse::<f64>().is_ok() && key != "created_at" {
                        "CAST(? AS REAL)"
                    } else {
                        "?"
                    };
                    conditions.push(format!("{} <= {}", col, placeholder));
                    params.push(stripped.to_string());
                } else if let Some(stripped) = val.strip_prefix(">") {
                    // --- Compare: ">" ---
                    let col = wrap_cast(&column_expr, stripped);
                    let placeholder = if stripped.parse::<f64>().is_ok() && key != "created_at" {
                        "CAST(? AS REAL)"
                    } else {
                        "?"
                    };
                    conditions.push(format!("{} > {}", col, placeholder));
                    params.push(stripped.to_string());
                } else if let Some(stripped) = val.strip_prefix("<") {
                    // --- Compare: "<" ---
                    let col = wrap_cast(&column_expr, stripped);
                    let placeholder = if stripped.parse::<f64>().is_ok() && key != "created_at" {
                        "CAST(? AS REAL)"
                    } else {
                        "?"
                    };
                    conditions.push(format!("{} < {}", col, placeholder));
                    params.push(stripped.to_string());
                } else if let Some((min, max)) = val.split_once('-') {
                    // --- Legacy Range: "-" (仅当两边都是纯数字时生效，避免误伤日期) ---
                    if min.parse::<f64>().is_ok() && max.parse::<f64>().is_ok() {
                        conditions.push(format!("(CAST({} AS REAL) >= CAST(? AS REAL) AND CAST({} AS REAL) <= CAST(? AS REAL))", column_expr, column_expr));
                        params.push(min.to_string());
                        params.push(max.to_string());
                    } else {
                        // 如果包含 - 但不是纯数字，当作普通字符串 LIKE 查询 (例如 name:Jean-Pierre)
                        conditions.push(format!("{} LIKE ?", column_expr));
                        params.push(format!("%{}%", val));
                    }
                } else {
                    // --- Default: LIKE (Fuzzy Match) ---
                    conditions.push(format!("{} LIKE ?", column_expr));
                    params.push(format!("%{}%", val));
                }
            } else {
                // --- Global Keyword Search ---
                conditions.push("(name LIKE ? OR attributes LIKE ?)".to_string());
                params.push(format!("%{}%", term));
                params.push(format!("%{}%", term));
            }
        }

        if conditions.is_empty() {
            (String::new(), vec![])
        } else {
            (format!("WHERE {}", conditions.join(" AND ")), params)
        }
    }

    /// 统计符合条件的总数
    pub fn count_subjects(conn: &Connection, query: Option<&str>) -> Result<usize> {
        let (where_clause, sql_params) = Self::build_search_query(query);
        let sql = format!("SELECT count(*) FROM subjects {}", where_clause);

        let params_ref: Vec<&dyn ToSql> = sql_params.iter().map(|s| s as &dyn ToSql).collect();

        let count: usize = conn.query_row(&sql, params_ref.as_slice(), |r| r.get(0))?;
        Ok(count)
    }

    /// 搜索与分页查询。
    /// `page`: 从 1 开始。
    pub fn search_subjects(
        conn: &Connection,
        query: Option<&str>,
        page: usize,
        page_size: usize,
    ) -> Result<Vec<Subject>> {
        let offset = (page.max(1) - 1) * page_size;

        // 1. 调用构建器生成 SQL 片段
        let (where_clause, sql_params) = Self::build_search_query(query);

        let sql = format!(
            "SELECT id, name, attributes, created_at, updated_at
                     FROM subjects
                     {}
                     ORDER BY name COLLATE NOCASE ASC
                     LIMIT {} OFFSET {}",
            where_clause, page_size, offset
        );

        let mut stmt = conn.prepare(&sql)?;

        // 2. 动态绑定参数
        // rusqlite 需要 &dyn ToSql 的 slice，我们需要转换一下
        let params_ref: Vec<&dyn ToSql> = sql_params.iter().map(|s| s as &dyn ToSql).collect();

        let mapper = |row: &rusqlite::Row| Self::map_row_to_subject(row);
        let rows = stmt.query_map(params_ref.as_slice(), mapper)?;

        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    /// 撤销指定用户的最近一次更改。
    /// 返回值: true 表示成功撤销，false 表示没有可撤销的操作。
    /// 支持撤销“修改”和“新建”
    pub fn undo_last_change(conn: &mut Connection, subject_id: i32) -> Result<bool> {
        let tx = conn.transaction()?;

        // 1. 查找最近一条日志
        let log_entry: Option<(i32, String, Option<String>, Option<String>)> = tx
            .query_row(
                "SELECT id, action_type, field_key, old_value FROM change_log
                 WHERE subject_id = ?
                 ORDER BY id DESC LIMIT 1",
                params![subject_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;

        if let Some((log_id, action_type, key_opt, old_val_opt)) = log_entry {
            match action_type.as_str() {
                // 场景 A: 撤销“新建” -> 删除该用户
                "CREATE" | "IMPORT_CREATE" => {
                    tx.execute("DELETE FROM subjects WHERE id = ?", params![subject_id])?;
                    // 级联删除日志
                    tx.execute(
                        "DELETE FROM change_log WHERE subject_id = ?",
                        params![subject_id],
                    )?;
                }

                // 场景 B: 撤销“修改” -> 恢复旧值
                "UPDATE" | "MANUAL_UPDATE" | "IMPORT_UPDATE" => {
                    if let Some(key) = key_opt {
                        // 读取当前 JSON
                        let curr_attr_str: String = tx.query_row(
                            "SELECT attributes FROM subjects WHERE id = ?",
                            params![subject_id],
                            |r| r.get(0),
                        )?;
                        let mut attrs: Map<String, Value> = serde_json::from_str(&curr_attr_str)?;

                        // 还原值
                        let old_val: Value = old_val_opt
                            .and_then(|s| serde_json::from_str(&s).ok())
                            .unwrap_or(Value::Null);

                        if old_val.is_null() {
                            attrs.remove(&key);
                        } else {
                            attrs.insert(key, old_val);
                        }

                        // 写回
                        tx.execute(
                                "UPDATE subjects SET attributes = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
                                params![serde_json::to_string(&attrs)?, subject_id]
                            )?;

                        // 删除该条日志 (视为回退)
                        tx.execute("DELETE FROM change_log WHERE id = ?", params![log_id])?;
                    }
                }

                _ => return Ok(false), // 其他类型暂不支持撤销 (如 GLOBAL_DELETE)
            }

            tx.commit()?;
            return Ok(true);
        }

        Ok(false)
    }

    /// 获取所有注册过的动态字段头 (用于前端渲染表格列)
    pub fn get_all_headers(conn: &Connection) -> Result<Vec<String>> {
        let mut stmt = conn.prepare("SELECT key FROM field_definitions ORDER BY key ASC")?;
        let rows = stmt.query_map([], |r| r.get(0))?;

        let mut headers = Vec::new();
        for r in rows {
            headers.push(r?);
        }
        Ok(headers)
    }

    /// 删除指定用户 (硬删除)
    /// 注意：由于我们在 schema.rs 里开启了 Foreign Key，
    /// 如果配置了 ON DELETE CASCADE，日志会自动删；否则需要手动删。
    /// 这里建议手动处理，或者保留日志作为“尸体”记录。
    pub fn delete_subject(conn: &mut Connection, subject_id: i32) -> Result<()> {
        let tx = conn.transaction()?;

        // 1. 先记录一条“删除”日志 (为了支持未来的撤销删除功能，需要把当前数据记下来)
        // 获取当前数据快照
        let current_json: String = tx
            .query_row(
                "SELECT attributes FROM subjects WHERE id = ?",
                params![subject_id],
                |r| r.get(0),
            )
            .optional()?
            .unwrap_or("{}".to_string());

        tx.execute(
            "INSERT INTO change_log (subject_id, action_type, old_value) VALUES (?, 'DELETE', ?)",
            params![subject_id, current_json],
        )?;

        // 2. 执行删除
        // 如果你的数据库 schema 外键没加 ON DELETE CASCADE，这里需要先删 log
        // tx.execute("DELETE FROM change_log WHERE subject_id = ?", params![subject_id])?;

        let affected = tx.execute("DELETE FROM subjects WHERE id = ?", params![subject_id])?;

        if affected == 0 {
            return Err(anyhow::anyhow!("Subject ID {} not found", subject_id));
        }

        tx.commit()?;
        Ok(())
    }

    /// 全局删除某个字段 (Schema 变更)
    /// 1. 从 field_definitions 移除
    /// 2. 从所有 subjects 的 JSON 中移除该 Key
    pub fn delete_field_globally(conn: &mut Connection, field_key: &str) -> Result<()> {
        let tx = conn.transaction()?;

        // 1. 删定义
        tx.execute(
            "DELETE FROM field_definitions WHERE key = ?",
            params![field_key],
        )?;

        // 2. 删数据 (使用 SQLite JSON 函数)
        // 语法: json_remove(attributes, '$.field_key')
        let json_path = format!("$.{}", field_key);
        tx.execute(
            "UPDATE subjects SET attributes = json_remove(attributes, ?)",
            params![json_path],
        )?;

        // 3. 记日志 (全局操作 subject_id = 0 或 NULL)
        tx.execute(
            "INSERT INTO change_log (subject_id, action_type, field_key, old_value)
                 VALUES (NULL, 'GLOBAL_DELETE', ?, 'COLUMN_REMOVED')",
            params![field_key],
        )?;

        tx.commit()?;
        Ok(())
    }

    // ========================================================================
    //  Private Helpers (内部辅助函数)
    // ========================================================================

    fn sync_definitions<'a, I>(tx: &Transaction, keys: I) -> Result<()>
    where
        I: Iterator<Item = &'a String>,
    {
        let mut stmt =
            tx.prepare("INSERT OR IGNORE INTO field_definitions (key, label) VALUES (?, ?)")?;
        for key in keys {
            stmt.execute(params![key, key])?;
        }
        Ok(())
    }

    fn find_subject_id_and_attrs(tx: &Transaction, name: &str) -> Result<Option<(i32, String)>> {
        tx.query_row(
            "SELECT id, attributes FROM subjects WHERE name = ?",
            params![name],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(Into::into)
    }

    fn map_row_to_subject(row: &rusqlite::Row) -> rusqlite::Result<Subject> {
        let json_str: String = row.get(2)?;
        Ok(Subject {
            id: row.get(0)?,
            name: row.get(1)?,
            attributes: serde_json::from_str(&json_str).unwrap_or(Value::Null),
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
        })
    }

    fn perform_insert(tx: &Transaction, name: &str, data: HashMap<String, Value>) -> Result<()> {
        let json_str = serde_json::to_string(&data)?;
        tx.execute(
            "INSERT INTO subjects (name, attributes) VALUES (?, ?)",
            params![name, json_str],
        )?;
        let id = tx.last_insert_rowid();

        tx.execute(
            "INSERT INTO change_log (subject_id, action_type, new_value) VALUES (?, 'CREATE', ?)",
            params![id, "User Created"],
        )?;
        Ok(())
    }

    fn perform_update(
        tx: &Transaction,
        id: i32,
        _name_debug: &str, // 仅用于日志打印，不参与逻辑
        old_json_str: String,
        updates: HashMap<String, Value>,
        action_type: &str,
    ) -> Result<()> {
        let mut attrs: Map<String, Value> = serde_json::from_str(&old_json_str)?;
        let mut has_changes = false;

        for (key, new_val) in updates {
            // 使用 cloned() 避免借用冲突
            let old_val = attrs.get(&key).cloned().unwrap_or(Value::Null);

            if old_val != new_val {
                has_changes = true;
                tx.execute(
                    "INSERT INTO change_log (subject_id, action_type, field_key, old_value, new_value)
                     VALUES (?, ?, ?, ?, ?)",
                    params![id, action_type, key, old_val.to_string(), new_val.to_string()]
                )?;
                attrs.insert(key, new_val);
            }
        }

        if has_changes {
            let new_json = serde_json::to_string(&attrs)?;
            tx.execute(
                "UPDATE subjects SET attributes = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
                params![new_json, id],
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::schema;
    use rusqlite::Connection;
    use serde_json::json;
    use std::collections::HashMap; // 假设 schema 模块可见

    /// 辅助函数：初始化内存数据库并创建表结构
    fn setup_db() -> Result<Connection> {
        let conn = Connection::open_in_memory()?;
        schema::create_tables(&conn)?;
        Ok(conn)
    }

    #[test]
    fn test_import_new_subject() -> Result<()> {
        let mut conn = setup_db()?;

        // 准备数据: Alice, Age=25, Role=Admin
        let mut row = HashMap::new();
        row.insert("age".to_string(), json!(25));
        row.insert("role".to_string(), json!("Admin"));

        // 执行导入
        DataService::import_row(&mut conn, "Alice", row)?;

        // 验证 1: 主表数据
        let subjects = DataService::search_subjects(&conn, Some("Alice"), 1, 10)?;
        assert_eq!(subjects.len(), 1);
        assert_eq!(subjects[0].name, "Alice");
        assert_eq!(subjects[0].attributes.get("age"), Some(&json!(25)));

        // 验证 2: 审计日志 (应该有一条 CREATE 记录)
        let count: i32 = conn.query_row(
            "SELECT count(*) FROM change_log WHERE action_type = 'CREATE'",
            [],
            |r| r.get(0),
        )?;
        assert_eq!(count, 1);

        Ok(())
    }

    #[test]
    fn test_import_update_existing_subject() -> Result<()> {
        let mut conn = setup_db()?;

        // 1. 第一次导入: Bob, Age=30
        let mut row1 = HashMap::new();
        row1.insert("age".to_string(), json!(30));
        DataService::import_row(&mut conn, "Bob", row1)?;

        // 2. 第二次导入: Bob, Age=31 (变了), City=NY (新增)
        let mut row2 = HashMap::new();
        row2.insert("age".to_string(), json!(31));
        row2.insert("city".to_string(), json!("NY"));
        DataService::import_row(&mut conn, "Bob", row2)?;

        let subjects = DataService::search_subjects(&conn, Some("Bob"), 1, 1)?;
        let bob = &subjects[0]; // 先获取 Vec，再取第 0 个元素的引用

        // Age 应该更新为 31
        assert_eq!(bob.attributes.get("age"), Some(&json!(31)));
        // City 应该存在
        assert_eq!(bob.attributes.get("city"), Some(&json!("NY")));

        // 验证日志: 应该有 UPDATE 记录记录了 Age 的变化
        let logs: Vec<String> = conn
            .prepare(
                "SELECT field_key FROM change_log WHERE action_type = 'IMPORT_UPDATE' ORDER BY id",
            )?
            .query_map([], |r| r.get(0))?
            .collect::<Result<Vec<_>, _>>()?; // rusqlite Result转换

        assert!(logs.contains(&"age".to_string()));
        // 注意: city 是新字段插入，视具体逻辑可能记为 UPDATE 或仅在 attributes 增加，这里 ops 实现是 diff 逻辑，所以 nil -> "NY" 也会被记录
        assert!(logs.contains(&"city".to_string()));

        Ok(())
    }

    #[test]
    fn test_manual_update_fields() -> Result<()> {
        let mut conn = setup_db()?;

        // 初始化数据
        DataService::import_row(
            &mut conn,
            "Charlie",
            HashMap::from([("score".to_string(), json!(100))]),
        )?;

        let subjects = DataService::search_subjects(&conn, Some("Charlie"), 1, 1)?;
        let charlie_id = subjects[0].id;

        // 执行手动修改 (PATCH)
        let mut updates = HashMap::new();
        updates.insert("score".to_string(), json!(95)); // 修改
        updates.insert("status".to_string(), json!("Active")); // 新增

        DataService::update_fields(&mut conn, charlie_id, updates)?;

        // 验证
        let subjects_updated = DataService::search_subjects(&conn, None, 1, 10)?;
        let charlie = &subjects_updated[0];
        assert_eq!(charlie.attributes.get("score"), Some(&json!(95)));
        assert_eq!(charlie.attributes.get("status"), Some(&json!("Active")));

        Ok(())
    }

    #[test]
    fn test_search_and_pagination() -> Result<()> {
        let mut conn = setup_db()?;

        // 插入 3 条数据
        DataService::import_row(&mut conn, "User_A", HashMap::new())?; // ID 1
        DataService::import_row(&mut conn, "User_B", HashMap::new())?; // ID 2
        DataService::import_row(&mut conn, "User_C", HashMap::new())?; // ID 3

        // 稍微 Sleep 保证 updated_at 不同 (SQLite 时间精度可能不够，如果不Sleep排序可能不稳定)
        // 但这里我们主要测试 limit/offset 逻辑

        // 测试 1: 搜索全部，分页 Page 1, Size 2
        let page1 = DataService::search_subjects(&conn, None, 1, 2)?;
        assert_eq!(page1.len(), 2);
        // 默认按 updated_at DESC，所以应该是 C 和 B
        assert_eq!(page1[0].name, "User_C");
        assert_eq!(page1[1].name, "User_B");

        // 测试 2: 分页 Page 2, Size 2
        let page2 = DataService::search_subjects(&conn, None, 2, 2)?;
        assert_eq!(page2.len(), 1);
        assert_eq!(page2[0].name, "User_A");

        // 测试 3: 关键词搜索
        let search_res = DataService::search_subjects(&conn, Some("User_B"), 1, 10)?;
        assert_eq!(search_res.len(), 1);
        assert_eq!(search_res[0].name, "User_B");

        Ok(())
    }

    #[test]
    fn test_undo_functionality() -> Result<()> {
        let mut conn = setup_db()?;

        // 1. 创建 Dave, Level=1
        DataService::import_row(
            &mut conn,
            "Dave",
            HashMap::from([("level".to_string(), json!(1))]),
        )?;
        let dave_id = DataService::search_subjects(&conn, Some("Dave"), 1, 1)?[0].id;

        // 2. 修改 Level 1 -> 2
        DataService::update_fields(
            &mut conn,
            dave_id,
            HashMap::from([("level".to_string(), json!(2))]),
        )?;

        // 确认修改成功
        let dave_v2 = &DataService::search_subjects(&conn, Some("Dave"), 1, 1)?[0];
        assert_eq!(dave_v2.attributes.get("level"), Some(&json!(2)));

        // 3. 第一次撤销：撤销修改 (Level 2 -> 1)
        let success = DataService::undo_last_change(&mut conn, dave_id)?;
        assert!(success, "Undo Update should return true");

        // 确认回退到了 Level 1
        let dave_v1 = &DataService::search_subjects(&conn, Some("Dave"), 1, 1)?[0];
        assert_eq!(dave_v1.attributes.get("level"), Some(&json!(1)));

        // 4. 第二次撤销：撤销创建 (应该成功，并删除用户)
        // 【修改点】：这里逻辑变了，现在支持撤销创建，所以应该返回 true
        let success_create_undo = DataService::undo_last_change(&mut conn, dave_id)?;
        assert!(success_create_undo, "Undo Create should return true");

        // 5. 验证 Dave 已经被删除了
        let res = DataService::search_subjects(&conn, Some("Dave"), 1, 1)?;
        assert_eq!(res.len(), 0, "Dave should be deleted after undoing create");

        Ok(())
    }

    #[test]
    fn test_field_definitions_sync() -> Result<()> {
        let mut conn = setup_db()?;

        // 导入包含不同 Key 的数据
        DataService::import_row(
            &mut conn,
            "User1",
            HashMap::from([
                ("email".to_string(), json!("a@b.com")),
                ("phone".to_string(), json!("123")),
            ]),
        )?;

        DataService::import_row(
            &mut conn,
            "User2",
            HashMap::from([
                ("phone".to_string(), json!("456")),      // 重复 key
                ("address".to_string(), json!("Street")), // 新 key
            ]),
        )?;

        // 获取所有 Header
        let headers = DataService::get_all_headers(&conn)?;

        // 应该是去重并排序后的结果
        assert_eq!(headers, vec!["address", "email", "phone"]);

        Ok(())
    }

    #[test]
    fn test_delete_subject() -> Result<()> {
        let mut conn = setup_db()?;
        DataService::import_row(&mut conn, "Deadpool", HashMap::new())?;

        let id = DataService::search_subjects(&conn, Some("Deadpool"), 1, 1)?[0].id;

        // 删除
        DataService::delete_subject(&mut conn, id)?;

        // 验证查不到了
        let res = DataService::search_subjects(&conn, Some("Deadpool"), 1, 1)?;
        assert_eq!(res.len(), 0);

        Ok(())
    }

    #[test]
    fn test_global_field_delete() -> Result<()> {
        let mut conn = setup_db()?;
        // 插入两人，都有 age
        DataService::import_row(
            &mut conn,
            "A",
            HashMap::from([("age".to_string(), json!(20))]),
        )?;
        DataService::import_row(
            &mut conn,
            "B",
            HashMap::from([("age".to_string(), json!(30))]),
        )?;

        // 全局删除 age
        DataService::delete_field_globally(&mut conn, "age")?;

        // 验证 A 的 age 没了
        let a = &DataService::search_subjects(&conn, Some("A"), 1, 1)?[0];
        assert_eq!(a.attributes.get("age"), None);

        // 验证定义表里也没了
        let headers = DataService::get_all_headers(&conn)?;
        assert!(!headers.contains(&"age".to_string()));

        Ok(())
    }

    #[test]
    fn test_undo_create() -> Result<()> {
        let mut conn = setup_db()?;

        // 1. 创建 user
        DataService::import_row(&mut conn, "MistakeUser", HashMap::new())?;
        let id = DataService::search_subjects(&conn, Some("MistakeUser"), 1, 1)?[0].id;

        // 2. 撤销 (此时最后一条日志是 CREATE)
        let success = DataService::undo_last_change(&mut conn, id)?;
        assert!(success);

        // 3. 验证用户消失了
        let res = DataService::search_subjects(&conn, Some("MistakeUser"), 1, 1)?;
        assert_eq!(res.len(), 0);

        Ok(())
    }
}

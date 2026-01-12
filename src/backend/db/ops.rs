// src/backend/ops.rs

use crate::backend::db::models::Subject;
use anyhow::{Context, Result};
use pinyin::ToPinyin;
use rusqlite::{Connection, OptionalExtension, ToSql, Transaction, params};
use serde_json::{Map, Value};
use std::collections::HashMap;

/// Data service layer, impl all database CRUD operations
pub struct DataService;

impl DataService {
    // ========================================================================
    //  Public Interfaces (External API)
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

    /// 批量导入 (事务处理)
    /// rows: 解析后的通用数据
    /// primary_key_col: 指定 CSV/Excel 中哪一列作为 'name' (主键)
    pub fn batch_import(
        conn: &mut Connection,
        rows: Vec<HashMap<String, Value>>,
        primary_key_col: &str,
    ) -> Result<usize> {
        let tx = conn.transaction()?;
        let mut count = 0;

        for mut row_data in rows {
            // 1. 提取 Name
            // remove 会把 name 从 attributes 中拿出来，剩下的作为 JSON 属性
            // 如果 JSON 中该字段是字符串，直接用；如果是数字，转字符串
            let name_val = row_data.remove(primary_key_col).and_then(|v| match v {
                Value::String(s) => Some(s),
                Value::Number(n) => Some(n.to_string()),
                _ => None,
            });

            if let Some(name) = name_val {
                if !name.trim().is_empty() {
                    // --- 复用单行导入逻辑 (内联版) ---
                    // 注意：因为 tx 是 Transaction，我们不能调用 import_row (它会尝试再开 transaction)
                    // 所以我们需要直接调用 private helpers

                    // 1. 同步字段定义
                    Self::sync_definitions(&tx, row_data.keys())?;

                    // 2. 查找现有用户
                    let existing = Self::find_subject_id_and_attrs(&tx, &name)?;

                    match existing {
                        Some((id, old_json)) => {
                            Self::perform_update(
                                &tx,
                                id,
                                &name,
                                old_json,
                                row_data,
                                "IMPORT_UPDATE",
                            )?;
                        }
                        None => {
                            Self::perform_insert(&tx, &name, row_data)?;
                        }
                    }
                    count += 1;
                }
            }
        }

        tx.commit()?;
        Ok(count)
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

    /// 辅助：生成全拼和首字母
    /// 返回 (全拼字符串, 首字母字符串)
    fn generate_pinyin(input: &str) -> (String, String) {
        let mut full_pinyin = String::new();
        let mut abbr_pinyin = String::new();

        for char in input.chars() {
            if let Some(pinyin) = char.to_pinyin() {
                full_pinyin.push_str(pinyin.plain());
                abbr_pinyin.push(pinyin.plain().chars().next().unwrap_or_default());
            } else {
                // 非汉字字符直接保留
                full_pinyin.push(char);
                abbr_pinyin.push(char);
            }
        }
        (full_pinyin.to_lowercase(), abbr_pinyin.to_lowercase())
    }

    /// 核心：构建动态 SQL WHERE 子句 。
    ///
    /// # 支持的搜索语法
    /// - `key:value` (精确/模糊)
    /// - `key:min..max` (范围)
    /// - `keyword` (全局模糊搜索：**支持中文名、拼音全拼、拼音首字母**)
    fn build_search_query(query: Option<&str>) -> (String, Vec<String>) {
        let raw_query = match query {
            Some(q) if !q.trim().is_empty() => q.trim(),
            _ => return (String::new(), vec![]),
        };

        let mut conditions = Vec::new();
        let mut params = Vec::new();

        for term in raw_query.split_whitespace() {
            if let Some((key, val)) = term.split_once(':') {
                // ... (Key-Value 搜索逻辑保持不变) ...
                // 1. 确定字段表达式
                let column_expr = match key {
                    "name" => "name".to_string(),
                    "created_at" => "created_at".to_string(),
                    "updated_at" => "updated_at".to_string(),
                    _ => format!("json_extract(attributes, '$.{}')", key),
                };

                // 辅助闭包：判断是否需要数字转换
                let wrap_cast = |expr: &str, val: &str| -> String {
                    if val.parse::<f64>().is_ok() && key != "created_at" && key != "updated_at" {
                        format!("CAST({} AS REAL)", expr)
                    } else {
                        expr.to_string()
                    }
                };

                // 2. 解析操作符
                if let Some((min, max)) = val.split_once("..") {
                    // Range
                    let col_left = wrap_cast(&column_expr, min);
                    let col_right = wrap_cast(&column_expr, max);
                    let vp_min = if min.parse::<f64>().is_ok() && key != "created_at" {
                        "CAST(? AS REAL)"
                    } else {
                        "?"
                    };
                    let vp_max = if max.parse::<f64>().is_ok() && key != "created_at" {
                        "CAST(? AS REAL)"
                    } else {
                        "?"
                    };

                    conditions.push(format!(
                        "({} >= {} AND {} <= {})",
                        col_left, vp_min, col_right, vp_max
                    ));
                    params.push(min.to_string());
                    params.push(max.to_string());
                } else if let Some(stripped) = val.strip_prefix(">=") {
                    // >=
                    let col = wrap_cast(&column_expr, stripped);
                    let vp = if stripped.parse::<f64>().is_ok() && key != "created_at" {
                        "CAST(? AS REAL)"
                    } else {
                        "?"
                    };
                    conditions.push(format!("{} >= {}", col, vp));
                    params.push(stripped.to_string());
                } else if let Some(stripped) = val.strip_prefix("<=") {
                    // <=
                    let col = wrap_cast(&column_expr, stripped);
                    let vp = if stripped.parse::<f64>().is_ok() && key != "created_at" {
                        "CAST(? AS REAL)"
                    } else {
                        "?"
                    };
                    conditions.push(format!("{} <= {}", col, vp));
                    params.push(stripped.to_string());
                } else if let Some(stripped) = val.strip_prefix(">") {
                    // >
                    let col = wrap_cast(&column_expr, stripped);
                    let vp = if stripped.parse::<f64>().is_ok() && key != "created_at" {
                        "CAST(? AS REAL)"
                    } else {
                        "?"
                    };
                    conditions.push(format!("{} > {}", col, vp));
                    params.push(stripped.to_string());
                } else if let Some(stripped) = val.strip_prefix("<") {
                    // <
                    let col = wrap_cast(&column_expr, stripped);
                    let vp = if stripped.parse::<f64>().is_ok() && key != "created_at" {
                        "CAST(? AS REAL)"
                    } else {
                        "?"
                    };
                    conditions.push(format!("{} < {}", col, vp));
                    params.push(stripped.to_string());
                } else {
                    // Default LIKE
                    conditions.push(format!("{} LIKE ?", column_expr));
                    params.push(format!("%{}%", val));
                }
            } else {
                // --- Global Keyword Search (修改部分) ---
                // 支持: 名字 OR 拼音 OR 首字母 OR JSON属性
                // 我们将输入的 term 转为小写进行模糊匹配，以配合生成的全小写拼音
                let pattern = format!("%{}%", term.to_lowercase());

                conditions.push(
                    "(name LIKE ? OR pinyin LIKE ? OR py_abbr LIKE ? OR attributes LIKE ?)"
                        .to_string(),
                );

                // 参数分别对应上面 SQL 中的 4 个 ?
                // 1. name (原始输入匹配，这里可以保留原始大小写，或者SQLite NOCASE处理)
                params.push(format!("%{}%", term));
                // 2. pinyin (全拼，全小写匹配)
                params.push(pattern.clone());
                // 3. py_abbr (首字母，全小写匹配)
                params.push(pattern.clone());
                // 4. attributes (JSON 属性)
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

        // 1. 生成拼音
        let (pinyin, py_abbr) = Self::generate_pinyin(name);

        // 2. 插入所有字段
        tx.execute(
            "INSERT INTO subjects (name, pinyin, py_abbr, attributes) VALUES (?, ?, ?, ?)",
            params![name, pinyin, py_abbr, json_str],
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
    use crate::backend::db::schema;
    use rusqlite::Connection;
    use serde_json::json;
    use std::collections::HashMap;

    /// 辅助函数：初始化内存数据库并创建表结构
    fn setup_db() -> Result<Connection> {
        let conn = Connection::open_in_memory()?;
        schema::create_tables(&conn)?;
        Ok(conn)
    }

    #[test]
    fn test_import_new_subject() -> Result<()> {
        let mut conn = setup_db()?;
        let mut row = HashMap::new();
        row.insert("age".to_string(), json!(25));
        row.insert("role".to_string(), json!("Admin"));

        DataService::import_row(&mut conn, "Alice", row)?;

        let subjects = DataService::search_subjects(&conn, Some("Alice"), 1, 10)?;
        assert_eq!(subjects.len(), 1);
        assert_eq!(subjects[0].name, "Alice");
        assert_eq!(subjects[0].attributes.get("age"), Some(&json!(25)));

        let count: i32 = conn.query_row(
            "SELECT count(*) FROM change_log WHERE action_type = 'CREATE'",
            [],
            |r| r.get(0),
        )?;
        assert_eq!(count, 1);
        Ok(())
    }

    #[test]
    fn test_batch_import_transaction() -> Result<()> {
        let mut conn = setup_db()?;

        // 准备 3 条数据
        // 1. New User
        let mut row1 = HashMap::new();
        row1.insert("name".to_string(), json!("BatchUser1")); // 主键
        row1.insert("score".to_string(), json!(10));

        // 2. New User
        let mut row2 = HashMap::new();
        row2.insert("name".to_string(), json!("BatchUser2"));
        row2.insert("score".to_string(), json!(20));

        // 3. Invalid User (没有 name，应该被跳过)
        let mut row3 = HashMap::new();
        row3.insert("score".to_string(), json!(30));

        let rows = vec![row1, row2, row3];

        // 执行批量导入
        let count = DataService::batch_import(&mut conn, rows, "name")?;

        // 验证：应该只有 2 条成功插入
        assert_eq!(count, 2);

        let subjects = DataService::search_subjects(&conn, Some("BatchUser"), 1, 10)?;
        assert_eq!(subjects.len(), 2);

        // 验证字段定义同步
        let headers = DataService::get_all_headers(&conn)?;
        assert!(headers.contains(&"score".to_string()));

        Ok(())
    }

    #[test]
    fn test_import_update_existing_subject() -> Result<()> {
        let mut conn = setup_db()?;

        let mut row1 = HashMap::new();
        row1.insert("age".to_string(), json!(30));
        DataService::import_row(&mut conn, "Bob", row1)?;

        let mut row2 = HashMap::new();
        row2.insert("age".to_string(), json!(31));
        row2.insert("city".to_string(), json!("NY"));
        DataService::import_row(&mut conn, "Bob", row2)?;

        let subjects = DataService::search_subjects(&conn, Some("Bob"), 1, 1)?;
        let bob = &subjects[0];

        assert_eq!(bob.attributes.get("age"), Some(&json!(31)));
        assert_eq!(bob.attributes.get("city"), Some(&json!("NY")));

        let logs: Vec<String> = conn
            .prepare(
                "SELECT field_key FROM change_log WHERE action_type = 'IMPORT_UPDATE' ORDER BY id",
            )?
            .query_map([], |r| r.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        assert!(logs.contains(&"age".to_string()));
        assert!(logs.contains(&"city".to_string()));

        Ok(())
    }

    #[test]
    fn test_manual_update_fields() -> Result<()> {
        let mut conn = setup_db()?;
        DataService::import_row(
            &mut conn,
            "Charlie",
            HashMap::from([("score".to_string(), json!(100))]),
        )?;

        let subjects = DataService::search_subjects(&conn, Some("Charlie"), 1, 1)?;
        let charlie_id = subjects[0].id;

        let mut updates = HashMap::new();
        updates.insert("score".to_string(), json!(95));
        updates.insert("status".to_string(), json!("Active"));

        DataService::update_fields(&mut conn, charlie_id, updates)?;

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

        // SQL 是 ORDER BY name ASC (A -> B -> C)

        // 测试 1: 搜索全部，分页 Page 1, Size 2 (Offset 0) -> 应该得到 A, B
        let page1 = DataService::search_subjects(&conn, None, 1, 2)?;
        assert_eq!(page1.len(), 2);
        assert_eq!(page1[0].name, "User_A");
        assert_eq!(page1[1].name, "User_B");

        // 测试 2: 分页 Page 2, Size 2 (Offset 2) -> 应该得到 C
        let page2 = DataService::search_subjects(&conn, None, 2, 2)?;
        assert_eq!(page2.len(), 1);
        assert_eq!(page2[0].name, "User_C");

        // 测试 3: 关键词搜索
        let search_res = DataService::search_subjects(&conn, Some("User_B"), 1, 10)?;
        assert_eq!(search_res.len(), 1);
        assert_eq!(search_res[0].name, "User_B");

        Ok(())
    }

    #[test]
    fn test_undo_functionality() -> Result<()> {
        let mut conn = setup_db()?;

        DataService::import_row(
            &mut conn,
            "Dave",
            HashMap::from([("level".to_string(), json!(1))]),
        )?;
        let dave_id = DataService::search_subjects(&conn, Some("Dave"), 1, 1)?[0].id;

        DataService::update_fields(
            &mut conn,
            dave_id,
            HashMap::from([("level".to_string(), json!(2))]),
        )?;

        let dave_v2 = &DataService::search_subjects(&conn, Some("Dave"), 1, 1)?[0];
        assert_eq!(dave_v2.attributes.get("level"), Some(&json!(2)));

        let success = DataService::undo_last_change(&mut conn, dave_id)?;
        assert!(success);

        let dave_v1 = &DataService::search_subjects(&conn, Some("Dave"), 1, 1)?[0];
        assert_eq!(dave_v1.attributes.get("level"), Some(&json!(1)));

        let success_create_undo = DataService::undo_last_change(&mut conn, dave_id)?;
        assert!(success_create_undo);

        let res = DataService::search_subjects(&conn, Some("Dave"), 1, 1)?;
        assert_eq!(res.len(), 0);

        Ok(())
    }

    #[test]
    fn test_field_definitions_sync() -> Result<()> {
        let mut conn = setup_db()?;

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
                ("phone".to_string(), json!("456")),
                ("address".to_string(), json!("Street")),
            ]),
        )?;

        let headers = DataService::get_all_headers(&conn)?;
        assert_eq!(headers, vec!["address", "email", "phone"]);

        Ok(())
    }

    #[test]
    fn test_delete_subject() -> Result<()> {
        let mut conn = setup_db()?;
        DataService::import_row(&mut conn, "Deadpool", HashMap::new())?;

        let id = DataService::search_subjects(&conn, Some("Deadpool"), 1, 1)?[0].id;
        DataService::delete_subject(&mut conn, id)?;

        let res = DataService::search_subjects(&conn, Some("Deadpool"), 1, 1)?;
        assert_eq!(res.len(), 0);

        Ok(())
    }

    #[test]
    fn test_global_field_delete() -> Result<()> {
        let mut conn = setup_db()?;
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

        DataService::delete_field_globally(&mut conn, "age")?;

        let a = &DataService::search_subjects(&conn, Some("A"), 1, 1)?[0];
        assert_eq!(a.attributes.get("age"), None);

        let headers = DataService::get_all_headers(&conn)?;
        assert!(!headers.contains(&"age".to_string()));

        Ok(())
    }

    #[test]
    fn test_undo_create() -> Result<()> {
        let mut conn = setup_db()?;
        DataService::import_row(&mut conn, "MistakeUser", HashMap::new())?;
        let id = DataService::search_subjects(&conn, Some("MistakeUser"), 1, 1)?[0].id;

        let success = DataService::undo_last_change(&mut conn, id)?;
        assert!(success);

        let res = DataService::search_subjects(&conn, Some("MistakeUser"), 1, 1)?;
        assert_eq!(res.len(), 0);

        Ok(())
    }

    #[test]
    fn test_complex_search_combinations() -> Result<()> {
        let mut conn = setup_db()?;

        // 准备数据
        let mut row1 = HashMap::new();
        row1.insert("age".to_string(), json!(25));
        row1.insert("city".to_string(), json!("Beijing"));
        row1.insert("role".to_string(), json!("Dev"));
        DataService::import_row(&mut conn, "Alice", row1)?;

        let mut row2 = HashMap::new();
        row2.insert("age".to_string(), json!(35));
        row2.insert("city".to_string(), json!("Shanghai"));
        row2.insert("role".to_string(), json!("Manager"));
        DataService::import_row(&mut conn, "Bob", row2)?;

        let mut row3 = HashMap::new();
        row3.insert("age".to_string(), json!(28));
        row3.insert("city".to_string(), json!("Beijing"));
        row3.insert("role".to_string(), json!("Manager"));
        DataService::import_row(&mut conn, "Charlie", row3)?;

        // 测试 1: 组合查询 (北京的 Manager)
        // 语法: city:Beijing role:Manager
        let res1 = DataService::search_subjects(&conn, Some("city:Beijing role:Manager"), 1, 10)?;
        assert_eq!(res1.len(), 1);
        assert_eq!(res1[0].name, "Charlie");

        // 测试 2: 范围 + 精确匹配 (年龄大于 30)
        // 语法: age:>30
        let res2 = DataService::search_subjects(&conn, Some("age:>30"), 1, 10)?;
        assert_eq!(res2.len(), 1);
        assert_eq!(res2[0].name, "Bob");

        // 测试 3: 范围区间 (20到30岁之间)
        // 语法: age:20..30
        let res3 = DataService::search_subjects(&conn, Some("age:20..30"), 1, 10)?;
        assert_eq!(res3.len(), 2); // Alice(25), Charlie(28)

        Ok(())
    }

    #[test]
    fn test_special_characters_and_security() -> Result<()> {
        let mut conn = setup_db()?;

        // 1. 测试 SQL 注入风险字符 (单引号)
        // 如果代码没有正确使用 prepared statement，这里会报错或由注入产生异常
        let name_with_quote = "O'Neil";
        DataService::import_row(&mut conn, name_with_quote, HashMap::new())?;

        let res = DataService::search_subjects(&conn, Some("O'Neil"), 1, 10)?;
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].name, "O'Neil");

        // 2. 测试 Emoji 和特殊符号
        let name_emoji = "User🚀";
        DataService::import_row(&mut conn, name_emoji, HashMap::new())?;

        let res_emoji = DataService::search_subjects(&conn, Some("🚀"), 1, 10)?;
        assert_eq!(res_emoji.len(), 1);
        assert_eq!(res_emoji[0].name, "User🚀");

        Ok(())
    }

    #[test]
    fn test_edge_case_inputs() -> Result<()> {
        let mut conn = setup_db()?;

        // 1. 批量导入空数组 (不应报错，应返回 0)
        let count = DataService::batch_import(&mut conn, vec![], "name")?;
        assert_eq!(count, 0);

        // 2. 分页越界 (第 999 页，应返回空 Vec，不报错)
        DataService::import_row(&mut conn, "A", HashMap::new())?;
        let res = DataService::search_subjects(&conn, None, 999, 10)?;
        assert_eq!(res.len(), 0);

        // 3. 搜索非法数字 (age:abc)
        // 系统应足够健壮，将其视为字符串搜索而不是 Panic
        let res_invalid = DataService::search_subjects(&conn, Some("age:abc"), 1, 10)?;
        assert_eq!(res_invalid.len(), 0);

        Ok(())
    }

    #[test]
    fn test_json_types_handling() -> Result<()> {
        let mut conn = setup_db()?;

        let mut row = HashMap::new();
        row.insert("is_active".to_string(), json!(true)); // Boolean
        row.insert("meta".to_string(), Value::Null); // Null
        row.insert("tags".to_string(), json!(["rust", "db"])); // Array

        DataService::import_row(&mut conn, "DataUser", row)?;

        let subjects = DataService::search_subjects(&conn, Some("DataUser"), 1, 1)?;
        let attrs = &subjects[0].attributes;

        assert_eq!(attrs.get("is_active"), Some(&json!(true)));
        assert_eq!(attrs.get("meta"), Some(&Value::Null));

        // 验证数组是否被正确序列化和反序列化
        let tags = attrs.get("tags").unwrap().as_array().unwrap();
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0], "rust");

        Ok(())
    }

    #[test]
    fn test_pinyin_generation_logic() {
        // 测试私有辅助函数 generate_pinyin 是否逻辑正确
        let (full, abbr) = DataService::generate_pinyin("张三");
        assert_eq!(full, "zhangsan", "全拼生成错误");
        assert_eq!(abbr, "zs", "首字母生成错误");

        let (full, abbr) = DataService::generate_pinyin("李四-Wang");
        assert_eq!(full, "lisi-wang", "混合字符全拼错误");

        // [修复]: 当前逻辑是非汉字字符原样保留，所以应该是 "ls-wang"
        assert_eq!(abbr, "ls-wang", "混合字符首字母错误");

        let (full, abbr) = DataService::generate_pinyin("重庆");
        // pinyin 库默认行为：多音字通常取第一个音
        println!("重庆 -> Full: {}, Abbr: {}", full, abbr);
        assert!(!full.is_empty());
    }

    #[test]
    fn test_import_auto_writes_pinyin() -> Result<()> {
        let mut conn = setup_db()?;

        // 导入带中文的数据
        DataService::import_row(&mut conn, "诸葛亮", HashMap::new())?;

        // 验证数据库是否正确存入拼音
        let (pinyin, abbr): (String, String) = conn.query_row(
            "SELECT pinyin, py_abbr FROM subjects WHERE name = '诸葛亮'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;

        assert_eq!(pinyin, "zhugeliang");
        assert_eq!(abbr, "zgl");

        Ok(())
    }

    #[test]
    fn test_search_by_pinyin_full() -> Result<()> {
        let mut conn = setup_db()?;
        DataService::import_row(&mut conn, "张三", HashMap::new())?;
        DataService::import_row(&mut conn, "张飞", HashMap::new())?;
        DataService::import_row(&mut conn, "李四", HashMap::new())?;

        // 1. 搜索 "zhang" (匹配 张三, 张飞)
        let res = DataService::search_subjects(&conn, Some("zhang"), 1, 10)?;
        assert_eq!(res.len(), 2);
        assert!(res.iter().any(|s| s.name == "张三"));
        assert!(res.iter().any(|s| s.name == "张飞"));

        // 2. 搜索 "san" (匹配 张三)
        let res = DataService::search_subjects(&conn, Some("san"), 1, 10)?;
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].name, "张三");

        Ok(())
    }

    #[test]
    fn test_search_by_pinyin_abbr() -> Result<()> {
        let mut conn = setup_db()?;
        DataService::import_row(&mut conn, "欧阳锋", HashMap::new())?;
        DataService::import_row(&mut conn, "郭靖", HashMap::new())?;

        // 1. 搜索 "oyf" (全首字母)
        let res = DataService::search_subjects(&conn, Some("oyf"), 1, 10)?;
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].name, "欧阳锋");

        // 2. 搜索 "yf" (部分首字母)
        let res = DataService::search_subjects(&conn, Some("yf"), 1, 10)?;
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].name, "欧阳锋");

        // 3. 搜索 "gj"
        let res = DataService::search_subjects(&conn, Some("gj"), 1, 10)?;
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].name, "郭靖");

        Ok(())
    }

    #[test]
    fn test_mixed_search_attributes_and_pinyin() -> Result<()> {
        let mut conn = setup_db()?;

        let mut row = HashMap::new();
        row.insert("job".to_string(), json!("Engineer"));
        DataService::import_row(&mut conn, "马云", row)?; // mayun, my

        // 搜索：名字首字母 "my" + 职位 "Engineer"
        // 我们的 build_search_query 空格是 AND 关系
        let res = DataService::search_subjects(&conn, Some("job:Engineer my"), 1, 10)?;

        assert_eq!(res.len(), 1);
        assert_eq!(res[0].name, "马云");

        // 搜索不存在的组合
        let res = DataService::search_subjects(&conn, Some("job:Doctor my"), 1, 10)?;
        assert_eq!(res.len(), 0);

        Ok(())
    }

    #[test]
    fn test_case_insensitive_search() -> Result<()> {
        let mut conn = setup_db()?;
        DataService::import_row(&mut conn, "测试", HashMap::new())?; // ceshi, cs

        // 搜索大写 "CS"
        let res = DataService::search_subjects(&conn, Some("CS"), 1, 10)?;
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].name, "测试");

        // 搜索大写全拼 "CESHI"
        let res = DataService::search_subjects(&conn, Some("CESHI"), 1, 10)?;
        assert_eq!(res.len(), 1);

        Ok(())
    }
}

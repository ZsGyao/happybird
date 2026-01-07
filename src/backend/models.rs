// src/backend/models.rs

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 数据库中 `subjects` 表的映射结构体。
/// 包含了固定的元数据和动态的 JSON 属性。
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Subject {
    /// 唯一标识符
    pub id: i32,
    /// 主字段：姓名
    pub name: String,
    /// 动态属性集合 (存储为 JSONB/Text)
    pub attributes: Value,
    /// 创建时间 (ISO 8601 字符串)
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
}

/// 用于前端列表展示的简化行结构（如果需要分离视图模型）。
#[derive(Debug, Clone)]
pub struct SubjectRow {
    pub id: i32,
    pub name: String,
    pub attributes: Value,
}

/// 审计日志结构体
#[derive(Debug, Serialize, Deserialize)]
pub struct ChangeLog {
    pub id: i32,
    pub subject_id: Option<i32>,
    pub action_type: String, // 'CREATE', 'UPDATE', 'DELETE'
    pub field_key: Option<String>,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub created_at: String,
}

/// 字段定义元数据
#[derive(Debug, Serialize, Deserialize)]
pub struct FieldDefinition {
    pub key: String,
    pub label: String,
}

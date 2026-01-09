use anyhow::{Result, anyhow};
use calamine::{Data, Reader, Xlsx, open_workbook};
use std::{collections::HashMap, fs::File, path::Path, sync::LazyLock};

use serde_json::{Value, json};

/// Generic import data structural： every column is Map (col name -> col value)
pub type ImportData = Vec<HashMap<String, Value>>;

// =============================================================================
//  1. Abstract layer: Import Strategy Trait
// =============================================================================

/// All file praser which want to support import need to impl this trait.
/// This trait constraint `Send + Sync` because that can let global instance
/// safety use in multithread env like "back task".
pub trait ImportStrategy: Send + Sync {
    /// The suffix this strategy support, for example ["csv"] or ["xls", "xlsx"].
    fn supported_extensions(&self) -> &[&str];

    /// Actual prase logical, input path, output generic JSON data.
    fn parse(&self, path: &Path) -> Result<ImportData>;
}

// =============================================================================
//  2. Impl layer: Strategy (CSV & Excel)
// =============================================================================

/// --------- CSV prase strategy ------------
pub struct CsvStrategy;

impl ImportStrategy for CsvStrategy {
    fn supported_extensions(&self) -> &[&str] {
        &["scv"]
    }

    fn parse(&self, path: &Path) -> Result<ImportData> {
        let file = File::open(path)?;
        let mut rdr = csv::Reader::from_reader(file);

        // read header
        let headers = rdr.headers()?.clone();
        let mut result = Vec::new();

        for record in rdr.records() {
            let record = record?;

            let mut row_map = HashMap::new();

            for (i, field) in record.iter().enumerate() {
                if let Some(header) = headers.get(i) {
                    // CSV 只有字符串，尝试智能类型推断：数字 -> 布尔 -> 字符串
                    let val = if let Ok(n) = field.parse::<f64>() {
                        json!(n)
                    } else if let Ok(b) = field.parse::<bool>() {
                        json!(b)
                    } else {
                        json!(field)
                    };
                    row_map.insert(header.to_string(), val);
                }
            }
            result.push(row_map);
        }
        Ok(result)
    }
}

/// --------- Excel prase strategy ------------
pub struct ExcelStrategy;

impl ImportStrategy for ExcelStrategy {
    fn supported_extensions(&self) -> &[&str] {
        &["xlsx", "xls", "xlsm"]
    }

    fn parse(&self, path: &Path) -> Result<ImportData> {
        // 打开 Excel 文件
        // 注意：calamine 的 open_workbook 会根据扩展名自动选择具体的 Reader 实现
        let mut workbook: Xlsx<_> =
            open_workbook(path).map_err(|e| anyhow!("Failed to open excel: {}", e))?;

        // 默认读取第一个 Sheet
        let range = workbook
            .worksheet_range_at(0)
            .ok_or_else(|| anyhow!("Cannot find any worksheet in excel"))?
            .map_err(|e| anyhow!("Failed to read worksheet: {}", e))?;

        let mut rows = range.rows();

        // 读取第一行作为 Header
        let headers: Vec<String> = rows
            .next()
            .ok_or_else(|| anyhow!("Excel file is empty"))?
            .iter()
            .map(|c| c.to_string())
            .collect();

        let mut result = Vec::new();

        for row in rows {
            let mut row_map = HashMap::new();
            for (i, cell) in row.iter().enumerate() {
                if let Some(header) = headers.get(i) {
                    // 将 Excel 的 DataType 转换为 serde_json::Value
                    let val = match cell {
                        Data::Int(v) => json!(*v),
                        Data::Float(v) => json!(*v),
                        Data::String(v) => json!(v),
                        Data::Bool(v) => json!(*v),
                        Data::DateTime(v) => json!(v.to_string()),
                        Data::Error(_) => Value::Null,
                        Data::Empty => Value::Null,
                        Data::DateTimeIso(v) => json!(*v),
                        Data::DurationIso(v) => json!(*v),
                    };
                    row_map.insert(header.to_string(), val);
                }
            }
            result.push(row_map);
        }
        Ok(result)
    }
}

// =============================================================================
//  3. 调度层：全局单例管理器
// =============================================================================

/// 负责注册和分发策略
pub struct FileImporter {
    strategies: Vec<Box<dyn ImportStrategy>>,
}

impl FileImporter {
    /// 创建实例并注册所有已知策略
    fn new() -> Self {
        Self {
            strategies: vec![
                Box::new(CsvStrategy),
                Box::new(ExcelStrategy),
                // 未来扩展：Box::new(JsonStrategy),
            ],
        }
    }

    /// 核心分发方法
    pub fn parse(&self, path: &Path) -> Result<ImportData> {
        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_lowercase())
            .ok_or_else(|| anyhow!("Could not determine file extension"))?;

        // 遍历策略列表，寻找支持该后缀的策略
        for strategy in &self.strategies {
            if strategy
                .supported_extensions()
                .contains(&extension.as_str())
            {
                return strategy.parse(path);
            }
        }

        Err(anyhow!("No importer found for extension: .{}", extension))
    }
}

// =============================================================================
//  4. 对外接口：LazyLock 单例
// =============================================================================

/// 全局唯一的 Importer 实例。
/// 使用 LazyLock 保证首次访问时才初始化，且线程安全。
static GLOBAL_IMPORTER: LazyLock<FileImporter> = LazyLock::new(|| FileImporter::new());

/// 公开的便捷函数，UI 层直接调用这个即可
pub fn parse_file<P: AsRef<Path>>(path: P) -> Result<ImportData> {
    GLOBAL_IMPORTER.parse(path.as_ref())
}

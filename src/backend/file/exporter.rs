// src/backend/file/exporter.rs

use crate::backend::db::models::Subject;
use anyhow::Result;
use rust_xlsxwriter::*;
use serde_json::Value;
use std::path::PathBuf;

/// Excel 导出服务
pub struct ExcelExporter;

impl ExcelExporter {
    /// 将用户数据导出为 Excel 文件 (.xlsx)
    ///
    /// # 参数
    /// - `path`: 目标文件保存路径
    /// - `subjects`: 要导出的用户数据列表
    /// - `fields`: 要导出的字段 Key 列表（即表头），按顺序排列
    ///
    /// # 示例
    /// ```rust
    /// ExcelExporter::export(path, subjects, vec!["name".into(), "email".into()])?;
    /// ```
    pub fn export(path: PathBuf, subjects: Vec<Subject>, fields: Vec<String>) -> Result<()> {
        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();

        println!("{:?}", subjects);

        // 1. 设置表头样式 (加粗)
        let header_format = Format::new().set_bold();

        // 2. 写入表头
        for (col, field) in fields.iter().enumerate() {
            worksheet.write_string_with_format(0, col as u16, field, &header_format)?;
            // 设置一个合理的列宽
            worksheet.set_column_width(col as u16, 20)?;
        }

        // 3. 写入数据行
        for (row_idx, subject) in subjects.iter().enumerate() {
            let row = (row_idx + 1) as u32; // 第0行是表头，数据从第1行开始

            for (col_idx, field) in fields.iter().enumerate() {
                let col = col_idx as u16;

                // 从 attributes JSON 中获取值
                if let Some(value) = subject.attributes.get(field) {
                    match value {
                        Value::String(s) => {
                            println!("String {}", s);
                            worksheet.write_string(row, col, s)?;
                        }
                        Value::Number(n) => {
                            if let Some(f) = n.as_f64() {
                                println!("Number {}", f);
                                worksheet.write_number(row, col, f)?;
                            }
                        }
                        Value::Bool(b) => {
                            println!("Bool {}", b);
                            worksheet.write_boolean(row, col, *b)?;
                        }
                        Value::Null => {
                            println!("Null");
                        } // 空值跳过
                        _ => {
                            worksheet.write_string(row, col, &value.to_string())?;
                        }
                    }
                }
            }
        }

        // 4. 保存文件
        workbook.save(path)?;

        Ok(())
    }
}

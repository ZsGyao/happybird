use std::rc::Rc;

use super::extra::AppThemeExtra;
use gpui::{App, SharedString, Window, px};
use gpui_component::{Theme, ThemeConfig};
use serde::Deserialize;

// --- JSON 映射结构 ---

/// 单个主题的完整配置结构
///
/// 使用 `#[serde(flatten)]` 将标准字段直接映射到 `base`，
/// 将自定义字段映射到 `extra` 和 `overrides`。
#[derive(Deserialize)]
struct ExtendedThemeConfig {
    /// 标准配置 (Colors, Highlight, Mode)
    /// 直接复用 gpui_component 的结构体
    #[serde(flatten)]
    base: ThemeConfig,

    /// 扩展配置 (Icons, Borders)
    #[serde(default)]
    extra: AppThemeExtra,

    /// 覆盖配置 (Radius, Fonts)
    /// 用于强行修改 gpui_component 的基础样式
    #[serde(default)]
    overrides: ThemeOverrides,
}

#[derive(Deserialize, Default)]
struct ThemeOverrides {
    radius: Option<f32>,
    radius_lg: Option<f32>,
    font_size: Option<f32>,
}

/// 对应 JSON 文件的根结构 (包含主题列表)
#[derive(Deserialize)]
struct ThemeFile {
    themes: Vec<ExtendedThemeConfig>,
}

// --- Service Logic ---

/// 加载并应用主题
///
/// # Arguments
/// * `cx`: 应用上下文
/// * `window`: **关键**。传入窗口句柄以触发 `refresh()`，确保边框等装饰层重绘。
/// * `json_content`: JSON 文件内容
/// * `theme_name`: 目标主题名称
pub fn apply_theme(
    cx: &mut App,
    window: Option<&mut Window>,
    json_content: &str,
    theme_name: &str,
) -> anyhow::Result<()> {
    // 1. 解析 JSON
    let theme_file: ThemeFile = serde_json::from_str(json_content)?;

    // 2. 查找目标主题
    let target = theme_file
        .themes
        .into_iter()
        .find(|t| t.base.name == theme_name)
        .ok_or_else(|| anyhow::anyhow!("Theme '{}' not found", theme_name))?;

    let font_family_override = target.extra.font_family.clone();
    // =========================================================
    // 步骤 A: 更新自定义扩展状态 (AppThemeExtra)
    // =========================================================
    // 这会触发所有读取 `AppThemeExtra` 的组件 (如 Icon) 重绘
    cx.set_global(target.extra);

    // =========================================================
    // 步骤 B: 更新标准库状态 (gpui_component::Theme)
    // =========================================================
    let theme = Theme::global_mut(cx);

    // B.1 应用颜色配置
    // apply_config 会自动处理 colors 和 highlight 字段
    let config_rc = Rc::new(target.base);
    theme.apply_config(&config_rc);

    // B.2 同步模式 (Light/Dark)
    // 虽然 apply_config 可能处理了，但显式设置 mode 是最佳实践
    theme.mode = config_rc.mode;

    // B.3 [深度定制] 侵入式修改库的基础属性
    // 这里我们将 JSON 中的 overrides 应用到 Theme 结构体上
    if let Some(r) = target.overrides.radius {
        theme.radius = px(r);
    }
    if let Some(r) = target.overrides.radius_lg {
        theme.radius_lg = px(r);
    }
    if let Some(s) = target.overrides.font_size {
        theme.font_size = px(s);
    }

    // B.4 同步字体覆盖
    // 如果 extra 中定义了字体，我们也更新标准库的 font_family，
    // 这样标准组件（如 Input, List）也会自动变字体
    if let Some(font) = font_family_override {
        theme.font_family = SharedString::from(font.clone());
    }

    // =========================================================
    // 步骤 C: 强制刷新窗口
    // =========================================================
    // 这一步对于 Window Decorations (边框、圆角、阴影) 至关重要。
    if let Some(win) = window {
        win.refresh();
    }

    Ok(())
}

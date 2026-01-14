use gpui::{App, SharedString};
use gpui_component::IconNamed;

use crate::ui::theme::extra::{AppThemeExtra, IconThemeStyle};

/// 图标目录枚举
#[derive(Clone, Copy, Debug)]
pub enum HappyBirdIcons {
    Search,
    File,
    Settings,
    Eidt,
    Message,
}

/// 已解析的图标 (携带了最终路径)
///
/// 这个结构体是为了解决 `IconNamed` trait 无法接收 `cx` 上下文的问题。
/// 我们先通过 `HappyBirdIcons::load(cx)` 生成这个结构体，
/// 然后由这个结构体去满足 `Icon::new()` 的 trait 约束。
#[derive(Clone, Debug)]
pub struct ResolvedIcon(SharedString);

// 1. 为中间结构体实现 IconNamed
impl IconNamed for ResolvedIcon {
    fn path(self) -> SharedString {
        self.0
    }
}

impl HappyBirdIcons {
    /// 注入上下文，根据当前主题配置解析出最终的图标路径。
    ///
    /// # 返回值
    /// 返回一个实现了 `IconNamed` 的结构体，可以直接传给 `Icon::new()`。
    ///
    /// # 示例
    /// ```rust
    /// Icon::new(HappyBirdIcons::Search.load(cx))
    /// ```
    pub fn load(self, cx: &App) -> ResolvedIcon {
        // 1. 获取全局主题配置
        let extra = AppThemeExtra::global(cx);

        // 2. 确定文件名
        let filename = match self {
            HappyBirdIcons::Search => "search",
            HappyBirdIcons::File => "file",
            HappyBirdIcons::Settings => "settings",
            HappyBirdIcons::Eidt => "edit",
            HappyBirdIcons::Message => "message",
        };

        // 3. 确定文件夹 (Outline_Solid / Pixel)
        let folder = match extra.icon_style {
            IconThemeStyle::Outline => "outline",
            IconThemeStyle::Solid => "solid",
            IconThemeStyle::Pixel => "pixel",
        };

        // 4. 构造路径并包装
        let path = format!("icons/{}/{}.svg", folder, filename);
        ResolvedIcon(SharedString::from(path))
    }
}

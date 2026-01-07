use std::path::PathBuf;

use gpui::{App, SharedString};
use gpui_component::{Theme, ThemeRegistry};

use crate::error;

pub fn init(cx: &mut App) {
    let theme_name = SharedString::from("Alduin");
    // Load and watch themes from ./themes directory
    if let Err(err) = ThemeRegistry::watch_dir(PathBuf::from("./themes"), cx, move |cx| {
        if let Some(theme) = ThemeRegistry::global(cx).themes().get(&theme_name).cloned() {
            Theme::global_mut(cx).apply_config(&theme);
        }
    }) {
        error!("Failed to watch themes directory: {}", err);
    }
}

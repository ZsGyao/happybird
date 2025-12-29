use std::path::PathBuf;

use gpui::{App, Global, Rgba, SharedString, rgb, rgba};
use gpui_component::{Theme, ThemeRegistry};
use serde::Deserialize;

use crate::error;

#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct UsrTheme {
    pub background_primary: Rgba,
    pub background_secondary: Rgba,
    pub background_tertiary: Rgba,

    pub border_color: Rgba,

    pub album_art_background: Rgba,

    pub text: Rgba,
    pub text_secondary: Rgba,
    pub text_link: Rgba,
    pub text_about_desc: Rgba,

    pub nav_button_hover: Rgba,
    pub nav_button_active: Rgba,

    pub playback_button: Rgba,
    pub playback_button_hover: Rgba,
    pub playback_button_active: Rgba,
    pub playback_button_border: Rgba,
    pub playback_button_toggled: Rgba,

    pub window_button: Rgba,
    pub window_button_hover: Rgba,
    pub window_button_active: Rgba,

    pub close_button: Rgba,
    pub close_button_hover: Rgba,
    pub close_button_active: Rgba,

    pub queue_item: Rgba,
    pub queue_item_hover: Rgba,
    pub queue_item_active: Rgba,
    pub queue_item_current: Rgba,

    pub button_primary: Rgba,
    pub button_primary_hover: Rgba,
    pub button_primary_active: Rgba,
    pub button_primary_text: Rgba,

    pub button_secondary: Rgba,
    pub button_secondary_hover: Rgba,
    pub button_secondary_active: Rgba,
    pub button_secondary_text: Rgba,

    pub button_warning: Rgba,
    pub button_warning_hover: Rgba,
    pub button_warning_active: Rgba,
    pub button_warning_text: Rgba,

    pub button_danger: Rgba,
    pub button_danger_hover: Rgba,
    pub button_danger_active: Rgba,
    pub button_danger_text: Rgba,

    pub slider_foreground: Rgba,
    pub slider_background: Rgba,

    pub elevated_background: Rgba,
    pub elevated_border_color: Rgba,

    pub menu_item: Rgba,
    pub menu_item_hover: Rgba,
    pub menu_item_active: Rgba,

    pub modal_overlay_bg: Rgba,

    pub text_input_selection: Rgba,
    pub caret_color: Rgba,

    pub palette_item_hover: Rgba,
    pub palette_item_active: Rgba,
}

impl Default for UsrTheme {
    fn default() -> Self {
        Self {
            background_primary: rgb(0x0C1116),
            background_secondary: rgb(0x161A22),
            background_tertiary: rgb(0x222831),

            border_color: rgb(0x272D37),

            album_art_background: rgb(0x4C5974),

            text: rgb(0xF4F5F6),
            text_secondary: rgb(0xBEC4CA),
            text_link: rgb(0x5279D4),
            text_about_desc: rgb(0xFFB6C1),

            nav_button_hover: rgb(0x161A22),
            nav_button_active: rgb(0x0A0E12),

            playback_button: rgba(0x282F3D00),
            playback_button_hover: rgb(0x282F3D),
            playback_button_active: rgb(0x0D1014),
            playback_button_border: rgba(0x37404E00),
            playback_button_toggled: rgb(0x0667B2),

            window_button: rgba(0x33415500),
            window_button_hover: rgb(0x282F3D),
            window_button_active: rgb(0x0D1014),

            queue_item: rgb(0x161A2200),
            queue_item_hover: rgb(0x161A22),
            queue_item_active: rgb(0x0C1116),
            queue_item_current: rgb(0x272D37),

            close_button: rgba(0x282F3D00),
            close_button_hover: rgb(0xAE0909),
            close_button_active: rgb(0x7A0606),

            button_primary: rgb(0x0667B2),
            button_primary_hover: rgb(0x087AD1),
            button_primary_active: rgb(0x065D9F),
            button_primary_text: rgb(0xE0F1FE),

            button_secondary: rgb(0x37404E),
            button_secondary_hover: rgb(0x495467),
            button_secondary_active: rgb(0x262C36),
            button_secondary_text: rgb(0xBEC4CA),

            button_warning: rgb(0xEDB407),
            button_warning_hover: rgb(0xF8C017),
            button_warning_active: rgb(0xD6A207),
            button_warning_text: rgb(0xFEF8E5),

            button_danger: rgb(0xCD0B0B),
            button_danger_hover: rgb(0xE80C0C),
            button_danger_active: rgb(0xB70A0A),
            button_danger_text: rgb(0xFEE3E3),

            slider_foreground: rgb(0x0673C6),
            slider_background: rgb(0x37404E),

            elevated_background: rgb(0x161A22),
            elevated_border_color: rgb(0x272D37),

            menu_item: rgba(0x282F3D00),
            menu_item_hover: rgb(0x282F3D),
            menu_item_active: rgb(0x0D1014),

            modal_overlay_bg: rgba(0x0C111655),

            text_input_selection: rgba(0x0673C688),
            caret_color: rgb(0xF4F5F6),

            palette_item_hover: rgb(0x282F3D),
            palette_item_active: rgb(0x0D1014),
        }
    }
}

impl Global for UsrTheme {}

pub fn create_theme(cx: &mut App, topic_theme_name: SharedString) {
    cx.set_global(UsrTheme::default());
    // Load and watch themes from ./themes directory
    if let Err(err) = ThemeRegistry::watch_dir(PathBuf::from("./themes"), cx, move |cx| {
        if let Some(theme) = ThemeRegistry::global(cx)
            .themes()
            .get(&topic_theme_name)
            .cloned()
        {
            Theme::global_mut(cx).apply_config(&theme);
        }
    }) {
        error!("Create theme error");
    }
}

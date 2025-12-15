use std::fs;

use directories::ProjectDirs;
use gpui::{
    App, AppContext, Application, Bounds, Context, IntoElement, ParentElement, Point, Render,
    SharedString, Styled, TitlebarOptions, Window, WindowBounds, WindowOptions, div, px, size,
};
use gpui_component::{
    Root, StyledExt,
    button::{Button, ButtonVariants},
};
use tracing::debug;

use crate::ui::assets::HappybirdAssetSource;

pub fn get_dirs() -> ProjectDirs {
    let secondary_dirs = directories::ProjectDirs::from("me", "zgy", "happybird")
        .expect("couldn't generate project dirs (secondary)");

    if secondary_dirs.data_dir().exists() {
        return secondary_dirs;
    }

    directories::ProjectDirs::from("org", "zgy", "happybird")
        .expect("couldn't generate project dirs")
}

pub fn find_fonts(cx: &mut App) -> gpui::Result<()> {
    let paths = cx.asset_source().list("!bundled:fonts")?;
    let mut fonts = vec![];
    for path in paths {
        if (path.ends_with(".ttf") || path.ends_with(".otf"))
            && let Some(v) = cx.asset_source().load(&path)?
        {
            fonts.push(v);
        }
    }

    let results = cx.text_system().add_fonts(fonts);
    debug!("loaded fonts: {:?}", cx.text_system().all_font_names());
    results
}

pub struct HelloWorld;

impl Render for HelloWorld {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .v_flex()
            .gap_2()
            .size_full()
            .items_center()
            .justify_center()
            .child("Hello, World!")
            .child(
                Button::new("ok")
                    .primary()
                    .label("Let's Go!")
                    .on_click(|_, _, _| println!("Clicked!")),
            )
    }
}

pub fn run() -> anyhow::Result<()> {
    let dirs = get_dirs();
    let data_dir = dirs.data_dir().to_path_buf();
    fs::create_dir_all(&data_dir).inspect_err(|error| {
        tracing::error!(
            ?error,
            "couldn't create data directory '{}'",
            data_dir.display(),
        )
    })?;

    // Create database pool
    // let pool = crate::RUNTIME
    //     .block_on(create_pool(data_dir.join("library.db")))
    //     .inspect_err(|error| {
    //         tracing::error!(?error, "fatal: unable to create database pool");
    //     })?;

    // let app = Application::new().with_assets(HappybirdAssetSource::new());
    let app = Application::new().with_assets(gpui_component_assets::Assets);

    app.run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_component::init(cx);
        let bounds = Bounds::centered(None, size(px(1024.0), px(700.0)), cx);
        find_fonts(cx).expect("unable to load fonts");
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some(SharedString::from("Happybird")),
                    appears_transparent: true,
                    traffic_light_position: Some(Point {
                        x: px(12.0),
                        y: px(11.0),
                    }),
                }),
                kind: gpui::WindowKind::Normal,
                window_background: gpui::WindowBackgroundAppearance::Opaque,
                app_id: Some("me.zgy.happybird".to_string()),
                window_decorations: Some(gpui::WindowDecorations::Client),
                ..Default::default()
            },
            |window, cx| {
                window.set_window_title("Happybird");

            },
        )
        .unwrap();
    });

    Ok(())
}

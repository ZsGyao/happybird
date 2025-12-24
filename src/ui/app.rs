use std::{fs, sync::Arc};

use directories::ProjectDirs;
use gpui::{
    App, AppContext, Application, Bounds, Context, CursorStyle, Decorations, Entity, ExternalPaths,
    FontFeatures, Hsla, InteractiveElement, IntoElement, MouseButton, ParentElement, Pixels, Point,
    Render, ResizeEdge, SharedString, Size, Styled, TextStyleRefinement, Tiling, TitlebarOptions,
    Window, WindowBackgroundAppearance, WindowBounds, WindowDecorations, WindowKind, WindowOptions,
    canvas, div, point, prelude::FluentBuilder, px, rgba, size, transparent_black,
};

use gpui_component::{ActiveTheme, Root};
use gpui_component_assets::Assets;
use tracing::debug;

use crate::ui::{
    about::about_dialog,
    constants::{APP_ROUNDING, APP_SHADOW_SIZE},
    custom_sidebar::CustomSidebar,
    header::Header,
    models::{Models, build_models},
    theme::create_theme,
    watch_list::WatchList,
};

pub fn get_dirs() -> ProjectDirs {
    let secondary_dirs = directories::ProjectDirs::from("me", "zgy", "happybird")
        .expect("couldn't generate project dirs (secondary)");

    if secondary_dirs.data_dir().exists() {
        return secondary_dirs;
    }

    directories::ProjectDirs::from("org", "zgy", "happybird")
        .expect("couldn't generate project dirs")
}

#[allow(dead_code)]
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

struct WindowShadow {
    pub show_about: Entity<bool>,
    pub header: Entity<Header>,
    pub sidebar: Entity<CustomSidebar>,
    pub watch_list: Entity<WatchList>,
}

impl Render for WindowShadow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let decorations = window.window_decorations();
        let rounding = APP_ROUNDING;
        let shadow_size = APP_SHADOW_SIZE;
        let border_size = px(1.0);
        window.set_client_inset(shadow_size);

        let show_about = *self.show_about.clone().read(cx);

        // let state = FileBrowserDelegate::new(cx, window);

        let mut element = div()
            .id("window-backdrop")
            .key_context("app")
            .bg(transparent_black())
            .flex()
            .map(|div| match decorations {
                gpui::Decorations::Server => div,
                gpui::Decorations::Client { tiling } => div
                    .bg(transparent_black())
                    .child(
                        canvas(
                            |_bounds, window, _| {
                                window.insert_hitbox(
                                    Bounds::new(
                                        Point {
                                            x: px(0.0),
                                            y: px(0.0),
                                        },
                                        window.window_bounds().get_bounds().size,
                                    ),
                                    gpui::HitboxBehavior::Normal,
                                )
                            },
                            move |_bounds, hitbox, window, _| {
                                let mouse = window.mouse_position();
                                let size = window.window_bounds().get_bounds().size;
                                let Some(edge) = resize_edge(mouse, px(30.0), size, tiling) else {
                                    return;
                                };
                                window.set_cursor_style(
                                    match edge {
                                        ResizeEdge::Top | ResizeEdge::Bottom => {
                                            CursorStyle::ResizeUpDown
                                        }
                                        ResizeEdge::Left | ResizeEdge::Right => {
                                            CursorStyle::ResizeLeftRight
                                        }
                                        ResizeEdge::TopLeft | ResizeEdge::BottomRight => {
                                            CursorStyle::ResizeUpLeftDownRight
                                        }
                                        ResizeEdge::TopRight | ResizeEdge::BottomLeft => {
                                            CursorStyle::ResizeUpRightDownLeft
                                        }
                                    },
                                    &hitbox,
                                );
                            },
                        )
                        .size_full()
                        .absolute(),
                    )
                    .when(!(tiling.top || tiling.right), |div| {
                        div.rounded_tr(rounding)
                    })
                    .when(!(tiling.top || tiling.left), |div| div.rounded_tl(rounding))
                    .when(!(tiling.bottom || tiling.right), |div| {
                        div.rounded_br(rounding)
                    })
                    .when(!(tiling.bottom || tiling.left), |div| {
                        div.rounded_bl(rounding)
                    })
                    .when(!tiling.top, |div| div.pt(shadow_size))
                    .when(!tiling.bottom, |div| div.pb(shadow_size))
                    .when(!tiling.left, |div| div.pl(shadow_size))
                    .when(!tiling.right, |div| div.pr(shadow_size))
                    .on_mouse_down(MouseButton::Left, move |e, window, _| {
                        let size = window.window_bounds().get_bounds().size;
                        let pos = e.position;

                        if let Some(edge) = resize_edge(pos, shadow_size, size, tiling) {
                            window.start_window_resize(edge)
                        };
                    }),
            })
            .size_full()
            .child(
                div()
                    .font_family("Inter")
                    .text_color(cx.theme().chart_1)
                    .cursor(CursorStyle::Arrow)
                    .map(|div| match decorations {
                        Decorations::Server => div,
                        Decorations::Client { tiling } => div
                            .when(cfg!(not(target_os = "macos")), |div| {
                                div.border_color(rgba(0x64748b33))
                            })
                            .when(!(tiling.top || tiling.right), |div| {
                                div.rounded_tr(rounding)
                            })
                            .when(!(tiling.top || tiling.left), |div| div.rounded_tl(rounding))
                            .when(!(tiling.bottom || tiling.right), |div| {
                                div.rounded_br(rounding)
                            })
                            .when(!(tiling.bottom || tiling.left), |div| {
                                div.rounded_bl(rounding)
                            })
                            .when(!tiling.top, |div| div.border_t(border_size))
                            .when(!tiling.bottom, |div| div.border_b(border_size))
                            .when(!tiling.left, |div| div.border_l(border_size))
                            .when(!tiling.right, |div| div.border_r(border_size))
                            .when(!tiling.is_tiled(), |div| {
                                div.shadow(vec![gpui::BoxShadow {
                                    color: Hsla {
                                        h: 0.,
                                        s: 0.,
                                        l: 0.,
                                        a: 0.4,
                                    },
                                    blur_radius: shadow_size / 2.,
                                    spread_radius: px(0.),
                                    offset: point(px(0.0), px(0.0)),
                                }])
                            }),
                    })
                    .on_mouse_move(|_e, _, cx| {
                        cx.stop_propagation();
                    })
                    .on_drop(|_ev: &ExternalPaths, _, _cx| {})
                    .overflow_hidden()
                    .bg(cx.theme().background)
                    .size_full()
                    .flex()
                    .flex_col()
                    .max_w_full()
                    .max_h_full()
                    .child(self.header.clone())
                    .child(
                        div()
                            .flex()
                            .h_full()
                            .w_full()
                            .child(self.sidebar.clone())
                            .when(*cx.global::<Models>().show_folder.clone().read(cx), |div| {
                                div.child(self.watch_list.clone())
                            }), //.child(List::new(&state)),
                    )
                    .when(show_about, |this| {
                        this.child(about_dialog(&|_, cx| {
                            let show_about = cx.global::<Models>().show_about.clone();
                            debug!("Folder show about exit");
                            show_about.write(cx, false);
                        }))
                    }),
            );

        let text_styles = element.text_style();
        *text_styles = Some(TextStyleRefinement::default());

        let ff = &mut text_styles.as_mut().unwrap().font_features;
        *ff = Some(FontFeatures(Arc::new(vec![("tnum".to_string(), 1)])));

        element
    }
}

fn resize_edge(
    pos: Point<Pixels>,
    shadow_size: Pixels,
    size: Size<Pixels>,
    tiling: Tiling,
) -> Option<ResizeEdge> {
    let edge = if pos.y < shadow_size * 2 && pos.x < shadow_size * 2 && !tiling.top && !tiling.left
    {
        ResizeEdge::TopLeft
    } else if pos.y < shadow_size * 2
        && pos.x > size.width - shadow_size * 2
        && !tiling.top
        && !tiling.right
    {
        ResizeEdge::TopRight
    } else if pos.y < shadow_size && !tiling.top {
        ResizeEdge::Top
    } else if pos.y > size.height - shadow_size * 2
        && pos.x < shadow_size * 2
        && !tiling.bottom
        && !tiling.left
    {
        ResizeEdge::BottomLeft
    } else if pos.y > size.height - shadow_size * 2
        && pos.x > size.width - shadow_size * 2
        && !tiling.bottom
        && !tiling.right
    {
        ResizeEdge::BottomRight
    } else if pos.y > size.height - shadow_size && !tiling.bottom {
        ResizeEdge::Bottom
    } else if pos.x < shadow_size && !tiling.left {
        ResizeEdge::Left
    } else if pos.x > size.width - shadow_size && !tiling.right {
        ResizeEdge::Right
    } else {
        return None;
    };
    Some(edge)
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
    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_component::init(cx);

        let bounds = Bounds::centered(None, size(px(1024.0), px(700.0)), cx);

        // find_fonts(cx).expect("unable to load fonts");
        create_theme(cx, SharedString::from("Hybrid Dark"));
        build_models(cx);
        cx.activate(true);

        let win_ops = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            window_background: WindowBackgroundAppearance::Opaque,
            window_decorations: Some(WindowDecorations::Client),
            window_min_size: Some(size(px(800.0), px(600.0))),
            titlebar: Some(TitlebarOptions {
                title: Some(SharedString::from("Happybird")),
                appears_transparent: true,
                traffic_light_position: Some(Point {
                    x: px(12.0),
                    y: px(11.0),
                }),
            }),
            app_id: Some("org.zgy.happybird".to_string()),
            kind: WindowKind::Normal,
            ..Default::default()
        };
        cx.open_window(win_ops, |window, cx| {
            window.set_window_title("Happybird");

            cx.new(|cx| {
                cx.observe_window_appearance(window, |_, _, cx| {
                    cx.refresh_windows();
                })
                .detach();

                let show_about = cx.global::<Models>().show_about.clone();
                let show_folder = cx.global::<Models>().show_folder.clone();
                cx.observe(&show_about, |_, _, cx| {
                    cx.notify();
                })
                .detach();
                cx.observe(&show_folder, |_, _, cx| {
                    cx.notify();
                })
                .detach();

                let view = cx.new(|cx| WindowShadow {
                    show_about,
                    header: Header::new(cx),
                    sidebar: CustomSidebar::new(cx),
                    watch_list: WatchList::new(cx, window),
                });
                Root::new(view, window, cx)
            })
        })
        .unwrap();
    });

    Ok(())
}

use std::sync::Arc;

use gpui::{prelude::FluentBuilder, *};

use crate::{
    debug,
    ui::{
        assets::HappybirdAsset,
        constants::{APP_LEFT_PANEL_INIT_W, APP_RIGHT_PANEL_INIT_W},
        detail_panel::DetailPanel,
        import_panel::ImportPanel,
        info_panel::InfoPanel,
        models::GlobalAppState,
        test_ui::HappyBirdComponentTest,
        theme,
    },
};
use gpui_component::{
    ActiveTheme, Root, StyledExt,
    resizable::{h_resizable, resizable_panel},
};

use crate::ui::{
    about::about_dialog,
    constants::{APP_ROUNDING, APP_SHADOW_SIZE},
    header::Header,
    models::build_models,
};

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

pub struct WindowShadow {
    pub header: Entity<Header>,
    pub info_panel: Entity<InfoPanel>,
    pub import_panel: Option<Entity<ImportPanel>>,
    pub test_table: Entity<HappyBirdComponentTest>,
    pub detail_panel: Entity<DetailPanel>,
}

impl Render for WindowShadow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let decorations = window.window_decorations();
        let rounding = APP_ROUNDING;
        let shadow_size = APP_SHADOW_SIZE;
        let border_size = px(1.0);
        window.set_client_inset(shadow_size);

        // cala size
        let center_init_size =
            window.bounds().size.width - APP_LEFT_PANEL_INIT_W - APP_RIGHT_PANEL_INIT_W;

        let show_about = cx.global::<GlobalAppState>().0.read(cx).show_about;
        let show_test = cx.global::<GlobalAppState>().0.read(cx).show_test;

        // 1. 获取开关状态
        let global = cx.global::<GlobalAppState>().0.read(cx);
        let show = global.import_preview_state.show_import_modal;
        let is_loading = global.import_preview_state.is_importing; // 你可以在界面上根据这个显示个 Loading

        // 2. 【核心逻辑】开关开了才创建，关了就销毁
        if show {
            // 如果开关是开的，但还没创建过 -> 创建它！
            // 此时 Model 里肯定已经有数据了，因为你是先 set_data 后 set_show 的
            if self.import_panel.is_none() {
                self.import_panel = Some(ImportPanel::new(window, cx));
            }
        } else {
            // 开关关了 -> 扔掉，释放内存
            self.import_panel = None;
        }

        let mut element = div()
            .id("window-backdrop")
            .key_context("app")
            .bg(cx.theme().background)
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
            .size_full() // Set window render finish
            .child(
                div()
                    .font_family("Inter")
                    .text_color(cx.theme().colors.foreground)
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
                    }) // 当有文件被拖到窗口上的行为，现在为空
                    .overflow_hidden()
                    .bg(cx.theme().colors.background)
                    .size_full()
                    .flex()
                    .v_flex()
                    .max_w_full()
                    .max_h_full()
                    .child(self.header.clone()) // 从此，窗口被绘制完成
                    .child(
                        div().flex().h_flex().h_full().child(
                            h_resizable("center-dock")
                                .child(
                                    resizable_panel()
                                        .size(px(260.0))
                                        .size_range(px(180.0)..Pixels::MAX)
                                        .child(self.info_panel.clone()),
                                )
                                .child(resizable_panel().size(center_init_size).child(
                                    div().w_full().v_flex().child(
                                        div().flex_1().size_full().child(self.detail_panel.clone()),
                                    ),
                                ))
                                .child(
                                    resizable_panel()
                                        .size(APP_RIGHT_PANEL_INIT_W)
                                        .size_range(px(80.0)..Pixels::MAX)
                                        .child(
                                            div()
                                                .w_full()
                                                .v_flex()
                                                .child(
                                                    div()
                                                        .h(px(40.0))
                                                        .border_t_1()
                                                        .border_color(cx.theme().colors.border)
                                                        .child("PROPERTY AUDIT"),
                                                )
                                                .child("ITEM VIEW"),
                                        ),
                                ),
                        ),
                    )
                    .when(show_about, |this| {
                        this.child(about_dialog(&|_, cx| {
                            cx.global::<GlobalAppState>()
                                .0
                                .clone()
                                .update(cx, |val, _| {
                                    val.show_about = !val.show_about;
                                });
                            debug!("Folder show about exit");
                        }))
                    })
                    .when_some(self.import_panel.clone(), |this, panel| {
                        this.child(div().absolute().size_full().child(panel))
                    })
                    .when(show_test, |this| this.child(self.test_table.clone())),
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
    // Create database pool
    // let pool = crate::RUNTIME
    //     .block_on(create_pool(data_dir.join("library.db")))
    //     .inspect_err(|error| {
    //         tracing::error!(?error, "fatal: unable to create database pool");
    //     })?;

    let app = Application::new().with_assets(HappybirdAsset);
    // let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_component::init(cx);
        // HappybirdAsset.load_fonts(cx).unwrap();
        let bounds = Bounds::centered(None, size(px(1024.0), px(700.0)), cx);

        find_fonts(cx).expect("unable to load fonts");
        theme::init(cx);

        build_models(cx);
        cx.activate(true);

        let win_ops = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)), // 设置窗口的初始位置和尺寸
            window_background: WindowBackgroundAppearance::Opaque, //定义窗口的背景样式: Opaque: 不透明，Transparent：透明，Blurred：毛玻璃
            window_decorations: Some(WindowDecorations::Client), // 控制窗口的“装饰”，即边框、标题栏和标准窗口按钮（关闭、最小化、最大化) Client` 表示**由客户端（也就是你的应用程序）来绘制**这些装饰。这通常用于实现自定义的、非原生外观的标题栏
            window_min_size: Some(size(px(1400.0), px(800.0))),
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

                let models_handle = cx.global::<GlobalAppState>().0.clone();
                cx.observe(&models_handle, |_, _, cx| {
                    cx.notify();
                })
                .detach();

                let view = cx.new(|cx| WindowShadow {
                    header: Header::new(cx),
                    info_panel: InfoPanel::new(window, cx),
                    import_panel: None,
                    test_table: HappyBirdComponentTest::new(cx, window),
                    detail_panel: DetailPanel::new(cx),
                });
                Root::new(view, window, cx)
            })
        })
        .unwrap();
    });

    Ok(())
}

use gpui::{prelude::FluentBuilder, *};
use tracing::debug;

use crate::ui::{
    assets::HappybirdAsset,
    constants::{APP_LEFT_PANEL_INIT_W, APP_RIGHT_PANEL_INIT_W},
    detail_panel::DetailPanel,
    export_modal::render_export_modal,
    import_panel::ImportPanel,
    info_panel::InfoPanel,
    lock_screen::LockScreen,
    models::GlobalAppState,
    set_password_modal::SetPasswordModal,
    sidebar::HappyBirdSideBar,
    status_bar::StatusBar,
    test_ui::HappyBirdComponentTest,
    theme,
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
    results
}

pub struct WindowShadow {
    pub header: Entity<Header>,
    pub info_panel: Entity<InfoPanel>,
    pub import_panel: Option<Entity<ImportPanel>>,
    pub test_table: Entity<HappyBirdComponentTest>,
    pub detail_panel: Entity<DetailPanel>,
    pub status_bar: Entity<StatusBar>,
    pub sidebar: Entity<HappyBirdSideBar>,
    pub set_password_screen: Entity<SetPasswordModal>,
    pub lock_screen: Entity<LockScreen>,
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
        // let is_loading = global.import_preview_state.is_importing; // 你可以在界面上根据这个显示个 Loading

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

        // 读取当前页面状态
        let current_page = cx
            .global::<GlobalAppState>()
            .0
            .read(cx)
            .current_page
            .clone();

        // 读取折叠状态
        let is_collapsed = cx
            .global::<GlobalAppState>()
            .0
            .read(cx)
            .is_sidebar_collapsed;
        let sidebar_width = if is_collapsed { px(56.0) } else { px(180.0) };

        let is_locked = cx
            .global::<GlobalAppState>()
            .0
            .read(cx)
            .lock_state
            .is_locked;

        let show_set_password_modal = cx
            .global::<GlobalAppState>()
            .0
            .read(cx)
            .lock_state
            .show_set_password_modal;

        let element = div()
            .id("window-backdrop")
            .key_context("app")
            .bg(gpui::transparent_black())
            .flex()
            .map(|div| match decorations {
                gpui::Decorations::Server => div,
                gpui::Decorations::Client { tiling } => div
                    .bg(gpui::transparent_black())
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
            // =========================================================
            //  主内容容器 (Main Container)
            // =========================================================
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
                                    color: cx.theme().colors.background,
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
                        div()
                            .flex()
                            .h_flex()
                            .size_full()
                            .overflow_hidden()
                            // SideBar: 固定在最左侧
                            .child(
                                div()
                                    .w(sidebar_width) // 固定宽度
                                    .h_full()
                                    .flex_shrink_0() // 不允许压缩
                                    .child(self.sidebar.clone()), // 渲染 SideBar
                            )
                            .child(
                                div()
                                    .flex_1() // 占据剩余空间
                                    .h_full()
                                    .child(match current_page {
                                        crate::ui::models::AppPage::Users => {
                                            h_resizable("center-dock")
                                                .child(
                                                    resizable_panel()
                                                        .size(px(260.0))
                                                        .size_range(px(180.0)..Pixels::MAX)
                                                        .child(
                                                            div()
                                                                .size_full()
                                                                .overflow_hidden()
                                                                .child(self.info_panel.clone()),
                                                        ),
                                                )
                                                .child(
                                                    resizable_panel().size(center_init_size).child(
                                                        div()
                                                            .size_full()
                                                            .overflow_hidden()
                                                            .child(self.detail_panel.clone()),
                                                    ),
                                                )
                                        }
                                    }),
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
                    .child(
                        div()
                            .flex_shrink_0() // 防止被压缩
                            .child(self.status_bar.clone()),
                    )
                    .children(render_export_modal(cx))
                    .when(is_locked, |this| this.child(self.lock_screen.clone()))
                    .when(show_set_password_modal, |this| {
                        this.child(self.set_password_screen.clone())
                    })
                    .when(show_test, |this| this.child(self.test_table.clone())),
            );

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
    let app = Application::new().with_assets(HappybirdAsset);

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
            window_background: WindowBackgroundAppearance::Transparent, //定义窗口的背景样式: Opaque: 不透明，Transparent：透明，Blurred：毛玻璃
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
                    status_bar: StatusBar::new(cx),
                    sidebar: HappyBirdSideBar::new(cx),
                    set_password_screen: SetPasswordModal::new(window, cx),
                    lock_screen: LockScreen::new(window, cx),
                });
                Root::new(view, window, cx)
            })
        })
        .unwrap();
    });

    Ok(())
}

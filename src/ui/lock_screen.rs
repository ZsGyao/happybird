// src/ui/lock/lock_screen.rs

use gpui::{
    App, AppContext, Context, Entity, FontWeight, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Render, Styled, Subscription, Window, black, div, px,
};
use gpui_component::{
    ActiveTheme, Icon, Sizable,
    button::{Button, ButtonVariants},
    h_flex,
    input::{Input, InputEvent, InputState},
    label::Label,
    v_flex,
};

use crate::ui::{hb_icons::HappyBirdIcons, models::GlobalAppState};

/// 应用锁定屏幕组件。
///
/// 当应用处于锁定状态时，此组件会以全屏遮罩的形式覆盖在最上层，
/// 要求用户输入密码进行解锁。
pub struct LockScreen {
    /// 密码输入框的状态。
    password_input_state: Entity<InputState>,
    /// 是否显示密码错误的提示。
    show_error: bool,
    _lockscreen_subscription: Subscription,
}

impl LockScreen {
    /// 创建一个新的 `LockScreen` 实例。
    pub fn new(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let password_input_state = cx.new(|cx| {
                InputState::new(window, cx)
                    .masked(true)
                    .placeholder("Enter Password")
            });

            let _lockscreen_subscription = cx.subscribe_in(
                &password_input_state,
                window,
                |this: &mut Self, _state, event: &InputEvent, window: &mut Window, cx| {
                    match event {
                        // 监听回车键
                        InputEvent::PressEnter { .. } => {
                            this.attempt_unlock(window, cx);
                        }
                        _ => {}
                    }
                },
            );

            Self {
                password_input_state,
                show_error: false,
                _lockscreen_subscription,
            }
        })
    }

    /// 执行解锁操作。
    fn attempt_unlock(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let password_attempt = self.password_input_state.read(cx).text().to_string();
        let global_model = cx.global::<GlobalAppState>().0.clone();

        let success = global_model.update(cx, |model, cx| {
            model.unlock_with_password(&password_attempt, cx)
        });

        if success {
            self.password_input_state.update(cx, |state, cx| {
                state.set_value("", window, cx); // 清空输入
            });
            self.show_error = false;
        } else {
            self.show_error = true;
            self.password_input_state.update(cx, |state, cx| {
                state.set_value("", window, cx); // 清空输入
            });
        }
        cx.notify();
    }
}

impl Render for LockScreen {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .absolute()
            .inset_0()
            .bg(cx.theme().colors.background)
            .flex()
            .items_center()
            .justify_center()
            // 拦截点击
            .on_mouse_down(MouseButton::Left, |_, _, cx: &mut App| {
                cx.stop_propagation()
            })
            .on_mouse_up(MouseButton::Left, |_, _, cx: &mut App| {
                cx.stop_propagation()
            })
            // 拦截右键
            .on_mouse_down(MouseButton::Right, |_, _, cx: &mut App| {
                cx.stop_propagation()
            })
            // 拦截鼠标移动 (这就是解决 Hover 穿透的关键)
            .on_mouse_move(|_, _, cx: &mut App| cx.stop_propagation())
            // 拦截滚轮 (防止底层列表滚动)
            .on_scroll_wheel(|_, _, cx: &mut App| cx.stop_propagation())
            .child(
                v_flex()
                    .w(px(380.0))
                    .bg(cx.theme().colors.background.opacity(0.95))
                    .border_1()
                    .border_color(cx.theme().colors.border)
                    .rounded_2xl()
                    .shadow_xl()
                    .p(px(40.0))
                    .items_center()
                    .gap(px(32.0))
                    // 1. 顶部 Logo 和状态区
                    .child(
                        v_flex()
                            .items_center()
                            .gap(px(16.0))
                            .child(
                                div()
                                    .p(px(12.0))
                                    .bg(cx.theme().colors.primary.opacity(0.1))
                                    .rounded_full()
                                    .child(
                                        Icon::new(HappyBirdIcons::Lock.load(cx))
                                            .size(px(48.0))
                                            .text_color(cx.theme().colors.primary),
                                    ),
                            )
                            .child(
                                Label::new("HappyBird is Locked")
                                    .font_weight(FontWeight::BOLD)
                                    .text_xl(),
                            )
                            .child(
                                Label::new("Enter your password to continue.")
                                    .text_color(cx.theme().colors.muted_foreground)
                                    .text_sm(),
                            ),
                    )
                    // 2. 密码输入区
                    .child(
                        v_flex()
                            .w_full()
                            .gap(px(12.0))
                            .child(
                                // [关键] 传递 &Entity<InputState>
                                Input::new(&self.password_input_state).large(),
                            )
                            // 错误提示
                            .child(div().h(px(20.0)).flex().justify_center().child(
                                if self.show_error {
                                    Label::new("Incorrect password provided.")
                                        // [修复] 使用 gpui::red()
                                        .text_color(gpui::red())
                                        .text_sm()
                                        .font_weight(FontWeight::SEMIBOLD)
                                } else {
                                    Label::new(" ").text_sm()
                                },
                            )),
                    )
                    // 3. 解锁按钮 (保持不变)
                    .child(
                        Button::new("unlock-btn")
                            .label("Unlock Application")
                            .primary()
                            .large()
                            .w_full()
                            .on_click(
                                cx.listener(|this, _, window, cx| this.attempt_unlock(window, cx)),
                            ),
                    )
                    // 4. 生物识别入口
                    .child(
                        v_flex()
                            // ... (标签保持不变)
                            .pt(px(16.0))
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                Label::new("Or unlock with")
                                    .text_xs()
                                    .text_color(cx.theme().colors.muted_foreground),
                            )
                            .child(
                                h_flex()
                                    .gap(px(16.0))
                                    // [修复] 将 Icon 包裹在 div 中以使用 tooltip
                                    .child(
                                        div().id("fingerprint-unlock").child(
                                            Icon::new(HappyBirdIcons::FingerprintPattern.load(cx))
                                                .size(px(24.0))
                                                .text_color(
                                                    cx.theme().colors.muted_foreground.opacity(0.5),
                                                )
                                                .cursor_not_allowed(),
                                        ),
                                    )
                                    // [修复] 将 Icon 包裹在 div 中以使用 tooltip
                                    .child(
                                        div().id("face-unlock").child(
                                            Icon::new(HappyBirdIcons::ScanFace.load(cx))
                                                .size(px(24.0))
                                                .text_color(
                                                    cx.theme().colors.muted_foreground.opacity(0.5),
                                                )
                                                .cursor_not_allowed(),
                                        ),
                                    ),
                            ),
                    ),
            )
    }
}

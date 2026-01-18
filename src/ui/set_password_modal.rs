// src/ui/lock/set_password_modal.rs

use gpui::{
    App, AppContext, Context, Entity, FontWeight, InteractiveElement, IntoElement, KeyDownEvent,
    ParentElement, Render, StatefulInteractiveElement, Styled, Subscription, Window, black, div,
    px,
};
use gpui_component::{
    ActiveTheme,
    button::{Button, ButtonVariants},
    h_flex,
    input::{Input, InputEvent, InputState}, // [关键] 使用 Input 组件
    label::Label,
    v_flex,
};

use crate::ui::models::GlobalAppState;

/// 设置密码模态弹窗组件。
///
/// 用于在用户首次点击锁定时，引导用户设置一个访问密码。
/// 需要用户输入两次密码以确认。
pub struct SetPasswordModal {
    /// 新密码输入框的状态。
    new_password_state: Entity<InputState>,
    /// 确认密码输入框的状态。
    confirm_password_state: Entity<InputState>,
    /// 是否显示错误提示（如两次输入不一致）。
    error_message: Option<String>,
    _password_subscription: Subscription,
}

impl SetPasswordModal {
    /// 创建一个新的 `SetPasswordModal` 实例。
    pub fn new(window: &mut Window, cx: &mut App) -> Entity<Self> {
        let new_password_state = cx.new(|cx| {
            InputState::new(window, cx)
                .masked(true)
                .placeholder("Enter new password")
        });

        let confirm_password_state = cx.new(|cx| {
            InputState::new(window, cx)
                .masked(true)
                .placeholder("Re-enter to confirm")
        });

        cx.new(|cx| {
            let _password_subscription = cx.subscribe_in(
                &confirm_password_state,
                window,
                |this: &mut Self, _state, event, _window, cx| match event {
                    InputEvent::Change => {}
                    InputEvent::PressEnter { .. } => {
                        this.submit(cx);
                    }
                    InputEvent::Focus => {}
                    InputEvent::Blur => {}
                },
            );
            Self {
                new_password_state,
                confirm_password_state,
                error_message: None,
                _password_subscription,
            }
        })
    }

    /// 尝试提交新密码。
    fn submit(&mut self, cx: &mut Context<Self>) {
        let new_pwd = self.new_password_state.read(cx).text().to_string();
        let confirm_pwd = self.confirm_password_state.read(cx).text().to_string();

        if new_pwd.is_empty() {
            self.error_message = Some("Password cannot be empty.".to_string());
        } else if new_pwd != confirm_pwd {
            self.error_message = Some("Passwords do not match.".to_string());
        } else {
            // 验证通过，调用全局 Model 保存密码
            self.error_message = None;
            let g = cx.global::<GlobalAppState>().0.clone();
            g.update(cx, |model, cx| model.set_password(&new_pwd, cx));
        }
        cx.notify();
    }

    /// 取消设置，关闭弹窗。
    fn cancel(&mut self, cx: &mut Context<Self>) {
        let g = cx.global::<GlobalAppState>().0.clone();
        g.update(cx, |model, _cx| {
            model.lock_state.show_set_password_modal = false
        });
    }
}

impl Render for SetPasswordModal {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .absolute()
            .inset_0()
            .bg(black().opacity(0.5))
            .flex()
            .items_center()
            .justify_center()
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if event.keystroke.key == "enter" {
                    this.submit(cx);
                }
            }))
            .child(
                v_flex()
                    .id("pawssard-card")
                    .w(px(400.0))
                    .bg(cx.theme().colors.background)
                    .border_1()
                    .border_color(cx.theme().colors.border)
                    .rounded_xl()
                    .shadow_lg()
                    .p(px(24.0))
                    .gap(px(20.0))
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(
                        Label::new("Set App Password")
                            .font_weight(FontWeight::BOLD)
                            .text_lg(),
                    )
                    .child(
                        v_flex()
                            .gap(px(16.0))
                            // 新密码输入框
                            .child(
                                v_flex()
                                    .gap(px(8.0))
                                    .child(Label::new("New Password").text_sm())
                                    .child(Input::new(&self.new_password_state)),
                            )
                            // 确认密码输入框
                            .child(
                                v_flex()
                                    .gap(px(8.0))
                                    .child(Label::new("Confirm Password").text_sm())
                                    .child(Input::new(&self.confirm_password_state)),
                            )
                            // 错误提示区域
                            .child(div().h(px(20.0)).child(
                                if let Some(msg) = &self.error_message {
                                    Label::new(msg.clone())
                                        // [修复] 使用 gpui::red() 代替不存在的主题色
                                        .text_color(gpui::red())
                                        .text_sm()
                                } else {
                                    Label::new(" ").text_sm()
                                },
                            )),
                    )
                    // 按钮操作区域
                    .child(
                        h_flex()
                            .justify_end()
                            .gap(px(12.0))
                            .child(
                                Button::new("cancel-set-pwd")
                                    .label("Cancel")
                                    .ghost()
                                    .on_click(cx.listener(|this, _, _, cx| this.cancel(cx))),
                            )
                            .child(
                                Button::new("save-pwd")
                                    .label("Set Password")
                                    .primary()
                                    .on_click(cx.listener(|this, _, _, cx| this.submit(cx))),
                            ),
                    ),
            )
    }
}

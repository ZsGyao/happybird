use gpui::{
    App, AppContext, Context, Entity, InteractiveElement, IntoElement, MouseButton, ParentElement,
    Render, ScrollHandle, StatefulInteractiveElement, Styled, div, px, size,
};
use gpui_component::{ActiveTheme, StyledExt, button::Button, h_flex, v_flex};

use crate::ui::models::GlobalAppState;

pub struct ImportPanel {
    scroll_state: ScrollHandle,
}

impl ImportPanel {
    pub fn new(cx: &mut App) -> Entity<Self> {
        cx.new(|_cx| Self {
            scroll_state: ScrollHandle::new(),
        })
    }
}

impl Render for ImportPanel {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) -> impl gpui::IntoElement {
        let global = cx.global::<GlobalAppState>().0.clone();
        let model = global.read(cx);

        // if !model.show_import_modal || model.import_preview_data.is_none() {
        //     return div().into_any_element();
        // }

        let data = model.import_preview_data.as_ref().unwrap();
        let headers: Vec<String> = if let Some(first) = data.first() {
            first.keys().cloned().collect()
        } else {
            vec![]
        };

        // 4. 构建 UI
        div()
            .id("import-panel")
            .absolute()
            .size_full()
            .bg(gpui::black().opacity(0.5)) // 遮罩
            .flex()
            .items_center()
            .justify_center()
            // 阻止点击事件穿透到下层
            // .on_mouse_down(MouseButton::Left, |_, cx| cx.stop_propagation())
            .child(
                v_flex()
                    .w(px(700.0))
                    .h(px(500.0))
                    .bg(cx.theme().colors.background)
                    .border_1()
                    .border_color(cx.theme().colors.border)
                    .rounded_xl()
                    .shadow_xl()
                    .p(px(20.0))
                    .gap(px(16.0))
                    // --- Header ---
                    .child(
                        h_flex()
                            .justify_between()
                            .child(div().font_bold().text_xl().child("Preview Data"))
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().colors.foreground)
                                    .child(format!("{} rows found", data.len())),
                            ),
                    )
                    // --- Table Area ---
                    .child(
                        v_flex()
                            .flex_1()
                            .border_1()
                            .border_color(cx.theme().colors.border)
                            .rounded_md()
                            .overflow_hidden()
                            .child(
                                div()
                                    .id("preview-table")
                                    .size_full()
                                    .overflow_scroll()
                                    .track_scroll(&self.scroll_state) // 绑定滚动句柄
                                    .child(
                                        v_flex()
                                            .min_w_full() // 确保内容撑开
                                            // Table Header
                                            .child(
                                                h_flex()
                                                    .bg(cx.theme().colors.secondary)
                                                    .font_bold()
                                                    .p(px(8.0))
                                                    .border_b_1()
                                                    .border_color(cx.theme().colors.border)
                                                    .children(headers.iter().map(|h| {
                                                        div()
                                                            .w(px(120.0))
                                                            .flex_shrink_0()
                                                            .child(h.clone())
                                                    })),
                                            )
                                            // Table Body (使用 children 迭代生成)
                                            .children(data.iter().take(100).map(|row| {
                                                // 限制渲染 100 行预览，防止卡顿
                                                h_flex()
                                                    .border_b_1()
                                                    .border_color(cx.theme().colors.border)
                                                    .p(px(8.0))
                                                    .children(headers.iter().map(|h| {
                                                        let val = row
                                                            .get(h)
                                                            .map(|v| v.to_string())
                                                            .unwrap_or_default();
                                                        div()
                                                            .w(px(120.0))
                                                            .flex_shrink_0()
                                                            .text_sm()
                                                            .text_ellipsis()
                                                            .child(val)
                                                    }))
                                            })),
                                    ),
                            ),
                    )
                    // --- Footer Buttons ---
                    .child(
                        h_flex()
                            .justify_end()
                            .gap(px(12.0))
                            .child(Button::new("cancel-import").label("Cancel").on_click(
                                |_, _, cx| {
                                    let global = cx.global::<GlobalAppState>().0.clone();
                                    global.update(cx, |m, cx| m.cancel_import(cx));
                                },
                            ))
                            .child(
                                Button::new("confirm-import")
                                    .label("Confirm & Import")
                                    // 可以在 Model 里加个 is_importing 状态来 disable 按钮
                                    .on_click(|_, _, cx| {
                                        let global = cx.global::<GlobalAppState>().0.clone();
                                        global.update(cx, |m, cx| m.confirm_import(cx));
                                    }),
                            ),
                    ),
            )
    }
}

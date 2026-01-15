use crate::ui::hb_icons::HappyBirdIcons;
use crate::ui::models::GlobalAppState;

use gpui::{
    App, AppContext, Context, Entity, ParentElement, Render, Styled, Subscription, Window, div, px,
};
use gpui_component::{
    ActiveTheme, Icon,
    input::{Input, InputEvent, InputState},
};

pub struct SearchPanel {
    search_input: Entity<InputState>,
    _search_subscription: Subscription,
    is_focused: bool, // [新增] 用于记录焦点状态
}

impl SearchPanel {
    pub fn new(window: &mut Window, cx: &mut App) -> Entity<Self> {
        let search_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Search (e.g. name:Bob age:20-30)...")
        });

        cx.new(|cx| {
            let _search_subscription = cx.subscribe_in(
                &search_input,
                window,
                |this: &mut Self, state, event, _window, cx| match event {
                    InputEvent::Change => {
                        let _query = state.read(cx).value();
                    }
                    InputEvent::PressEnter { .. } => {
                        let query = state.read(cx).value();

                        this.perform_search(query.to_string(), cx);
                    }
                    InputEvent::Focus => {
                        this.is_focused = true;
                        cx.notify(); // 通知视图重绘以更新边框颜色
                    }
                    InputEvent::Blur => {
                        this.is_focused = false;
                        cx.notify(); // 通知视图重绘恢复边框颜色
                    }
                },
            );
            Self {
                search_input,
                _search_subscription,
                is_focused: false,
            }
        })
    }

    /// 执行实际的搜索逻辑
    fn perform_search(&mut self, query: String, cx: &mut Context<Self>) {
        // 获取全局状态
        let global_model = cx.global::<GlobalAppState>().0.clone();

        global_model.update(cx, |model, cx| {
            // 更新查询条件
            model.search_query = query;
            // 触发重新加载 (is_reload = true)，这会重置页码并清空列表
            model.fetch_page(cx, true);
        });
    }
}

impl Render for SearchPanel {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        let border_color = if self.is_focused {
            cx.theme().secondary_active // 或者是你主题中的 success 颜色，例如 cx.theme().success
        } else {
            cx.theme().border // 默认边框颜色，或者是 gpui::transparent_black()
        };

        div()
            .bg(cx.theme().secondary)
            .border(px(1.2))
            .border_color(border_color)
            .m_2()
            .child(
                Input::new(&self.search_input)
                    .appearance(false)
                    .prefix(Icon::new(HappyBirdIcons::Search.load(cx))),
            )
    }
}

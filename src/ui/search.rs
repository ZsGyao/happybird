use crate::ui::hb_icons::HappyBirdIcons;
use crate::ui::models::GlobalAppState;
use crate::zlog::log_impl::debug;
use gpui::{App, AppContext, Context, Entity, Render, Subscription, Window};
use gpui_component::{
    Icon,
    input::{Input, InputEvent, InputState},
};

pub struct SearchPanel {
    search_input: Entity<InputState>,
    _search_subscription: Subscription,
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
                        let query = state.read(cx).value();

                        debug!("Input changed: {}", query);
                    }
                    InputEvent::PressEnter { .. } => {
                        debug!("InputEvent::PressEnter");
                        let query = state.read(cx).value();

                        this.perform_search(query.to_string(), cx);
                    }
                    InputEvent::Focus => debug!("Input focused"),
                    InputEvent::Blur => debug!("Input blurred"),
                },
            );
            Self {
                search_input,
                _search_subscription,
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
        _cx: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        Input::new(&self.search_input).prefix(Icon::new(HappyBirdIcons::Search))
    }
}

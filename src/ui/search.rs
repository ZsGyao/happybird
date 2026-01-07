use crate::ui::{hb_icons::HappyBirdIcons, info_panel::InfoPanel};
use crate::zlog::log_impl::debug;
use gpui::{AppContext, Context, Entity, Render, Subscription, Window};
use gpui_component::{
    Icon,
    input::{Input, InputEvent, InputState},
};

pub struct SearchPanel {
    search_input: Entity<InputState>,
    _search_subscription: Subscription,
}

impl SearchPanel {
    pub fn new(window: &mut Window, cx: &mut Context<InfoPanel>) -> Entity<Self> {
        let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("Search..."));
        let _search_subscription = cx.subscribe_in(
            &search_input,
            window,
            |_view, state, event, _window, cx| match event {
                InputEvent::Change => {
                    let text = state.read(cx).value();
                    debug!("Input changed: {}", text);
                }
                InputEvent::PressEnter { secondary } => {
                    if !*secondary {
                        // if user only press enter
                        debug!("Enter pressed, secondary {}", secondary);
                        let text = state.read(cx).value();
                        debug!("Input: {} ", text);
                    } else {
                        // if user press enter + else (like shift)
                        debug!("Enter pressed, secondary {}", secondary);
                    }
                }
                InputEvent::Focus => debug!("Input focused"),
                InputEvent::Blur => debug!("Input blurred"),
            },
        );

        cx.new(|_cx| Self {
            search_input,
            _search_subscription,
        })
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

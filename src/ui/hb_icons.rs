use gpui_component::IconNamed;

pub enum HappyBirdIcons {
    Search,
}

impl IconNamed for HappyBirdIcons {
    fn path(self) -> gpui::SharedString {
        match self {
            HappyBirdIcons::Search => "icons/search.svg".into(),
        }
    }
}

use gpui::{App, AppContext, Entity, Global};

pub struct Models {
    /// Whether open about
    pub show_about: Entity<bool>,
    /// Whether open folder
    pub show_folder: Entity<bool>,
}

impl Global for Models {}

pub fn build_models(cx: &mut App) {
    let show_about: Entity<bool> = cx.new(|_| false);
    let show_folder: Entity<bool> = cx.new(|_| false);
    cx.set_global(Models {
        show_about,
        show_folder,
    });
}

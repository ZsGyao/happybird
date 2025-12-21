use std::iter::Map;

use chrono::{DateTime, Local};
use gpui::{App, Context, Entity, Window};
use gpui_component::{
    IndexPath,
    list::{ListDelegate, ListItem, ListState},
};

#[derive(Clone)]
pub struct InfoBrowserDelegate {
    /// The folder info and child usr info
    pub folder: Map<FolderInfo, Vec<UsrInfo>>,
    /// The search click and the filter index
    pub filter_indices: Map<String, Vec<usize>>,
}

impl Default for InfoBrowserDelegate {
    fn default() -> Self {
        Self {
            folder: Default::default(),
            filter_indices: Default::default(),
        }
    }
}

#[derive(Clone)]
enum UsrIsLiked {
    Liked,
    Normal,
    Unliked,
}

#[derive(Clone)]
pub struct UsrInfo {
    /// The insert user name
    pub usr_name: String,
    /// The user is liked
    pub usr_liked: UsrIsLiked,
    /// The user update time
    pub usr_update_time: DateTime<Local>,
}

#[derive(Clone)]
pub struct FolderInfo {
    /// The folder name
    folder_name: String,
    /// The folder last update time
    folder_update_time: DateTime<Local>,
    /// The folder create time
    folder_create_time: DateTime<Local>,
}

impl InfoBrowserDelegate {
    pub fn new(cx: &mut App, window: &mut Window) -> Entity<ListState<InfoBrowserDelegate>> {
        /*------------------------------------------------ */
        // let delegate =
        // cx.new(|cx| ListState::new(delegate, window, cx).searchable(true))
    }
}

#[derive(Clone)]
pub struct FileInfo {
    pub name: String,
    pub is_directory: bool,
    pub size: Option<u64>,
}

impl ListDelegate for InfoBrowserDelegate {
    type Item = ListItem;

    fn items_count(&self, section: usize, cx: &App) -> usize {
        todo!()
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        todo!()
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        todo!()
    }
}

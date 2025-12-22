use std::process::Child;

use chrono::{DateTime, Local};
use gpui::{
    App, AppContext, Context, Entity, FontWeight, ParentElement, Styled, Window, div,
    prelude::FluentBuilder, px, rgb, InteractiveElement, StatefulInteractiveElement,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, IndexPath, Sizable, StyledExt,
    button::Button,
    h_flex,
    label::Label,
    list::{ListDelegate, ListItem, ListState},
    v_flex,
};
use tracing::debug;

#[derive(Clone)]
pub struct InfoBrowserDelegate {
    /// The folder info and child user info
    pub folders: Vec<FolderItem>,
    /// The search click and the filter index, usize1: folder index usize2: child item idex vec
    pub filter_indices: Vec<(usize, Vec<usize>)>,
    /// The folder expand state
    pub expanded_states: Vec<bool>,
    /// Selected folder index
    pub selected_index: Option<IndexPath>,
}

impl InfoBrowserDelegate {
    pub fn new(cx: &mut App, window: &mut Window) -> Entity<ListState<InfoBrowserDelegate>> {
        let folders: Vec<FolderItem> = vec![
            FolderItem {
                info: FolderInfo::new("Folder1".to_string(), Local::now(), Local::now()),
                users: vec![
                    UsrInfo::new("Jim".to_string(), UsrIsLiked::Liked, Local::now()),
                    UsrInfo::new("Aurora".to_string(), UsrIsLiked::Liked, Local::now()),
                    UsrInfo::new("Kim".to_string(), UsrIsLiked::Normal, Local::now()),
                    UsrInfo::new("Boss".to_string(), UsrIsLiked::Unliked, Local::now()),
                ],
            },
            FolderItem {
                info: FolderInfo::new("Folder2".to_string(), Local::now(), Local::now()),
                users: vec![UsrInfo::new(
                    "Aki".to_string(),
                    UsrIsLiked::Normal,
                    Local::now(),
                )],
            },
        ];

        let mut filter_indices: Vec<(usize, Vec<usize>)> = Vec::new();
        for (idx, folder) in folders.iter().enumerate() {
            filter_indices.push((idx, (0..folder.users.len()).collect::<Vec<usize>>()));
        }

        let expanded_states = vec![false; folders.len()];

        let delegate = Self {
            folders,
            filter_indices,
            expanded_states,
            selected_index: None,
        };
        cx.new(|cx| ListState::new(delegate, window, cx).searchable(true))
    }

    pub fn filter_indices_fill(&mut self) {
        for (idx, folder) in self.folders.iter().enumerate() {
            self.filter_indices
                .push((idx, (0..folder.users.len()).collect::<Vec<usize>>()));
        }
    }

    fn toggle_folder(&mut self, section: usize, cx: &mut Context<ListState<Self>>) {
        if let Some(state) = self.expanded_states.get_mut(section) {
            *state = !*state;
            cx.notify();
        }
    }
}

#[derive(Clone)]
pub struct FolderItem {
    pub info: FolderInfo,
    pub users: Vec<UsrInfo>,
}

#[derive(Clone)]
pub enum UsrIsLiked {
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

impl UsrInfo {
    pub fn new(usr_name: String, usr_liked: UsrIsLiked, usr_update_time: DateTime<Local>) -> Self {
        UsrInfo {
            usr_name,
            usr_liked,
            usr_update_time,
        }
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct FolderInfo {
    /// The folder name
    folder_name: String,
    /// The folder last update time
    folder_update_time: DateTime<Local>,
    /// The folder create time
    folder_create_time: DateTime<Local>,
}

impl FolderInfo {
    pub fn new(
        folder_name: String,
        folder_update_time: DateTime<Local>,
        folder_create_time: DateTime<Local>,
    ) -> Self {
        FolderInfo {
            folder_name,
            folder_update_time,
            folder_create_time,
        }
    }
}

impl ListDelegate for InfoBrowserDelegate {
    type Item = ListItem;

    fn sections_count(&self, cx: &App) -> usize {
        let count = self.folders.len();
        debug!("Section count: {}", count);
        count
    }

    fn items_count(&self, section: usize, cx: &App) -> usize {
        if self.expanded_states.get(section).copied().unwrap_or(false) {
            let count = self
                .folders
                .get(section)
                .map(|folder| folder.users.len())
                .unwrap_or(0);
            debug!("Items count: {}", count);

            count
        } else {
            // Ensure section headers render even when collapsed by returning a placeholder item
            debug!("Section collapsed -> return 1 placeholder item");
            1
        }
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        // Render a zero-height placeholder item for collapsed sections to keep headers visible
        let is_expanded = self
            .expanded_states
            .get(ix.section)
            .copied()
            .unwrap_or(false);
        if !is_expanded {
            return Some(ListItem::new(ix).child(div().h(px(0.0))));
        }

        let folder = self.folders.get(ix.section)?;
        let user = folder.users.get(ix.row)?;

        let icon = match user.usr_liked {
            UsrIsLiked::Liked => IconName::Heart,
            UsrIsLiked::Normal => IconName::Bot,
            UsrIsLiked::Unliked => IconName::HeartOff,
        };

        Some(
            ListItem::new(ix).child(
                div()
                    .child(
                        h_flex()
                            .items_center()
                            .justify_between()
                            .w_full()
                            .p_3()
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(Icon::new(icon))
                                    .child(Label::new(folder.users[ix.row].usr_name.clone())),
                            )
                            .child(
                                Label::new(format!(
                                    "{}",
                                    folder.users[ix.row]
                                        .usr_update_time
                                        .format("%d/%m/%Y %H:%M")
                                ))
                                .text_xs()
                                .text_color(cx.theme().muted_foreground),
                            ),
                    )
                    .hover(|this| this.bg(cx.theme().background)),
            ),
        )
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        self.selected_index = ix;
        cx.notify();
    }

    fn render_section_header(
        &mut self,
        section: usize,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<impl gpui::IntoElement> {
        let folder = match self.folders.get(section) {
            Some(f) => f,
            None => {
                debug!("  -> ERROR: No folder for section {}", section);
                return None;
            }
        };
        let view_handle = cx.entity().clone();
        let is_expanded = self
            .expanded_states
            .get(section)
            .copied()
            .unwrap_or(false);
        let chevron_icon = if is_expanded {
            IconName::ChevronDown
        } else {
            IconName::ChevronRight
        };

        Some(
            h_flex()
                .items_center()
                .justify_between()
                .w_full()
                .p_3()
                .bg(cx.theme().background)
                .border_1()
                .border_color(cx.theme().border)
                .child(
                    h_flex()
                        .items_center()
                        .gap_2()
                        .child(
                            Button::new("folder-id")
                                .icon(Icon::new(chevron_icon))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.delegate_mut().toggle_folder(section, cx);
                                })),
                        )
                        .child(
                            Label::new(folder.info.folder_name.to_string())
                                .text_color(cx.theme().accent_foreground)
                                .font_weight(FontWeight::BOLD),
                        ),
                )
                .child(
                    Label::new(format!("{}", folder.users.len()))
                        .text_xs()
                        .text_color(cx.theme().muted_foreground),
                ),
        )
    }
}

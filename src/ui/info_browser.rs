use crate::ui::constants::APP_ROUNDING;
use chrono::{DateTime, Local};
use gpui::{
    App, AppContext, Context, Entity, FontWeight, InteractiveElement, ParentElement, Render,
    Styled, Window, div, prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, IndexPath,
    accordion::Accordion,
    h_flex,
    label::Label,
    list::{List, ListDelegate, ListItem, ListState},
};
use tracing::info;

// The main component that will render the list of folders
pub struct InfoBrowser {
    folders: Vec<FolderEntry>,
}

impl InfoBrowser {
    pub fn new(cx: &mut App, window: &mut Window) -> Entity<Self> {
        // Raw data for folders and users
        let folder_data = vec![
            (
                FolderInfo::new("Folder1".to_string(), Local::now(), Local::now()),
                vec![
                    UsrInfo::new("Jim".to_string(), UsrIsLiked::Liked, Local::now()),
                    UsrInfo::new("Aurora".to_string(), UsrIsLiked::Liked, Local::now()),
                    UsrInfo::new("Kim".to_string(), UsrIsLiked::Normal, Local::now()),
                    UsrInfo::new("Boss".to_string(), UsrIsLiked::Unliked, Local::now()),
                ],
            ),
            (
                FolderInfo::new("Folder2".to_string(), Local::now(), Local::now()),
                vec![
                    UsrInfo::new("Aki".to_string(), UsrIsLiked::Normal, Local::now()),
                    UsrInfo::new("Alice".to_string(), UsrIsLiked::Normal, Local::now()),
                ],
            ),
            (
                FolderInfo::new("Folder3".to_string(), Local::now(), Local::now()),
                vec![],
            ),
        ];

        // Create `FolderEntry`s from the raw data
        let folders = folder_data
            .into_iter()
            .map(|(info, users)| {
                // For each group of users, create a delegate and a ListState entity
                let user_delegate = UserListDelegate::new(users);
                let user_list_state = cx.new(|cx| ListState::new(user_delegate, window, cx));
                FolderEntry {
                    info,
                    user_list_state,
                    is_expanded: true,
                }
            })
            .collect();

        cx.new(|_| Self { folders })
    }
}

impl Render for InfoBrowser {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let mut accordion = Accordion::new("info-browser-accordion")
            .multiple(true)
            .bordered(true);

        for (index, folder) in &mut self.folders.iter().enumerate() {
            let user_count = folder.user_list_state.read(cx).delegate().users.len();

            accordion = accordion
                .on_toggle_click(|_, _, _| {
                    info!("Folder toggled");
                })
                .item(|item| {
                    item.title(
                        h_flex()
                            .w_full()
                            .items_center()
                            .p_2()
                            //.gap_3()
                            .rounded(APP_ROUNDING)
                            .hover(|s| s.bg(cx.theme().muted))
                            .child(Icon::new(if folder.is_expanded {
                                IconName::FolderOpen
                            } else {
                                IconName::Folder
                            }))
                            .child(
                                Label::new(folder.info.folder_name.clone())
                                    .font_weight(FontWeight::SEMIBOLD),
                            )
                            .child(div().flex_grow())
                            .child(div().px_2().rounded_full().bg(cx.theme().border).when(
                                user_count > 0,
                                |this| {
                                    this.child(
                                        Label::new(format!("{}", user_count))
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground),
                                    )
                                },
                            )),
                    )
                    .when(folder.is_expanded, |this| {
                        this.child(List::new(&folder.user_list_state))
                    })
                });
        }

        div().flex().flex_col().child(accordion)
    }
}

// A struct to hold all the data and state for a single folder
struct FolderEntry {
    info: FolderInfo,
    is_expanded: bool,
    user_list_state: Entity<ListState<UserListDelegate>>,
}

// Data model for a folder
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct FolderInfo {
    folder_name: String,
    folder_update_time: DateTime<Local>,
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

#[derive(Clone)]
pub enum UsrIsLiked {
    Liked,
    Normal,
    Unliked,
}

// Data model for a user
#[derive(Clone)]
pub struct UsrInfo {
    pub usr_name: String,
    pub usr_liked: UsrIsLiked,
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

// Delegate for the list of users inside an accordion
#[derive(Clone)]
pub struct UserListDelegate {
    pub users: Vec<UsrInfo>,
}

impl UserListDelegate {
    pub fn new(users: Vec<UsrInfo>) -> Self {
        Self { users }
    }
}

impl ListDelegate for UserListDelegate {
    type Item = ListItem;

    fn items_count(&self, _section: usize, _cx: &App) -> usize {
        self.users.len()
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        _window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        let user = &self.users[ix.row];
        let theme = cx.theme();
        let icon = match user.usr_liked {
            UsrIsLiked::Liked => IconName::Heart,
            UsrIsLiked::Normal => IconName::Bot,
            UsrIsLiked::Unliked => IconName::HeartOff,
        };
        Some(
            ListItem::new(ix).child(
                div()
                    .rounded(APP_ROUNDING)
                    .border_1()
                    .border_color(theme.border)
                    .m_1()
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
                                    .child(Icon::new(icon).size(px(18.0)))
                                    .child(Label::new(user.usr_name.clone())),
                            )
                            .child(
                                Label::new(format!(
                                    "{}",
                                    user.usr_update_time.format("%d/%m/%Y %H:%M")
                                ))
                                .text_xs()
                                .text_color(theme.muted_foreground),
                            ),
                    )
                    .hover(|this| this.bg(theme.background)),
            ),
        )
    }

    fn set_selected_index(
        &mut self,
        _ix: Option<IndexPath>,
        _window: &mut Window,
        _cx: &mut Context<ListState<Self>>,
    ) {
        // You could store the selected index in `UserListDelegate` if needed
    }
}

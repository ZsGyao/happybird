use gpui::{
    App, AppContext, Context, Entity, ParentElement, Styled, Window, prelude::FluentBuilder,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, IndexPath, h_flex,
    label::Label,
    list::{ListDelegate, ListItem, ListState},
};
use tracing::info;

pub struct FileBrowserDelegate {
    pub files: Vec<FileInfo>,
    pub selected: Option<IndexPath>,
}

impl FileBrowserDelegate {
    pub fn new(cx: &mut App, window: &mut Window) -> Entity<ListState<FileBrowserDelegate>> {
        /*------------------------------------------------ */
        let delegate = FileBrowserDelegate {
            files: vec![
                FileInfo {
                    name: "File1".to_string(),
                    is_directory: false,
                    size: Some(20),
                },
                FileInfo {
                    name: "File2".to_string(),
                    is_directory: true,
                    size: Some(20),
                },
                FileInfo {
                    name: "File3".to_string(),
                    is_directory: false,
                    size: Some(20),
                },
                FileInfo {
                    name: "File4".to_string(),
                    is_directory: false,
                    size: Some(20),
                },
            ],
            selected: None,
        };
        cx.new(|cx| ListState::new(delegate, window, cx))
    }
}

#[derive(Clone)]
pub struct FileInfo {
    pub name: String,
    pub is_directory: bool,
    pub size: Option<u64>,
}

impl ListDelegate for FileBrowserDelegate {
    type Item = ListItem;

    fn render_item(
        &mut self,
        ix: IndexPath,
        window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        self.files.get(ix.row).map(|file| {
            let icon = if file.is_directory {
                IconName::Folder
            } else {
                IconName::File
            };

            info!("FileBrowserDelegate Render");

            ListItem::new(ix)
                .child(
                    h_flex()
                        .items_center()
                        .justify_between()
                        .w_full()
                        .child(
                            h_flex()
                                .items_center()
                                .gap_2()
                                .child(Icon::new(icon))
                                .child(Label::new(file.name.clone())),
                        )
                        .when_some(file.size, |this, size| {
                            this.child(
                                Label::new(size.to_string())
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground),
                            )
                        }),
                )
                .selected(Some(ix) == self.selected)
        })
    }

    fn items_count(&self, section: usize, cx: &App) -> usize {
        self.files.len()
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) {
        self.selected = ix;
        cx.notify();
    }
}

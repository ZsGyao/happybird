use gpui::{
    App, AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled, Window, div, px,
};
use gpui_component::{
    StyledExt,
    input::{Input, InputState},
    table::{Column, Table, TableDelegate, TableState},
};

pub struct HappyBirdComponentTest {
    test_table: Entity<TableState<MyTableDelegate>>,
    input: Entity<InputState>,
}

impl HappyBirdComponentTest {
    pub fn new(cx: &mut App, window: &mut Window) -> Entity<Self> {
        // Create the table
        let delegate = MyTableDelegate::new();
        let state = cx.new(|cx| TableState::new(delegate, window, cx));

        let input = cx.new(|cx| InputState::new(window, cx).placeholder("test..."));

        cx.new(|_cx| Self {
            test_table: state,
            input,
        })
    }
}

impl Render for HappyBirdComponentTest {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // 1. 外层容器：充当背景板，占满全屏并负责居中子元素
        div()
            .absolute()
            .size_full() // 占满窗口宽高
            .items_center() // 垂直方向居中
            .justify_center()
            .relative() // 水平方向居中
            .child(
                // 2. 内层容器：这是你的具体内容区域，设置固定大小
                div()
                    .v_flex()
                    .w(px(950.0))
                    .h(px(500.0))
                    // 可选：添加边框或阴影以便看清边界
                    // .border_1()
                    // .border_color(gpui::gray())
                    .child(
                        Table::new(&self.test_table)
                            .stripe(true)
                            .scrollbar_visible(true, true),
                    )
                    .child(Input::new(&self.input)),
            )
    }
}

struct MyData {
    id: usize,
    name: String,
    age: u32,
    email_1: String,
    email_2: String,
    email_3: String,
    email_4: String,
    email_5: String,
    email_6: String,
    email_7: String,
    email_8: String,
    email_9: String,
}

struct MyTableDelegate {
    data: Vec<MyData>,
    columns: Vec<Column>,
}

impl MyTableDelegate {
    fn new() -> Self {
        Self {
            data: vec![
                MyData {
                    id: 1,
                    name: "John".to_string(),
                    age: 30,
                    email_1: "john-1@example.com".to_string(),
                    email_2: "john-2@example.com".to_string(),
                    email_3: "john-3@example.com".to_string(),
                    email_4: "john-4@example.com".to_string(),
                    email_5: "john-5@example.com".to_string(),
                    email_6: "john-6@example.com".to_string(),
                    email_7: "john-7@example.com".to_string(),
                    email_8: "john-8@example.com".to_string(),
                    email_9: "john-9@example.com".to_string(),
                },
                MyData {
                    id: 2,
                    name: "Jane".to_string(),
                    age: 25,
                    email_1: "Jane-1@example.com".to_string(),
                    email_2: "Jane-2@example.com".to_string(),
                    email_3: "Jane-3@example.com".to_string(),
                    email_4: "Jane-4@example.com".to_string(),
                    email_5: "Jane-5@example.com".to_string(),
                    email_6: "Jane-6@example.com".to_string(),
                    email_7: "Jane-7@example.com".to_string(),
                    email_8: "Jane-8@example.com".to_string(),
                    email_9: "Jane-9@example.com".to_string(),
                },
            ],
            columns: vec![
                Column::new("id", "ID").width(60.),
                Column::new("name", "Name").width(150.).sortable(),
                Column::new("age", "Age").width(80.).sortable(),
                Column::new("email-1", "Email-1").width(200.),
                Column::new("email-2", "Email-2").width(200.),
                Column::new("email-3", "Email-3").width(200.),
                Column::new("email-4", "Email-4").width(200.),
                Column::new("email-5", "Email-5").width(200.),
                Column::new("email-6", "Email-6").width(200.),
                Column::new("email-7", "Email-7").width(200.),
                Column::new("email-8", "Email-8").width(200.),
                Column::new("email-9", "Email-9").width(200.),
            ],
        }
    }
}

impl TableDelegate for MyTableDelegate {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        self.data.len()
    }

    fn column(&self, col_ix: usize, _: &App) -> Column {
        self.columns[col_ix].clone()
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let row = &self.data[row_ix];
        let col = &self.columns[col_ix];

        match col.key.as_ref() {
            "id" => row.id.to_string(),
            "name" => row.name.clone(),
            "age" => row.age.to_string(),
            "email-1" => row.email_1.clone(),
            "email-2" => row.email_2.clone(),
            "email-3" => row.email_3.clone(),
            "email-4" => row.email_4.clone(),
            "email-5" => row.email_5.clone(),
            "email-6" => row.email_6.clone(),
            "email-7" => row.email_7.clone(),
            "email-8" => row.email_8.clone(),
            "email-9" => row.email_9.clone(),
            _ => "".to_string(),
        }
    }
}

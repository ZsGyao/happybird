1. 搜索可以按名字的拼音或拼音首字母模糊搜索
2. 可按部门分类，当部门用户中的信息被修改了，可以把这个用户放在新的部门文件夹下（用户目录可以让用户选择组织的方式），比如用户中有一个信息是部门，我现在显示的是所用用户在一个文件夹下，那当用户先择用部门这个字段组织的时候，那就应该有不同部门的文件夹，文件夹下就是属于那个部门的用户的item条目
3. 在history panel的地方给一个用户可以输入的备注

三个文档的链接 gpui component 0.5.0： https://docs.rs/gpui-component/latest/gpui_component/ gpui 0.2.2 https://docs.rs/gpui/0.2.2/gpui/  gpui component组件文档 https://longbridge.github.io/gpui-component/docs/components/input

src/ui/
├── mod.rs                  (入口：导出子模块)
├── app.rs                  (主程序：负责组装，它是 Strategy 的消费者)
├── assets.rs               (资源加载：字体、图片)
├── constants.rs            (全局常量)
│
├── models/                 (M层：纯数据状态，无 UI 代码)
│   ├── mod.rs              (导出：build_models 初始化函数)
│   ├── global.rs           (原有 GlobalAppState：兼容旧业务)
│   ├── security.rs         (拆分：锁定、密码逻辑)
│   ├── theme.rs            (拆分：ThemeModel 管理器状态)
│   └── navigation.rs       (拆分：路由、侧边栏状态)
│
├── components/             (V层：UI 组件库 - 按功能分类，而非按主题分类)
│   ├── mod.rs
│   │
│   ├── shared/             (原子组件：所有主题通用的积木)
│   │   ├── mod.rs
│   │   ├── icon.rs
│   │   └── nav_item.rs     (可配置的导航按钮)
│   │
│   ├── layout/             (布局组件：Header, Sidebar)
│   │   ├── mod.rs
│   │   │
│   │   ├── header/         (Header 文件夹：聚合所有 Header 变体)
│   │   │   ├── mod.rs      (导出 Default 和 Modern)
│   │   │   ├── default.rs  (原有 Header 代码)
│   │   │   └── modern.rs   (新写的胶囊式 Tab Header)
│   │   │
│   │   ├── sidebar/        (Sidebar 文件夹：聚合所有 Sidebar 变体)
│   │   │   ├── mod.rs
│   │   │   ├── default.rs  (原有 SideBar 代码)
│   │   │   └── parts.rs    (拆分出的 Logo、Menu、User 小部件)
│   │   │
│   │   └── status_bar.rs   (通用 Status Bar，暂无变体)
│   │
│   └── features/           (业务功能面板)
│       ├── mod.rs
│       ├── info_panel.rs
│       ├── detail_panel.rs
│       ├── import_panel.rs
│       ├── lock_screen.rs
│       └── set_password_modal.rs
│
└── theme/                  (策略层：大脑)
    ├── mod.rs
    │
    ├── strategy.rs         (核心：定义 ThemeStrategy Trait)
    ├── manager.rs          (核心：ThemeModel 实现)
    │
    ├── impls/              (具体策略实现：决定使用哪个组件)
    │   ├── mod.rs
    │   ├── default.rs      (DefaultTheme 结构体：组装 default header/sidebar)
    │   └── modern.rs       (ModernTheme 结构体：组装 modern header)
    │
    └── infra/              (基础设施：你担心的那些文件放这里)
        ├── mod.rs
        ├── loader.rs       (加载 JSON 颜色配置)
        ├── style.rs        (颜色样式辅助)
        └── extra.rs        (其他辅助)

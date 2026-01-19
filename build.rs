// build.rs
fn main() {
    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        // 设置图标路径
        res.set_icon("happybird_logo.ico");
        // 这里还可以设置文件属性，如版本号等
        // res.set("FileVersion", "0.1.0");
        res.compile().unwrap();
    }
}

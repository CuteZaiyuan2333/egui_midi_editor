use eframe::egui;
use egui_file_tree::{FileTree, FileTreeEvent};
use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    env_logger::init();

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "File Tree Example",
        native_options,
        Box::new(|_cc| Ok(Box::new(FileTreeApp::new()))),
    )
}

struct FileTreeApp {
    file_tree: Option<FileTree>,
    current_root: Option<PathBuf>,
    status_message: String,
}

impl FileTreeApp {
    fn new() -> Self {
        Self {
            file_tree: None,
            current_root: None,
            status_message: "Please select a directory to open from the File menu".to_string(),
        }
    }

    fn open_directory(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Select Directory")
            .pick_folder()
        {
            let path_clone = path.clone();
            let file_tree = FileTree::new(path.clone());
            self.file_tree = Some(file_tree);
            self.current_root = Some(path);
            self.status_message = format!("Opened directory: {:?}", self.current_root.as_ref().unwrap());
            log::info!("Opened directory: {:?}", path_clone);
        }
    }
}

impl eframe::App for FileTreeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 顶部菜单栏
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open Directory").clicked() {
                        self.open_directory();
                        ui.close_menu();
                    }
                });
            });
        });

        // 底部状态栏
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status_message);
            });
        });

        // 中央面板显示文件树
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(ref mut file_tree) = self.file_tree {
                let events = file_tree.ui(ui);
                
                // 处理事件
                for event in events {
                    match event {
                        FileTreeEvent::PathSelected { path } => {
                            self.status_message = format!("Selected: {:?}", path);
                            log::info!("Path selected: {:?}", path);
                        }
                        FileTreeEvent::PathDoubleClicked { path } => {
                            self.status_message = format!("Double clicked: {:?} (File opening is handled by the application)", path);
                            log::info!("Path double clicked: {:?}", path);
                        }
                        FileTreeEvent::PathRightClicked { path, pos: _ } => {
                            self.status_message = format!("Right clicked: {:?}", path);
                            log::info!("Path right clicked: {:?}", path);
                        }
                        FileTreeEvent::PathDragStarted { path } => {
                            self.status_message = format!("Drag started: {:?}", path);
                            log::info!("Path drag started: {:?}", path);
                        }
                        FileTreeEvent::NavigateToParent => {
                            if let Some(ref mut file_tree) = self.file_tree {
                                if let Some(parent) = file_tree.root_path().parent() {
                                    let parent_path = parent.to_path_buf();
                                    file_tree.set_root_path(parent_path.clone());
                                    self.current_root = Some(parent_path.clone());
                                    self.status_message = format!("Navigated to parent directory: {:?}", parent_path);
                                    log::info!("Navigated to parent directory: {:?}", parent_path);
                                }
                            }
                        }
                    }
                }
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Please select a directory to open from the File menu");
                });
            }
        });
    }
}


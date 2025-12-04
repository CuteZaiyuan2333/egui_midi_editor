//! 文件树组件实现

use egui::*;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// 文件树事件
#[derive(Debug, Clone)]
pub enum FileTreeEvent {
    /// 路径被选中
    PathSelected { path: PathBuf },
    /// 路径被双击
    PathDoubleClicked { path: PathBuf },
    /// 导航到父目录
    NavigateToParent,
}

/// 文件树组件
pub struct FileTree {
    /// 根目录路径
    root_path: PathBuf,
    /// 已展开的路径集合
    expanded: BTreeSet<PathBuf>,
    /// 当前选中的路径
    selected: Option<PathBuf>,
}

impl FileTree {
    /// 创建新的文件树组件
    pub fn new(root_path: PathBuf) -> Self {
        let mut tree = Self {
            root_path,
            expanded: BTreeSet::new(),
            selected: None,
        };
        // 默认展开根目录
        tree.expanded.insert(tree.root_path.clone());
        tree
    }

    /// 设置根目录路径
    pub fn set_root_path(&mut self, path: PathBuf) {
        self.root_path = path;
        self.expanded.clear();
        self.expanded.insert(self.root_path.clone());
        self.selected = None;
    }

    /// 展开指定路径
    pub fn expand_path(&mut self, path: &PathBuf) {
        self.expanded.insert(path.clone());
    }

    /// 折叠指定路径
    pub fn collapse_path(&mut self, path: &PathBuf) {
        self.expanded.remove(path);
    }

    /// 渲染UI并返回事件列表
    pub fn ui(&mut self, ui: &mut Ui) -> Vec<FileTreeEvent> {
        let mut events = Vec::new();
        let root_path = self.root_path.clone();
        
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                self.render_directory(&root_path, ui, 0, &mut events);
            });
        
        events
    }

    /// 获取当前根目录路径
    pub fn root_path(&self) -> &PathBuf {
        &self.root_path
    }

    /// 渲染目录节点（递归）
    fn render_directory(
        &mut self,
        dir_path: &Path,
        ui: &mut Ui,
        indent_level: usize,
        events: &mut Vec<FileTreeEvent>,
    ) {
        // 如果是根目录且indent_level为0，显示"../"选项
        if indent_level == 0 && dir_path == self.root_path.as_path() {
            if dir_path.parent().is_some() {
                ui.horizontal(|ui| {
                    ui.add_space(indent_level as f32 * 20.0);
                    ui.add_space(16.0); // 占位，对齐展开按钮
                    
                    let label_text = "📁 ../";
                    let response = ui.selectable_label(false, label_text)
                        .on_hover_cursor(CursorIcon::PointingHand);
                    
                    if response.clicked() {
                        events.push(FileTreeEvent::NavigateToParent);
                    }
                });
            }
        }

        // 读取目录内容
        let entries = match std::fs::read_dir(dir_path) {
            Ok(entries) => {
                let mut entries: Vec<_> = entries
                    .filter_map(|e| e.ok())
                    .collect();
                // 排序：文件夹在前，然后按名称排序
                entries.sort_by(|a, b| {
                    let a_is_dir = a.path().is_dir();
                    let b_is_dir = b.path().is_dir();
                    match (a_is_dir, b_is_dir) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => a.file_name().cmp(&b.file_name()),
                    }
                });
                entries
            }
            Err(_) => {
                // 无法读取目录，显示错误信息
                ui.horizontal(|ui| {
                    ui.add_space(indent_level as f32 * 20.0);
                    ui.label(RichText::new("⚠ Cannot access").color(Color32::RED));
                });
                return;
            }
        };

        // 渲染每个条目
        for entry in entries {
            let path = entry.path();
            let is_dir = path.is_dir();
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();

            // 跳过隐藏文件（以.开头的文件，在Unix系统上）
            #[cfg(unix)]
            if file_name_str.starts_with('.') {
                continue;
            }

            let path_buf = path.clone();
            let is_expanded = self.expanded.contains(&path_buf);
            let is_selected = self.selected.as_ref().map_or(false, |s| s == &path_buf);

            ui.horizontal(|ui| {
                // 缩进
                ui.add_space(indent_level as f32 * 20.0);

                // 展开/折叠按钮（仅文件夹）
                if is_dir {
                    let expand_icon = if is_expanded { "▼" } else { "▶" };
                    let expand_button = ui.selectable_label(false, expand_icon)
                        .on_hover_cursor(CursorIcon::PointingHand);
                    
                    if expand_button.clicked() {
                        if is_expanded {
                            self.collapse_path(&path_buf);
                        } else {
                            self.expand_path(&path_buf);
                        }
                    }
                } else {
                    // 文件不需要展开按钮，但需要占位
                    ui.add_space(16.0);
                }

                // 图标和文件名
                let icon = if is_dir { "📁" } else { "📄" };
                let label_text = format!("{} {}", icon, file_name_str);
                
                let response = ui.selectable_label(is_selected, label_text)
                    .on_hover_cursor(CursorIcon::PointingHand);

                // 处理点击事件
                if response.clicked() {
                    self.selected = Some(path_buf.clone());
                    events.push(FileTreeEvent::PathSelected {
                        path: path_buf.clone(),
                    });
                }

                // 处理双击事件
                if response.double_clicked() {
                    events.push(FileTreeEvent::PathDoubleClicked {
                        path: path_buf.clone(),
                    });
                }
            });

            // 如果文件夹已展开，递归渲染子目录
            if is_dir && is_expanded {
                self.render_directory(&path, ui, indent_level + 1, events);
            }
        }
    }
}


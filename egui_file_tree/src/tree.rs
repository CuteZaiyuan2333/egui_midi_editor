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
    /// 路径被右键点击
    PathRightClicked { path: PathBuf, pos: Pos2 },
    /// 路径开始被拖拽
    PathDragStarted { path: PathBuf },
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
    /// 正在拖拽的路径
    dragging_path: Option<PathBuf>,
    /// 拖拽开始时的文件路径和鼠标位置
    drag_start: Option<(PathBuf, Pos2)>,
}

impl FileTree {
    /// 创建新的文件树组件
    pub fn new(root_path: PathBuf) -> Self {
        let mut tree = Self {
            root_path,
            expanded: BTreeSet::new(),
            selected: None,
            dragging_path: None,
            drag_start: None,
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
        self.dragging_path = None;
        self.drag_start = None;
    }
    
    /// 获取正在拖拽的路径
    pub fn dragging_path(&self) -> Option<&PathBuf> {
        self.dragging_path.as_ref()
    }
    
    /// 清除拖拽状态
    pub fn clear_drag(&mut self) {
        self.dragging_path = None;
        self.drag_start = None;
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
        
        // 检查是否在拖拽过程中鼠标释放（全局检查）
        if self.dragging_path.is_some() && !ui.input(|i| i.pointer.primary_down()) {
            self.clear_drag();
        }
        
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

                // 处理拖拽检测（仅对文件，且是 .midiclip 文件）
                if !is_dir {
                    if let Some(ext) = path_buf.extension() {
                        if ext == "midiclip" {
                            // 检测鼠标按下
                            if response.is_pointer_button_down_on() {
                                if self.dragging_path.is_none() && self.drag_start.is_none() {
                                    // 记录拖拽开始位置和文件路径
                                    if let Some(pointer) = response.interact_pointer_pos() {
                                        self.drag_start = Some((path_buf.clone(), pointer));
                                    }
                                }
                            }
                            
                            // 检测拖拽开始（鼠标按下并移动一定距离）
                            // 只有当拖拽开始的文件与当前文件匹配时，才检测拖拽
                            // 这确保只有真正从当前文件开始的拖拽才会被识别
                            if let Some((drag_file_path, start_pos)) = &self.drag_start {
                                // 只有当拖拽开始的文件与当前文件匹配时，才检测拖拽
                                if drag_file_path == &path_buf {
                                    if ui.input(|i| i.pointer.primary_down()) {
                                        if let Some(current_pos) = ui.input(|i| i.pointer.hover_pos()) {
                                            let drag_distance = (current_pos - *start_pos).length();
                                            const DRAG_THRESHOLD: f32 = 5.0; // 5像素阈值
                                            
                                            // 只有当没有正在拖拽的文件，且拖拽距离超过阈值时，才触发拖拽开始事件
                                            if drag_distance > DRAG_THRESHOLD && self.dragging_path.is_none() {
                                                // 触发拖拽开始事件
                                                self.dragging_path = Some(path_buf.clone());
                                                events.push(FileTreeEvent::PathDragStarted {
                                                    path: path_buf.clone(),
                                                });
                                            }
                                        }
                                    } else {
                                        // 鼠标释放，清除拖拽状态
                                        if self.dragging_path.as_ref() == Some(&path_buf) {
                                            self.clear_drag();
                                        }
                                        // 清除拖拽开始位置（只有当是当前文件时才清除）
                                        self.drag_start = None;
                                    }
                                }
                            }
                        }
                    }
                }

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

                // 处理右键点击事件
                if response.secondary_clicked() {
                    if let Some(pointer) = response.interact_pointer_pos() {
                        events.push(FileTreeEvent::PathRightClicked {
                            path: path_buf.clone(),
                            pos: pointer,
                        });
                    }
                }
            });

            // 如果文件夹已展开，递归渲染子目录
            if is_dir && is_expanded {
                self.render_directory(&path, ui, indent_level + 1, events);
            }
        }
    }
}


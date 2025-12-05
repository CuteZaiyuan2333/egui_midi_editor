//! UI渲染模块
//!
//! 处理所有用户界面的渲染逻辑，包括菜单栏、状态栏、主内容区域等。

use crate::{MidiTrackFileApp, TopTab};
use crate::midiclip;
use eframe::egui;
use egui_track::format_time;
use egui_file_tree::FileTreeEvent;

impl MidiTrackFileApp {
    /// 渲染新项目对话框
    pub fn render_new_project_dialog(&mut self, ctx: &egui::Context) {
        if !self.new_project_dialog_open {
            return;
        }
        
        egui::Window::new("Create New Project")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.label("Enter project name:");
                    let response = ui.text_edit_singleline(&mut self.new_project_name);
                    
                    // 如果用户按下 Enter，完成创建
                    if response.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.finish_new_project();
                    }
                    
                    ui.add_space(10.0);
                    
                    ui.horizontal(|ui| {
                        if ui.button("Create").clicked() {
                            self.finish_new_project();
                        }
                        if ui.button("Cancel").clicked() {
                            self.new_project_dialog_open = false;
                            self.new_project_parent_dir = None;
                            self.new_project_name.clear();
                        }
                    });
                });
            });
    }
    
    /// 渲染顶部菜单栏
    pub fn render_menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New").clicked() {
                        self.new_project();
                        ui.close_menu();
                    }
                    if ui.button("Open").clicked() {
                        self.open_project();
                        ui.close_menu();
                    }
                    if ui.button("Save").clicked() {
                        self.save_project();
                        ui.close_menu();
                    }
                    if ui.button("Save As").clicked() {
                        self.save_project_as();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Export").clicked() {
                        self.export_project();
                        ui.close_menu();
                    }
                });
                
                ui.menu_button("MIDI", |ui| {
                    if ui.button("New MIDI Editor").clicked() {
                        self.add_midi_editor();
                        ui.close_menu();
                    }
                });
                
                // Playback controls
                ui.separator();
                if self.is_playing {
                    if ui.button("⏸ Pause").clicked() {
                        self.playback_engine.pause();
                        self.is_playing = false;
                        use egui_track::TrackEditorCommand;
                        self.track_editor.execute_command(TrackEditorCommand::SetPlayback { is_playing: false });
                    }
                    if ui.button("⏹ Stop").clicked() {
                        self.playback_engine.stop();
                        self.is_playing = false;
                        use egui_track::TrackEditorCommand;
                        self.track_editor.execute_command(TrackEditorCommand::StopPlayback);
                    }
                } else {
                    if ui.button("▶ Play").clicked() {
                        let current_time = ctx.input(|i| i.time);
                        let start_position = self.track_editor.timeline().playhead_position;
                        self.playback_engine.start_from_position(current_time, start_position);
                        self.is_playing = true;
                        use egui_track::TrackEditorCommand;
                        self.track_editor.execute_command(TrackEditorCommand::SetPlayback { is_playing: true });
                    }
                }
            });
        });
    }

    /// 渲染底部状态栏
    pub fn render_status_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(ref path) = self.current_project_path {
                    let project_name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Unknown");
                    ui.label(format!("Project: {}", project_name));
                } else {
                    ui.label("Project: Unsaved");
                }

                ui.separator();

                ui.label(format!("Tracks: {}", self.track_editor.tracks().len()));

                ui.separator();

                let total_clips: usize = self.track_editor.tracks().iter()
                    .map(|t| t.clips.len())
                    .sum();
                ui.label(format!("Clips: {}", total_clips));

                ui.separator();

                let pos = self.track_editor.timeline().playhead_position;
                ui.label(format!("Position: {}", format_time(pos)));
            });
        });
    }

    /// 渲染主内容区域
    pub fn render_main_content(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let available_rect = ui.available_rect_before_wrap();
            let min_top_height = 150.0;
            let min_bottom_height = 150.0;
            
            // Calculate top and bottom heights based on split ratio
            let top_height = (available_rect.height() * self.vertical_split_ratio)
                .max(min_top_height)
                .min(available_rect.height() - min_bottom_height);
            let bottom_height = available_rect.height() - top_height;
            
            ui.vertical(|ui| {
                // Top section: Tabs for track editor and other tools
                let top_rect = self.render_top_tabs(ui, available_rect.width(), top_height);
                
                // Vertical splitter (between top and bottom)
                self.render_vertical_splitter(ui, &available_rect, &top_rect, min_top_height, min_bottom_height);
                
                // Bottom section: File tree (left) + MIDI editors (right)
                self.render_bottom_section(ui, &available_rect, bottom_height);
            });
        });
    }

    /// 渲染顶部标签页区域
    fn render_top_tabs(&mut self, ui: &mut egui::Ui, width: f32, height: f32) -> egui::Rect {
        ui.allocate_ui_with_layout(
            egui::Vec2::new(width, height),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                // Tab bar
                ui.horizontal(|ui| {
                    let track_selected = self.top_active_tab == TopTab::TrackEditor;
                    if ui.selectable_label(track_selected, "Track Editor").clicked() {
                        self.top_active_tab = TopTab::TrackEditor;
                    }
                    
                    let other_selected = self.top_active_tab == TopTab::OtherTools;
                    if ui.selectable_label(other_selected, "Other Tools").clicked() {
                        self.top_active_tab = TopTab::OtherTools;
                    }
                });
                
                ui.separator();
                
                // Tab content
                let content_rect = ui.available_rect_before_wrap();
                let track_editor_response = ui.allocate_ui(content_rect.size(), |ui| {
                    match self.top_active_tab {
                        TopTab::TrackEditor => {
                            self.track_editor.ui(ui);
                        }
                        TopTab::OtherTools => {
                            ui.centered_and_justified(|ui| {
                                ui.label("Other Tools Tab (Placeholder)");
                                ui.label("Additional tools and features can be added here");
                            });
                        }
                    }
                });
                
                // Handle drag and drop on track editor
                if self.top_active_tab == TopTab::TrackEditor {
                    // 在每次检查时记录当前的 dragging_file_path 状态
                    if let Some(ref dragging_path) = self.dragging_file_path {
                        log::debug!("[DROP] Checking drag state: file={:?}, exists={}", 
                                   dragging_path, dragging_path.exists());
                    }
                    
                    if let Some(dragging_path) = &self.dragging_file_path.clone() {
                        if midiclip::is_midiclip_file(&dragging_path) {
                            let response = &track_editor_response.response;
                            
                            // 检查鼠标状态
                            let mouse_released = ui.input(|i| i.pointer.primary_released());
                            let mouse_down = ui.input(|i| i.pointer.primary_down());
                            
                            // 使用全局鼠标位置手动检查是否在轨道编辑器区域内
                            let pointer_pos = ui.input(|i| i.pointer.hover_pos());
                            let rect = response.rect;
                            let is_pointer_in_rect = pointer_pos.map(|p| rect.contains(p)).unwrap_or(false);
                            
                            log::info!("[DROP] State: hovered={}, pointer_in_rect={}, mouse_released={}, mouse_down={}, pointer={:?}, rect={:?}, file={:?}", 
                                      response.hovered(), is_pointer_in_rect, mouse_released, mouse_down, pointer_pos, rect, dragging_path);
                            
                            // Check if dragging over track editor
                            if is_pointer_in_rect {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::Copy);
                                
                                // Show drop preview
                                if let Some(pointer) = response.hover_pos() {
                                    // Draw drop preview indicator
                                    let painter = ui.painter();
                                    let rect = response.rect;
                                    let timeline = self.track_editor.timeline();
                                    let key_width = 250.0;
                                    let timeline_height = 40.0;
                                    
                                    // Calculate preview position
                                    let rel_y = pointer.y - rect.min.y - timeline_height;
                                    let track_index = (rel_y / timeline.zoom_y).floor() as usize;
                                    let rel_x = pointer.x - rect.min.x - key_width;
                                    let adjusted_x = (rel_x - timeline.manual_scroll_x).max(0.0);
                                    let beats = adjusted_x / timeline.zoom_x;
                                    let seconds_per_beat = 60.0 / timeline.bpm as f64;
                                    let start_time = beats as f64 * seconds_per_beat;
                                    
                                    log::debug!("[DROP] Preview: track_index={}, start_time={}, pointer={:?}", 
                                               track_index, start_time, pointer);
                                    
                                    // Get clip duration for preview
                                    let duration = match crate::midiclip::load_midiclip_file(dragging_path) {
                                        Ok(state) => {
                                            if state.notes.is_empty() {
                                                4.0
                                            } else {
                                                let max_end_tick = state.notes.iter()
                                                    .map(|note| note.start + note.duration)
                                                    .max()
                                                    .unwrap_or(0);
                                                crate::clip_operations::ticks_to_seconds(max_end_tick, state.bpm, state.ticks_per_beat)
                                            }
                                        }
                                        Err(e) => {
                                            log::warn!("[DROP] Failed to load file for preview: {:?}", e);
                                            4.0
                                        }
                                    };
                                    
                                    // Draw preview clip rectangle
                                    let tracks = self.track_editor.tracks();
                                    if track_index < tracks.len() || tracks.is_empty() {
                                        let track_y = if track_index < tracks.len() {
                                            rect.min.y + timeline_height + (track_index as f32 * timeline.zoom_y)
                                        } else {
                                            rect.min.y + timeline_height
                                        };
                                        
                                        let clip_x = rect.min.x + key_width + timeline.manual_scroll_x + (start_time as f32 * timeline.zoom_x / seconds_per_beat as f32);
                                        let clip_width = (duration as f32 * timeline.zoom_x / seconds_per_beat as f32).max(30.0);
                                        
                                        let preview_rect = egui::Rect::from_min_size(
                                            egui::pos2(clip_x, track_y),
                                            egui::Vec2::new(clip_width, timeline.zoom_y)
                                        );
                                        
                                        let preview_color = egui::Color32::from_rgba_unmultiplied(100, 150, 255, 150);
                                        painter.rect_filled(preview_rect, 2.0, preview_color);
                                        painter.rect_stroke(preview_rect, 2.0, egui::Stroke::new(2.0, egui::Color32::from_rgb(50, 100, 200)));
                                    }
                                    
                                    // Draw drop indicator line at mouse position
                                    let line_y = pointer.y;
                                    let line_start = egui::pos2(rect.min.x, line_y);
                                    let line_end = egui::pos2(rect.max.x, line_y);
                                    painter.line_segment([line_start, line_end], egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 150, 255)));
                                }
                            }
                            
                            // Check for drop using multiple methods for reliability
                            // Method 1: response.drag_stopped() (most reliable)
                            let drag_stopped = response.drag_stopped();
                            // Method 2: primary_released while pointer is in rect
                            let drop_detected = is_pointer_in_rect && mouse_released && !mouse_down;
                            
                            log::info!("[DROP] Detection: drag_stopped={}, drop_detected={}, is_pointer_in_rect={}", 
                                      drag_stopped, drop_detected, is_pointer_in_rect);
                            
                            if drag_stopped || drop_detected {
                                log::info!("[DROP] Drop detected on track editor! Method: drag_stopped={}, drop_detected={}, file: {:?}", 
                                          drag_stopped, drop_detected, dragging_path);
                                
                                // Convert pointer position to track and time
                                // 使用全局鼠标位置，如果 response.hover_pos() 不可用
                                let pointer = pointer_pos.or_else(|| response.hover_pos());
                                if let Some(pointer) = pointer {
                                    let rect = response.rect;
                                    let timeline = self.track_editor.timeline();
                                    let key_width = 250.0; // Track header width
                                    let timeline_height = 40.0; // Timeline height
                                    
                                    // Calculate track index from y position
                                    let rel_y = pointer.y - rect.min.y - timeline_height;
                                    let track_index = (rel_y / timeline.zoom_y).floor() as usize;
                                    
                                    // Calculate time from x position
                                    let rel_x = pointer.x - rect.min.x - key_width;
                                    let adjusted_x = (rel_x - timeline.manual_scroll_x).max(0.0);
                                    let beats = adjusted_x / timeline.zoom_x;
                                    let seconds_per_beat = 60.0 / timeline.bpm as f64;
                                    let start_time = beats as f64 * seconds_per_beat;
                                    
                                    log::info!("[DROP] Calculated position: track_index={}, start_time={}, pointer={:?}", 
                                              track_index, start_time, pointer);
                                    
                                    // Get or create track
                                    let tracks_before = self.track_editor.tracks();
                                    let track_count_before = tracks_before.len();
                                    log::info!("[DROP] Tracks before: count={}", track_count_before);
                                    
                                    let track_id = if track_index < tracks_before.len() {
                                        let track_id = tracks_before[track_index].id;
                                        log::info!("[DROP] Using existing track: index={}, id={:?}", track_index, track_id);
                                        Some(track_id)
                                    } else if tracks_before.is_empty() {
                                        // Create first track
                                        log::info!("[DROP] Creating first track");
                                        use egui_track::TrackEditorCommand;
                                        self.track_editor.execute_command(TrackEditorCommand::CreateTrack {
                                            name: "Track 1".to_string(),
                                        });
                                        let tracks_after = self.track_editor.tracks();
                                        let track_id = tracks_after.first().map(|t| t.id);
                                        log::info!("[DROP] Created track: id={:?}", track_id);
                                        track_id
                                    } else {
                                        // Use last track
                                        let track_id = tracks_before.last().map(|t| t.id);
                                        log::info!("[DROP] Using last track: id={:?}", track_id);
                                        track_id
                                    };
                                    
                                    if let Some(track_id) = track_id {
                                        log::info!("[DROP] Creating clip at track {:?}, start_time={}, file_path={:?}", 
                                                  track_id, start_time, dragging_path);
                                        // Create clip at precise position
                                        let file_path_to_use = dragging_path.clone();
                                        log::info!("[DROP] Using file path: {:?}", file_path_to_use);
                                        self.create_clip_from_file_at_position(file_path_to_use, track_id, start_time.max(0.0));
                                    } else {
                                        log::warn!("[DROP] No valid track_id, falling back to playhead position");
                                        // Fallback to playhead position
                                        self.create_clip_from_file(dragging_path.clone());
                                    }
                                } else {
                                    log::warn!("[DROP] No hover position available, falling back to playhead position");
                                    // Fallback to playhead position
                                    self.create_clip_from_file(dragging_path.clone());
                                }
                                
                                log::info!("[DROP] Clearing drag state");
                                self.dragging_file_path = None;
                                
                                // Clear file tree drag state
                                if let Some(ref mut file_tree) = self.file_tree {
                                    file_tree.clear_drag();
                                }
                            } else if !is_pointer_in_rect && mouse_released && !mouse_down {
                                // Clear dragging state if mouse is released outside track editor
                                log::info!("[DROP] Drop cancelled - mouse released outside track editor (pointer_in_rect={}, pointer={:?}, rect={:?})", 
                                          is_pointer_in_rect, pointer_pos, rect);
                                self.dragging_file_path = None;
                                
                                // Clear file tree drag state
                                if let Some(ref mut file_tree) = self.file_tree {
                                    file_tree.clear_drag();
                                }
                            }
                        }
                    }
                }
            }
        ).response.rect
    }

    /// 渲染垂直分割器
    fn render_vertical_splitter(
        &mut self,
        ui: &mut egui::Ui,
        available_rect: &egui::Rect,
        top_rect: &egui::Rect,
        min_top_height: f32,
        min_bottom_height: f32,
    ) {
        let splitter_height = 4.0;
        let splitter_rect = egui::Rect::from_min_size(
            egui::pos2(top_rect.min.x, top_rect.max.y),
            egui::Vec2::new(available_rect.width(), splitter_height)
        );
        let splitter_response = ui.allocate_rect(splitter_rect, egui::Sense::click_and_drag());
        
        // Use default separator style
        ui.painter().line_segment(
            [egui::pos2(available_rect.min.x, splitter_rect.center().y),
             egui::pos2(available_rect.max.x, splitter_rect.center().y)],
            ui.style().visuals.widgets.noninteractive.bg_stroke
        );
        
        // Handle vertical splitter dragging
        if splitter_response.drag_started() {
            self.dragging_vertical_splitter = true;
        }
        if self.dragging_vertical_splitter {
            if let Some(pointer) = ui.input(|i| i.pointer.hover_pos()) {
                let new_ratio = ((pointer.y - available_rect.min.y) / available_rect.height())
                    .clamp(min_top_height / available_rect.height(), 1.0 - min_bottom_height / available_rect.height());
                self.vertical_split_ratio = new_ratio;
            }
            if ui.input(|i| i.pointer.any_released()) {
                self.dragging_vertical_splitter = false;
            }
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
        } else if splitter_response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
        }
    }

    /// 渲染底部区域（文件树 + MIDI编辑器）
    fn render_bottom_section(
        &mut self,
        ui: &mut egui::Ui,
        available_rect: &egui::Rect,
        bottom_height: f32,
    ) {
        let min_file_tree_width = 150.0;
        let min_midi_editor_width = 200.0;
        let file_tree_width = (available_rect.width() * self.horizontal_split_ratio)
            .max(min_file_tree_width)
            .min(available_rect.width() - min_midi_editor_width);
        
        let _bottom_rect = ui.allocate_ui_with_layout(
            egui::Vec2::new(available_rect.width(), bottom_height),
            egui::Layout::left_to_right(egui::Align::TOP),
            |ui| {
                // Left: File tree panel
                let file_tree_rect = self.render_file_tree_panel(ui, file_tree_width, bottom_height);
                
                // Horizontal splitter (between file tree and MIDI editors)
                self.render_horizontal_splitter(ui, available_rect, &file_tree_rect, min_file_tree_width, min_midi_editor_width);
                
                // Right: MIDI editors tabs
                self.render_midi_editors(ui);
            }
        ).response.rect;
    }

    /// 渲染文件树面板
    fn render_file_tree_panel(&mut self, ui: &mut egui::Ui, width: f32, height: f32) -> egui::Rect {
        ui.allocate_ui_with_layout(
            egui::Vec2::new(width, height),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                if let Some(ref mut file_tree) = self.file_tree {
                    let events = file_tree.ui(ui);
                    
                    // 显示拖拽预览（如果正在拖拽）
                    if let Some(dragging_path) = file_tree.dragging_path() {
                        if let Some(pointer) = ui.input(|i| i.pointer.hover_pos()) {
                            // 绘制拖拽预览
                            let file_name = dragging_path.file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("File");
                            
                            egui::Area::new(egui::Id::new("drag_preview"))
                                .order(egui::Order::Tooltip)
                                .fixed_pos(pointer + egui::Vec2::new(10.0, 10.0))
                                .show(ui.ctx(), |ui| {
                                    egui::Frame::popup(ui.style())
                                        .fill(ui.style().visuals.extreme_bg_color)
                                        .stroke(egui::Stroke::new(1.0, ui.style().visuals.widgets.noninteractive.fg_stroke.color))
                                        .show(ui, |ui| {
                                            ui.label(format!("📄 {}", file_name));
                                        });
                                });
                            
                            // 设置拖拽光标
                            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                        }
                    }
                    
                    // Handle file tree events
                    for event in events {
                        match event {
                            FileTreeEvent::PathSelected { path } => {
                                log::info!("File selected: {:?}", path);
                            }
                            FileTreeEvent::PathDoubleClicked { path } => {
                                // Check if it's a .midiclip file
                                if midiclip::is_midiclip_file(&path) {
                                    self.open_midiclip_file(&path);
                                } else if midiclip::is_midi_file(&path) {
                                    self.open_midi_file(&path);
                                }
                                log::info!("File double clicked: {:?}", path);
                            }
                            FileTreeEvent::PathRightClicked { path, pos } => {
                                // Show context menu for right-clicked path
                                let path_clone = path.clone();
                                self.file_tree_context_menu_path = Some(path);
                                self.file_tree_context_menu_pos = Some(pos);
                                log::info!("File right clicked: {:?}", path_clone);
                            }
                            FileTreeEvent::PathDragStarted { path } => {
                                // Start dragging .midiclip file
                                // 只有当没有正在拖拽的文件时，才设置新的拖拽文件
                                // 这防止在拖拽过程中，鼠标经过其他文件时覆盖已设置的拖拽文件路径
                                if self.dragging_file_path.is_none() {
                                    if midiclip::is_midiclip_file(&path) {
                                        self.dragging_file_path = Some(path.clone());
                                        log::info!("[DRAG] Started dragging file: {:?}", path);
                                        log::info!("[DRAG] Drag state set, file path stored");
                                    } else {
                                        log::warn!("[DRAG] Attempted to drag non-midiclip file: {:?}", path);
                                    }
                                } else {
                                    log::debug!("[DRAG] Ignoring PathDragStarted event - already dragging: {:?} (new: {:?})", 
                                               self.dragging_file_path, path);
                                }
                            }
                            FileTreeEvent::NavigateToParent => {
                                if let Some(ref mut file_tree) = self.file_tree {
                                    if let Some(parent) = file_tree.root_path().parent() {
                                        let parent_path = parent.to_path_buf();
                                        file_tree.set_root_path(parent_path.clone());
                                        log::info!("Navigated to parent directory: {:?}", parent_path);
                                    }
                                }
                            }
                        }
                    }
                    
                    // Show context menu if needed
                    if let Some(path) = self.file_tree_context_menu_path.clone() {
                        if let Some(menu_pos) = self.file_tree_context_menu_pos {
                            let menu_response = egui::Area::new(egui::Id::new("file_tree_context_menu"))
                                .order(egui::Order::Foreground)
                                .fixed_pos(menu_pos)
                                .show(ui.ctx(), |ui| {
                                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                                        if midiclip::is_midi_file(&path) {
                                            if ui.button("Convert to .midiclip").clicked() {
                                                match midiclip::convert_mid_to_midiclip(&path) {
                                                    Ok(midiclip_path) => {
                                                        log::info!("Converted to: {:?}", midiclip_path);
                                                        self.file_tree_context_menu_path = None;
                                                        self.file_tree_context_menu_pos = None;
                                                    }
                                                    Err(e) => {
                                                        log::error!("Failed to convert: {:?}", e);
                                                    }
                                                }
                                            }
                                        } else if midiclip::is_midiclip_file(&path) {
                                            if ui.button("Edit").clicked() {
                                                self.open_midiclip_file(&path);
                                                self.file_tree_context_menu_path = None;
                                                self.file_tree_context_menu_pos = None;
                                            }
                                            ui.separator();
                                            if ui.button("Delete").clicked() {
                                                if let Err(e) = std::fs::remove_file(&path) {
                                                    log::error!("Failed to delete file: {:?}", e);
                                                } else {
                                                    log::info!("Deleted file: {:?}", path);
                                                }
                                                self.file_tree_context_menu_path = None;
                                                self.file_tree_context_menu_pos = None;
                                            }
                                        } else if path.is_dir() {
                                            if ui.button("New MIDI Clip").clicked() {
                                                let mut new_path = path.join("clip.midiclip");
                                                let mut counter = 1;
                                                while new_path.exists() {
                                                    new_path = path.join(format!("clip_{}.midiclip", counter));
                                                    counter += 1;
                                                }
                                                match midiclip::create_midiclip_file(&new_path) {
                                                    Ok(_) => {
                                                        log::info!("Created new MIDI clip: {:?}", new_path);
                                                    }
                                                    Err(e) => {
                                                        log::error!("Failed to create MIDI clip: {:?}", e);
                                                    }
                                                }
                                                self.file_tree_context_menu_path = None;
                                                self.file_tree_context_menu_pos = None;
                                            }
                                        }
                                    });
                                });
                            
                            // Close menu on click outside
                            let ctx = ui.ctx();
                            if ctx.input(|i| i.pointer.primary_clicked() || i.pointer.secondary_clicked()) {
                                if let Some(click_pos) = ctx.input(|i| i.pointer.interact_pos()) {
                                    let menu_rect = menu_response.response.rect;
                                    if !menu_rect.contains(click_pos) {
                                        self.file_tree_context_menu_path = None;
                                        self.file_tree_context_menu_pos = None;
                                    }
                                }
                            }
                        }
                    }
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.label("No directory opened");
                        ui.add_space(10.0);
                        if ui.button("Open Directory").clicked() {
                            self.open_directory();
                        }
                    });
                }
            }
        ).response.rect
    }

    /// 渲染水平分割器
    fn render_horizontal_splitter(
        &mut self,
        ui: &mut egui::Ui,
        available_rect: &egui::Rect,
        file_tree_rect: &egui::Rect,
        min_file_tree_width: f32,
        min_midi_editor_width: f32,
    ) {
        let splitter_width = 4.0;
        let splitter_rect = egui::Rect::from_min_size(
            egui::pos2(file_tree_rect.max.x, file_tree_rect.min.y),
            egui::Vec2::new(splitter_width, file_tree_rect.height())
        );
        let splitter_response = ui.allocate_rect(splitter_rect, egui::Sense::click_and_drag());
        
        // Use default separator style
        ui.painter().line_segment(
            [egui::pos2(splitter_rect.center().x, splitter_rect.min.y),
             egui::pos2(splitter_rect.center().x, splitter_rect.max.y)],
            ui.style().visuals.widgets.noninteractive.bg_stroke
        );
        
        // Handle horizontal splitter dragging
        if splitter_response.drag_started() {
            self.dragging_horizontal_splitter = true;
        }
        if self.dragging_horizontal_splitter {
            if let Some(pointer) = ui.input(|i| i.pointer.hover_pos()) {
                let new_ratio = ((pointer.x - available_rect.min.x) / available_rect.width())
                    .clamp(min_file_tree_width / available_rect.width(), 1.0 - min_midi_editor_width / available_rect.width());
                self.horizontal_split_ratio = new_ratio;
            }
            if ui.input(|i| i.pointer.any_released()) {
                self.dragging_horizontal_splitter = false;
            }
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        } else if splitter_response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
    }

    /// 渲染MIDI编辑器标签页
    fn render_midi_editors(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            // MIDI editor tab bar
            if !self.midi_editors.is_empty() {
                ui.horizontal(|ui| {
                    let mut to_remove: Option<usize> = None;
                    let mut to_save: Option<usize> = None;
                    
                    for (index, tab) in self.midi_editors.iter().enumerate() {
                        let is_active = self.active_midi_tab == Some(index);
                        
                        // Tab button with close button
                        ui.horizontal(|ui| {
                            if ui.selectable_label(is_active, &tab.name).clicked() {
                                self.active_midi_tab = Some(index);
                            }
                            
                            // Close button
                            if ui.small_button("✕").clicked() {
                                to_remove = Some(index);
                            }
                        });
                    }
                    
                    ui.separator();
                    
                    // Save button (for active editor)
                    if let Some(active_index) = self.active_midi_tab {
                        if self.midi_editors.get(active_index).and_then(|t| t.file_path.as_ref()).is_some() {
                            if ui.button("💾 Save").clicked() {
                                to_save = Some(active_index);
                            }
                        }
                    }
                    
                    // Add new MIDI editor button
                    if ui.button("+").clicked() {
                        self.add_midi_editor();
                    }
                    
                    // Save tab if needed
                    if let Some(index) = to_save {
                        if let Err(e) = self.save_midi_editor(index) {
                            log::error!("Failed to save: {}", e);
                        }
                    }
                    
                    // Remove tab if needed
                    if let Some(index) = to_remove {
                        self.close_midi_editor(index);
                    }
                });
                
                ui.separator();
                
                // Active MIDI editor content
                if let Some(active_index) = self.active_midi_tab {
                    if let Some(tab) = self.midi_editors.get_mut(active_index) {
                        let content_rect = ui.available_rect_before_wrap();
                        ui.allocate_ui(content_rect.size(), |ui| {
                            tab.editor.ui(ui);
                        });
                    }
                }
            } else {
                // No MIDI editors, show placeholder
                ui.centered_and_justified(|ui| {
                    ui.label("No MIDI editors open");
                    ui.add_space(10.0);
                    if ui.button("New MIDI Editor").clicked() {
                        self.add_midi_editor();
                    }
                });
            }
        });
    }
}


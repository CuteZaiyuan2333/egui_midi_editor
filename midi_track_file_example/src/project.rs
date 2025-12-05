//! 项目文件操作模块
//!
//! 处理项目的加载、保存、导出等文件操作。

use crate::MidiTrackFileApp;
use egui_track::{TrackEditor, ProjectFile};
use std::path::PathBuf;
use rfd::FileDialog;

impl MidiTrackFileApp {
    /// 加载项目文件
    pub fn load_project(&mut self, path: &PathBuf) {
        match ProjectFile::load_from_path(path) {
            Ok(project_file) => {
                log::info!("Project loaded: {:?}", path);
                log::info!("Track count: {}", project_file.tracks.len());
                self.current_project_path = Some(path.clone());
                
                // 将文件树设置到项目目录（项目文件夹）
                if let Some(project_dir) = path.parent() {
                    let project_dir_path = project_dir.to_path_buf();
                    self.file_tree = Some(egui_file_tree::FileTree::new(project_dir_path.clone()));
                    log::info!("File tree set to project directory: {:?}", project_dir_path);
                }
                
                // 恢复轨道编辑器状态
                // 注意：这里需要直接设置 tracks，但 TrackEditor 可能没有公开的 setter
                // 暂时使用 execute_command 逐个创建轨道和剪辑
                // TODO: 当 TrackEditor 提供批量设置方法时优化
                
                // 清除现有轨道
                let existing_tracks: Vec<_> = self.track_editor.tracks().iter()
                    .map(|t| t.id)
                    .collect();
                for track_id in existing_tracks {
                    use egui_track::TrackEditorCommand;
                    self.track_editor.execute_command(TrackEditorCommand::DeleteTrack { track_id });
                }
                
                // 恢复轨道和剪辑
                use egui_track::TrackEditorCommand;
                for track in &project_file.tracks {
                    self.track_editor.execute_command(TrackEditorCommand::CreateTrack {
                        name: track.name.clone(),
                    });
                    
                    // 设置轨道属性
                    let track_id = track.id;
                    self.track_editor.execute_command(TrackEditorCommand::SetTrackMute {
                        track_id,
                        muted: track.muted,
                    });
                    self.track_editor.execute_command(TrackEditorCommand::SetTrackSolo {
                        track_id,
                        solo: track.solo,
                    });
                    self.track_editor.execute_command(TrackEditorCommand::SetTrackVolume {
                        track_id,
                        volume: track.volume,
                    });
                    self.track_editor.execute_command(TrackEditorCommand::SetTrackPan {
                        track_id,
                        pan: track.pan,
                    });
                    
                    // 恢复剪辑
                    for clip in &track.clips {
                        self.track_editor.execute_command(TrackEditorCommand::CreateClip {
                            track_id,
                            start: clip.start_time,
                            duration: clip.duration,
                            clip_type: clip.clip_type.clone(),
                        });
                    }
                }
                
                // 恢复时间轴状态
                self.track_editor.execute_command(TrackEditorCommand::SetBPM {
                    bpm: project_file.timeline.bpm,
                });
                self.track_editor.execute_command(TrackEditorCommand::SetTimeSignature {
                    numer: project_file.timeline.time_signature.0,
                    denom: project_file.timeline.time_signature.1,
                });
                self.track_editor.execute_command(TrackEditorCommand::SetPlayhead {
                    position: project_file.timeline.playhead_position,
                });
            }
            Err(e) => {
                log::error!("Failed to load project: {}", e);
            }
        }
    }

    /// 创建新项目（弹出对话框让用户选择父目录）
    pub fn new_project(&mut self) {
        // 先让用户选择父目录
        if let Some(parent_dir) = FileDialog::new()
            .set_title("Select Parent Directory for New Project")
            .pick_folder()
        {
            // 打开输入对话框
            self.new_project_dialog_open = true;
            self.new_project_parent_dir = Some(parent_dir);
            self.new_project_name = String::new();
        }
    }
    
    /// 完成新项目创建（在用户输入项目名称后调用）
    pub fn finish_new_project(&mut self) {
        if let Some(parent_dir) = self.new_project_parent_dir.take() {
            let project_name = self.new_project_name.trim();
            if project_name.is_empty() {
                log::warn!("Project name cannot be empty");
                self.new_project_dialog_open = false;
                return;
            }
            
            // 创建项目文件夹路径
            let project_dir = parent_dir.join(project_name);
            
            // 创建项目文件路径（使用项目名称作为文件名，.tracks 扩展名）
            let project_json_path = project_dir.join(format!("{}.tracks", project_name));
            
            // 清除所有状态
            let options = egui_track::TrackEditorOptions::default();
            self.track_editor = TrackEditor::new(options);
            
            // 清除所有 MIDI 编辑器标签页
            self.midi_editors.clear();
            self.active_midi_tab = None;
            self.next_midi_tab_id = 0;
            
            // 停止播放
            self.playback_engine.stop();
            self.is_playing = false;
            
            // 清除拖拽状态
            self.dragging_file_path = None;
            
            // 清除文件树上下文菜单
            self.file_tree_context_menu_path = None;
            self.file_tree_context_menu_pos = None;
            
            // 保存新项目到指定路径（会自动创建文件夹和子文件夹）
            self.save_project_to_path(&project_json_path);
            
            // 更新文件树到项目目录
            self.file_tree = Some(egui_file_tree::FileTree::new(project_dir.clone()));
            
            log::info!("New project created at: {:?}", project_dir);
        }
        
        self.new_project_dialog_open = false;
        self.new_project_name.clear();
    }

    /// 打开项目文件对话框
    pub fn open_project(&mut self) {
        if let Some(path) = FileDialog::new()
            .add_filter("Track Project", &["tracks", "json"])
            .set_title("Open Project")
            .pick_file()
        {
            self.load_project(&path);
        }
    }

    /// 保存项目（如果已有路径则直接保存，否则弹出另存为对话框）
    pub fn save_project(&mut self) {
        if let Some(path) = self.current_project_path.clone() {
            self.save_project_to_path(&path);
        } else {
            self.save_project_as();
        }
    }

    /// 另存为项目
    pub fn save_project_as(&mut self) {
        if let Some(path) = FileDialog::new()
            .add_filter("Track Project", &["tracks"])
            .set_title("Save Project As")
            .set_file_name("project.tracks")
            .save_file()
        {
            self.save_project_to_path(&path);
        }
    }

    /// 保存项目到指定路径
    pub fn save_project_to_path(&mut self, path: &PathBuf) {
        // 清除所有 midi_state，只保留文件路径
        let mut tracks = self.track_editor.tracks().to_vec();
        for track in &mut tracks {
            for clip in &mut track.clips {
                if let egui_track::ClipType::Midi { ref mut midi_data } = clip.clip_type {
                    if let Some(ref mut midi_data) = midi_data {
                        // 只使用文件路径，不保存 midi_state
                        midi_data.midi_state = None;
                    }
                }
            }
        }
        
        let project_file = ProjectFile::new(
            self.track_editor.timeline().clone(),
            tracks,
        );
        
        match project_file.save_to_path(path) {
            Ok(_) => {
                self.current_project_path = Some(path.clone());
                log::info!("Project saved to: {:?}", path);
            }
            Err(e) => {
                log::error!("Failed to save project: {}", e);
            }
        }
    }

    /// 导出项目
    pub fn export_project(&mut self) {
        if let Some(path) = FileDialog::new()
            .add_filter("Track Project", &["json"])
            .set_title("Export Project")
            .set_file_name("export.json")
            .save_file()
        {
            self.save_project_to_path(&path);
            log::info!("Project exported to: {:?}", path);
        }
    }
}


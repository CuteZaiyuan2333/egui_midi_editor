mod project;
mod ui;
mod clip_operations;
mod playback;
mod midiclip;
mod audio;

use eframe::egui;
use egui_track::{TrackEditor, TrackEditorOptions, ClipId};
use egui_midi::{ui::MidiEditor, audio::{AudioEngine, PlaybackBackend}, structure::MidiState};
use egui_file_tree::FileTree;
use std::path::PathBuf;
use std::sync::Arc;
use rfd::FileDialog;

fn main() -> eframe::Result<()> {
    // 配置日志：设置默认级别为 info，确保日志输出到 stderr
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "MIDI & Track & File Example",
        native_options,
        Box::new(|_cc| Ok(Box::new(MidiTrackFileApp::new()))),
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TopTab {
    TrackEditor,
    OtherTools,
}

pub struct MidiEditorTab {
    #[allow(dead_code)]
    id: usize,
    name: String,
    editor: MidiEditor,
    associated_clip_id: Option<ClipId>,
    file_path: Option<PathBuf>,  // 关联的 .midiclip 文件路径
}

impl MidiEditorTab {
    fn new(id: usize, name: String, audio: Arc<dyn PlaybackBackend>) -> Self {
        Self {
            id,
            name,
            editor: MidiEditor::new(Some(audio)),
            associated_clip_id: None,
            file_path: None,
        }
    }
}

pub struct MidiTrackFileApp {
    // Top tabs
    top_active_tab: TopTab,
    
    // Track editor
    track_editor: TrackEditor,
    current_project_path: Option<PathBuf>,
    
    // MIDI editors
    midi_editors: Vec<MidiEditorTab>,
    active_midi_tab: Option<usize>,
    next_midi_tab_id: usize,
    
    // Shared audio engine for MIDI editors
    audio_engine: Arc<dyn PlaybackBackend>,
    
    // File tree
    file_tree: Option<FileTree>,
    
    // Splitter states
    vertical_split_ratio: f32,  // Ratio for top/bottom split (0.0-1.0)
    horizontal_split_ratio: f32,  // Ratio for left/right split in bottom area (0.0-1.0)
    dragging_vertical_splitter: bool,
    dragging_horizontal_splitter: bool,
    
    // Playback
    playback_engine: playback::MultiTrackPlaybackEngine,
    is_playing: bool,
    
    // File tree context menu
    file_tree_context_menu_path: Option<PathBuf>,
    file_tree_context_menu_pos: Option<egui::Pos2>,
    
    // Drag and drop
    dragging_file_path: Option<PathBuf>,
    
    // New project dialog
    new_project_dialog_open: bool,
    new_project_parent_dir: Option<PathBuf>,
    new_project_name: String,
}

impl MidiTrackFileApp {
    fn new() -> Self {
        let options = TrackEditorOptions::default();
        let track_editor = TrackEditor::new(options);
        
        let audio: Arc<dyn PlaybackBackend> = Arc::new(AudioEngine::new());
        
        // Initialize file tree with current directory
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let file_tree = Some(FileTree::new(current_dir));
        
        Self {
            top_active_tab: TopTab::TrackEditor,
            track_editor,
            current_project_path: None,
            midi_editors: Vec::new(),
            active_midi_tab: None,
            next_midi_tab_id: 0,
            audio_engine: audio,
            file_tree,
            vertical_split_ratio: 0.5,  // 50% for top, 50% for bottom
            horizontal_split_ratio: 0.2,  // 20% for file tree, 80% for MIDI editors
            dragging_vertical_splitter: false,
            dragging_horizontal_splitter: false,
            playback_engine: playback::MultiTrackPlaybackEngine::new(1),  // 初始 1 个轨道，会根据实际轨道数量动态扩展
            is_playing: false,
            file_tree_context_menu_path: None,
            file_tree_context_menu_pos: None,
            dragging_file_path: None,
            new_project_dialog_open: false,
            new_project_parent_dir: None,
            new_project_name: String::new(),
        }
    }

    fn add_midi_editor(&mut self) {
        let id = self.next_midi_tab_id;
        self.next_midi_tab_id += 1;
        let name = format!("MIDI {}", id + 1);
        let tab = MidiEditorTab::new(id, name, Arc::clone(&self.audio_engine));
        self.midi_editors.push(tab);
        self.active_midi_tab = Some(self.midi_editors.len() - 1);
    }

    fn close_midi_editor(&mut self, index: usize) {
        if index < self.midi_editors.len() {
            self.midi_editors.remove(index);
            if self.midi_editors.is_empty() {
                self.active_midi_tab = None;
            } else {
                // Adjust active tab index
                if let Some(active) = self.active_midi_tab {
                    if active >= index {
                        if active > 0 {
                            self.active_midi_tab = Some(active - 1);
                        } else {
                            self.active_midi_tab = Some(0);
                        }
                    }
                }
            }
        }
    }

    fn open_directory(&mut self) {
        if let Some(path) = FileDialog::new()
            .set_title("Select Directory")
            .pick_folder()
        {
            self.file_tree = Some(FileTree::new(path));
            log::info!("Opened directory: {:?}", self.file_tree.as_ref().unwrap().root_path());
        }
    }

    fn open_midi_file(&mut self, path: &PathBuf) {
        match std::fs::read(path) {
            Ok(data) => {
                match midly::Smf::parse(&data) {
                    Ok(smf) => {
                        match MidiState::from_smf_strict(&smf) {
                            Ok(state) => {
                                // Create a new MIDI editor tab with the loaded state
                                let id = self.next_midi_tab_id;
                                self.next_midi_tab_id += 1;
                                let file_name = path.file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("MIDI")
                                    .to_string();
                                let name = format!("{}", file_name);
                                
                                let mut tab = MidiEditorTab::new(id, name, Arc::clone(&self.audio_engine));
                                tab.editor.replace_state(state);
                                tab.file_path = Some(path.clone());
                                self.midi_editors.push(tab);
                                self.active_midi_tab = Some(self.midi_editors.len() - 1);
                                log::info!("Opened MIDI file: {:?}", path);
                            }
                            Err(e) => {
                                log::error!("Failed to parse MIDI file: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to parse MIDI file: {:?}", e);
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to read file: {:?}", e);
            }
        }
    }

    /// 打开 .midiclip 文件到 MIDI 编辑器
    fn open_midiclip_file(&mut self, path: &PathBuf) {
        match midiclip::load_midiclip_file(path) {
            Ok(state) => {
                let id = self.next_midi_tab_id;
                self.next_midi_tab_id += 1;
                let file_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("MIDI Clip")
                    .to_string();
                let name = format!("{}", file_name);
                
                let mut tab = MidiEditorTab::new(id, name, Arc::clone(&self.audio_engine));
                tab.editor.replace_state(state);
                tab.file_path = Some(path.clone());
                self.midi_editors.push(tab);
                self.active_midi_tab = Some(self.midi_editors.len() - 1);
                log::info!("Opened MIDI clip file: {:?}", path);
            }
            Err(e) => {
                log::error!("Failed to load MIDI clip file: {:?}", e);
            }
        }
    }

    /// 保存 MIDI 编辑器到关联的文件
    fn save_midi_editor(&mut self, editor_index: usize) -> Result<(), String> {
        if let Some(tab) = self.midi_editors.get_mut(editor_index) {
            if let Some(ref file_path) = tab.file_path {
                let state = tab.editor.snapshot_state();
                midiclip::save_midiclip_file(file_path, &state)
                    .map_err(|e| format!("Failed to save file: {:?}", e))?;
                log::info!("Saved MIDI clip file: {:?}", file_path);
                
                // 刷新所有使用该文件的剪辑预览
                let file_path_str = file_path.to_string_lossy().to_string();
                // 遍历所有轨道和剪辑，找到所有使用该文件路径的剪辑
                let mut clip_ids_to_refresh = Vec::new();
                for track in self.track_editor.tracks() {
                    for clip in &track.clips {
                        if let egui_track::ClipType::Midi { midi_data: Some(midi_data) } = &clip.clip_type {
                            if let Some(ref clip_file_path) = midi_data.midi_file_path {
                                if clip_file_path == &file_path_str {
                                    clip_ids_to_refresh.push(clip.id);
                                }
                            }
                        }
                    }
                }
                // 刷新所有匹配的剪辑预览
                for clip_id in clip_ids_to_refresh {
                    self.refresh_clip_preview(clip_id);
                }
                
                Ok(())
            } else {
                Err("No file associated with this editor".to_string())
            }
        } else {
            Err("Invalid editor index".to_string())
        }
    }

    /// 关联 MIDI 编辑器与剪辑
    pub fn associate_midi_editor_with_clip(&mut self, editor_index: usize, clip_id: ClipId) {
        if let Some(tab) = self.midi_editors.get_mut(editor_index) {
            tab.associated_clip_id = Some(clip_id);
        }
    }

    /// 获取编辑器关联的剪辑 ID
    pub fn get_clip_for_editor(&self, editor_index: usize) -> Option<ClipId> {
        self.midi_editors.get(editor_index)
            .and_then(|tab| tab.associated_clip_id)
    }

    /// 获取关联到指定剪辑的编辑器索引
    pub fn get_editor_for_clip(&self, clip_id: ClipId) -> Option<usize> {
        self.midi_editors.iter()
            .position(|tab| tab.associated_clip_id == Some(clip_id))
    }

    /// 从文件创建剪辑（用于拖放）
    fn create_clip_from_file(&mut self, file_path: PathBuf) {
        use egui_track::{TrackEditorCommand, ClipType, MidiClipData};
        use crate::midiclip;
        
        // 使用播放头位置作为开始时间
        let timeline = self.track_editor.timeline();
        let playhead_pos = timeline.playhead_position;
        
        // 尝试从文件加载 MIDI 数据以获取持续时间
        let duration = match midiclip::load_midiclip_file(&file_path) {
            Ok(state) => {
                // 计算持续时间（秒）
                if state.notes.is_empty() {
                    4.0  // 默认 4 秒
                } else {
                    let max_end_tick = state.notes.iter()
                        .map(|note| note.start + note.duration)
                        .max()
                        .unwrap_or(0);
                    crate::clip_operations::ticks_to_seconds(max_end_tick, state.bpm, state.ticks_per_beat)
                }
            }
            Err(_) => 4.0,  // 默认 4 秒
        };
        
        // 获取第一个轨道（或创建新轨道）
        let tracks = self.track_editor.tracks();
        let track_id = if tracks.is_empty() {
            // 创建新轨道
            self.track_editor.execute_command(TrackEditorCommand::CreateTrack {
                name: "Track 1".to_string(),
            });
            // 获取新创建的轨道
            self.track_editor.tracks().first().map(|t| t.id)
        } else {
            tracks.first().map(|t| t.id)
        };
        
        if let Some(track_id) = track_id {
            // 创建剪辑
            let file_path_str = file_path.to_string_lossy().to_string();
            let _clip_name = file_path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("MIDI Clip")
                .to_string();
            
            let midi_data = Some(MidiClipData {
                midi_file_path: Some(file_path_str),
                preview_notes: Vec::new(),
                midi_state: None,  // 优先使用文件路径
            });
            
            self.track_editor.execute_command(TrackEditorCommand::CreateClip {
                track_id,
                start: playhead_pos,
                duration,
                clip_type: ClipType::Midi { midi_data },
            });
            
            log::info!("Created clip from file: {:?} at position {}", file_path, playhead_pos);
        }
    }
    
    /// 从文件在指定位置创建剪辑（用于精确拖放）
    fn create_clip_from_file_at_position(&mut self, file_path: PathBuf, track_id: egui_track::TrackId, start_time: f64) {
        use egui_track::{TrackEditorCommand, ClipType, MidiClipData};
        use crate::midiclip;
        
        // 验证文件路径
        log::info!("[CLIP] create_clip_from_file_at_position called with file_path: {:?}", file_path);
        log::info!("[CLIP] File path details: exists={}, is_absolute={}, file_name={:?}", 
                  file_path.exists(), file_path.is_absolute(), file_path.file_name());
        
        // 如果文件不存在，尝试转换为绝对路径
        let file_path_to_use = if file_path.exists() {
            // 如果文件存在，尝试规范化路径（转换为绝对路径）
            file_path.canonicalize().unwrap_or_else(|_| file_path.clone())
        } else {
            // 如果文件不存在，尝试从当前工作目录解析相对路径
            if file_path.is_relative() {
                std::env::current_dir()
                    .ok()
                    .and_then(|cwd| cwd.join(&file_path).canonicalize().ok())
                    .unwrap_or_else(|| {
                        log::warn!("[CLIP] Cannot resolve relative path: {:?}, using as-is", file_path);
                        file_path.clone()
                    })
            } else {
                log::warn!("[CLIP] File does not exist: {:?}, but continuing anyway", file_path);
                file_path.clone()
            }
        };
        
        log::info!("[CLIP] Using file path: {:?} (original: {:?})", file_path_to_use, file_path);
        
        // 尝试从文件加载 MIDI 数据以获取持续时间和预览数据
        let (duration, preview_notes) = match midiclip::load_midiclip_file(&file_path_to_use) {
            Ok(state) => {
                // 计算持续时间（秒）
                let duration = if state.notes.is_empty() {
                    4.0  // 默认 4 秒
                } else {
                    let max_end_tick = state.notes.iter()
                        .map(|note| note.start + note.duration)
                        .max()
                        .unwrap_or(0);
                    crate::clip_operations::ticks_to_seconds(max_end_tick, state.bpm, state.ticks_per_beat)
                };
                
                // 生成预览音符
                let preview_notes = match crate::clip_operations::generate_preview_notes_from_file(&file_path_to_use) {
                    Ok(notes) => {
                        log::info!("[CLIP] Generated {} preview notes", notes.len());
                        notes
                    }
                    Err(e) => {
                        log::warn!("[CLIP] Failed to generate preview notes: {:?}, using empty preview", e);
                        Vec::new()
                    }
                };
                
                (duration, preview_notes)
            }
            Err(e) => {
                log::warn!("[CLIP] Failed to load MIDI file: {:?}, using default duration", e);
                (4.0, Vec::new())  // 默认 4 秒，无预览
            }
        };
        
        // 创建剪辑
        let file_path_str = file_path.to_string_lossy().to_string();
        let clip_name = file_path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("MIDI Clip")
            .to_string();
        
        log::info!("[CLIP] Creating clip with file_path_str: {:?}, clip_name: {:?}", file_path_str, clip_name);
        
        let midi_data = Some(MidiClipData {
            midi_file_path: Some(file_path_str.clone()),
            preview_notes,
            midi_state: None,  // 优先使用文件路径
        });
        
        log::info!("[CLIP] MidiClipData created with midi_file_path: {:?}", midi_data.as_ref().and_then(|d| d.midi_file_path.as_ref()));
        
        self.track_editor.execute_command(TrackEditorCommand::CreateClip {
            track_id,
            start: start_time,
            duration,
            clip_type: ClipType::Midi { midi_data },
        });
        
        log::info!("[CLIP] Created clip from file: {:?} at track {:?} position {} (duration: {})", 
                   file_path_to_use, track_id, start_time, duration);
        
        // Verify clip was created
        let tracks_after = self.track_editor.tracks();
        if let Some(track) = tracks_after.iter().find(|t| t.id == track_id) {
            let clip_count = track.clips.len();
            log::info!("[CLIP] Verification: Track {:?} now has {} clips", track_id, clip_count);
            if clip_count == 0 {
                log::error!("[CLIP] ERROR: Clip was not created! Track {:?} has no clips", track_id);
            } else {
                if let Some(last_clip) = track.clips.last() {
                    use egui_track::ClipType;
                    log::info!("[CLIP] Last clip: id={:?}, start={}, duration={}, name={}", 
                              last_clip.id, last_clip.start_time, last_clip.duration, last_clip.name);
                    match &last_clip.clip_type {
                        ClipType::Midi { midi_data } => {
                            if let Some(data) = midi_data {
                                log::info!("[CLIP] Clip midi_file_path: {:?}", data.midi_file_path);
                            } else {
                                log::warn!("[CLIP] Clip has no midi_data!");
                            }
                        }
                        _ => log::warn!("[CLIP] Clip is not a MIDI clip!"),
                    }
                }
            }
        } else {
            log::error!("[CLIP] ERROR: Track {:?} not found after clip creation!", track_id);
        }
    }
    
    /// 处理剪辑重命名事件
    fn handle_clip_renamed(&mut self, clip_id: egui_track::ClipId, new_name: String) {
        // 先收集文件路径（避免借用冲突）
        let file_path_opt = self.track_editor.tracks()
            .iter()
            .flat_map(|track| &track.clips)
            .find(|clip| clip.id == clip_id)
            .and_then(|clip| {
                if let egui_track::ClipType::Midi { midi_data: Some(midi_data) } = &clip.clip_type {
                    midi_data.midi_file_path.as_ref().map(|p| p.clone())
                } else {
                    None
                }
            });
        
        // 如果找到文件路径，重命名文件
        if let Some(file_path) = file_path_opt {
            if let Err(e) = self.rename_midiclip_file(&file_path, &new_name) {
                log::error!("Failed to rename MIDI clip file: {:?}", e);
            }
        }
    }
    
    /// 重命名 MIDI clip 文件
    fn rename_midiclip_file(&mut self, old_path: &str, new_name: &str) -> Result<(), String> {
        use std::fs;
        use std::path::Path;
        
        let old_path_buf = Path::new(old_path);
        if !old_path_buf.exists() {
            return Err(format!("File does not exist: {:?}", old_path));
        }
        
        // 获取原文件的目录和扩展名
        let parent_dir = old_path_buf.parent()
            .ok_or_else(|| "Cannot get parent directory".to_string())?;
        let extension = old_path_buf.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("midiclip");
        
        // 构建新路径
        let new_file_name = format!("{}.{}", new_name, extension);
        let new_path = parent_dir.join(&new_file_name);
        
        // 如果新文件名与旧文件名相同，不需要重命名
        if new_path == old_path_buf {
            return Ok(());
        }
        
        // 如果新文件已存在，返回错误
        if new_path.exists() {
            return Err(format!("File already exists: {:?}", new_path));
        }
        
        // 重命名文件
        fs::rename(old_path_buf, &new_path)
            .map_err(|e| format!("Failed to rename file: {:?}", e))?;
        
        log::info!("Renamed MIDI clip file: {:?} -> {:?}", old_path, new_path);
        
        // 更新所有引用该文件的剪辑
        let new_path_str = new_path.to_string_lossy().to_string();
        self.update_clip_file_path(old_path, &new_path_str);
        
        // 更新 MIDI 编辑器中的文件路径
        for tab in &mut self.midi_editors {
            if let Some(ref tab_path) = tab.file_path {
                if tab_path.to_string_lossy() == old_path {
                    tab.file_path = Some(new_path.clone());
                }
            }
        }
        
        Ok(())
    }
    
    /// 更新剪辑中的文件路径
    fn update_clip_file_path(&mut self, old_path: &str, new_path: &str) {
        use egui_track::TrackEditorCommand;
        
        // 从新路径提取文件名（不含扩展名）
        let new_name = std::path::Path::new(new_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("MIDI Clip")
            .to_string();
        
        // 收集需要更新的剪辑 ID（避免借用冲突）
        let mut clip_ids_to_update = Vec::new();
        for track in self.track_editor.tracks() {
            for clip in &track.clips {
                if let egui_track::ClipType::Midi { midi_data: Some(midi_data) } = &clip.clip_type {
                    if let Some(ref file_path) = midi_data.midi_file_path {
                        if file_path == old_path {
                            clip_ids_to_update.push(clip.id);
                        }
                    }
                }
            }
        }
        
        // 更新所有匹配的剪辑的文件路径和名称
        for clip_id in clip_ids_to_update {
            // 更新文件路径
            self.track_editor.execute_command(TrackEditorCommand::UpdateClipMidiFilePath {
                clip_id,
                new_file_path: new_path.to_string(),
            });
            // 更新名称
            self.track_editor.execute_command(TrackEditorCommand::RenameClip {
                clip_id,
                new_name: new_name.clone(),
            });
            log::info!("Updated clip file path and name: {} -> {} ({})", old_path, new_path, new_name);
        }
    }
    
    /// 刷新剪辑预览
    fn refresh_clip_preview(&mut self, clip_id: egui_track::ClipId) {
        use egui_track::TrackEditorCommand;
        use crate::clip_operations;
        
        // 找到剪辑并重新加载预览数据
        for track in self.track_editor.tracks() {
            if let Some(clip) = track.clips.iter().find(|c| c.id == clip_id) {
                if let egui_track::ClipType::Midi { midi_data: Some(midi_data) } = &clip.clip_type {
                    if let Some(ref file_path) = midi_data.midi_file_path {
                        let path = std::path::Path::new(file_path);
                        if path.exists() {
                            // 重新生成预览音符
                            match clip_operations::generate_preview_notes_from_file(path) {
                                Ok(preview_notes) => {
                                    log::info!("Refreshed preview for clip {:?}: {} notes", clip_id, preview_notes.len());
                                    // 使用 UpdateClipPreview 命令来更新预览
                                    self.track_editor.execute_command(TrackEditorCommand::UpdateClipPreview {
                                        clip_id,
                                        preview_notes,
                                    });
                                }
                                Err(e) => {
                                    log::warn!("Failed to generate preview notes: {:?}", e);
                                }
                            }
                        }
                    }
                }
                break;
            }
        }
    }
    
}

impl eframe::App for MidiTrackFileApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update playback engine
        let current_time = ctx.input(|i| i.time);
        let timeline_bpm = self.track_editor.timeline().bpm;
        self.playback_engine.update(current_time, self.track_editor.tracks(), timeline_bpm);
        
        // Update playback position in track editor
        // 使用 playback_engine.is_playing() 来判断，而不是 self.is_playing
        if self.playback_engine.is_playing() {
            let position = self.playback_engine.position();
            // Sync track editor playhead position
            use egui_track::TrackEditorCommand;
            self.track_editor.execute_command(TrackEditorCommand::SetPlayhead { position });
            self.is_playing = true;  // 同步状态
        } else {
            self.is_playing = false;  // 同步状态
        }

        // Render UI components
        self.render_menu_bar(ctx);
        self.render_status_bar(ctx);
        self.render_main_content(ctx);
        self.render_new_project_dialog(ctx);

        // Handle track editor events
        for event in self.track_editor.take_events() {
            log::info!("[TrackEditorEvent] {:?}", event);
            
            // 处理播放状态变化事件
            if let egui_track::TrackEditorEvent::PlaybackStateChanged { is_playing } = event {
                let current_time = ctx.input(|i| i.time);
                if is_playing {
                    // 如果轨道编辑器开始播放，同步启动播放引擎
                    if !self.playback_engine.is_playing() {
                        // 从轨道编辑器的播放头位置开始播放
                        let start_position = self.track_editor.timeline().playhead_position;
                        self.playback_engine.start_from_position(current_time, start_position);
                        self.is_playing = true;
                        log::info!("[Playback] Started from position: {}", start_position);
                    } else {
                        // 如果已经在播放，恢复播放
                        self.playback_engine.resume(current_time);
                        self.is_playing = true;
                        log::info!("[Playback] Resumed");
                    }
                } else {
                    // 如果轨道编辑器暂停或停止，检查播放头位置
                    let playhead_position = self.track_editor.timeline().playhead_position;
                    if playhead_position == 0.0 {
                        // 如果播放头在 0 位置，说明是停止操作
                        if self.playback_engine.is_playing() {
                            self.playback_engine.stop();
                            self.is_playing = false;
                            log::info!("[Playback] Stopped");
                        }
                    } else {
                        // 如果播放头不在 0 位置，说明是暂停操作
                        if self.playback_engine.is_playing() {
                            self.playback_engine.pause();
                            self.is_playing = false;
                            log::info!("[Playback] Paused");
                        }
                    }
                }
            }
            
            // 处理播放头位置变化事件
            if let egui_track::TrackEditorEvent::PlayheadChanged { position } = event {
                // 如果正在播放，同步播放引擎的位置（但排除手动拖动播放头的情况）
                // 注意：如果用户手动拖动播放头，我们不应该 seek，因为这会打断播放
                // 只有在播放过程中自动更新播放头位置时才需要同步
                // 这里我们只在播放引擎正在播放且位置变化较大时才 seek（说明可能是手动拖动）
                if self.playback_engine.is_playing() {
                    let current_position = self.playback_engine.position();
                    let position_diff = (position - current_position).abs();
                    // 如果位置差异较大（超过 0.1 秒），说明可能是手动拖动，需要 seek
                    if position_diff > 0.1 {
                        self.playback_engine.seek(position);
                        log::debug!("[Playback] Seeked to position: {} (diff: {})", position, position_diff);
                    }
                }
            }
            
            // 处理双击剪辑事件
            if let egui_track::TrackEditorEvent::ClipDoubleClicked { clip_id } = event {
                // 查找剪辑并打开对应的 MIDI 编辑器
                for track in self.track_editor.tracks() {
                    if let Some(clip) = track.clips.iter().find(|c| c.id == clip_id) {
                        if let egui_track::ClipType::Midi { midi_data: Some(midi_data) } = &clip.clip_type {
                            if let Some(ref file_path) = midi_data.midi_file_path {
                                let path = std::path::PathBuf::from(file_path);
                                if path.exists() {
                                    self.open_midiclip_file(&path);
                                    log::info!("Opened MIDI clip from double-click: {:?}", path);
                                }
                            }
                        }
                        break;
                    }
                }
            }
            
            // 处理剪辑重命名事件
            if let egui_track::TrackEditorEvent::ClipRenamed { clip_id, new_name } = event {
                self.handle_clip_renamed(clip_id, new_name);
            }
        }
        
        // Handle MIDI editor events
        for tab in &mut self.midi_editors {
            for event in tab.editor.take_events() {
                log::info!("[MidiEditorEvent] {:?}", event);
            }
        }
        
        // Request repaint for smooth playback
        if self.is_playing {
            ctx.request_repaint();
        }
    }
}

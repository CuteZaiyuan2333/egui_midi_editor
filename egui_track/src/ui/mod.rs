//! UI 模块
//!
//! 包含音轨编辑器的主要 UI 组件，基于 MIDI 编辑器的钢琴卷帘实现。

mod clip;
mod toolbar;

use crate::editor::{TrackEditorCommand, TrackEditorEvent};
use crate::structure::{Track, Clip, TrackId, ClipId, TimelineState, ClipType};
use egui::*;
use std::collections::BTreeSet;
use std::rc::Rc;
use std::cell::RefCell;

// UI 常量
const CLIP_TITLE_BAR_HEIGHT: f32 = 18.0;
const CLIP_TITLE_BAR_MIN_HEIGHT: f32 = 4.0;
const CLIP_EDGE_THRESHOLD: f32 = 5.0;
const TRACK_BUTTON_SIZE: f32 = 18.0;
const TRACK_MONITOR_BUTTON_WIDTH: f32 = 26.0;
const TRACK_VOLUME_SLIDER_WIDTH: f32 = 33.75;
const TRACK_PAN_SLIDER_WIDTH: f32 = 22.5;
const TRACK_CONTROL_SLIDER_HEIGHT: f32 = 20.0;
const TRACK_CONTEXT_MENU_THRESHOLD: f32 = 5.0;
const TIMELINE_MEASURE_LABEL_OFFSET_X: f32 = 4.0;
const TIMELINE_MEASURE_LABEL_OFFSET_Y: f32 = 15.0;
const TIMELINE_MEASURE_LINE_OFFSET: f32 = 5.0;

/// 音轨编辑器的配置选项
///
/// 用于自定义编辑器的外观和行为。
///
/// # 示例
///
/// ```rust
/// use egui_track::TrackEditorOptions;
///
/// let options = TrackEditorOptions {
///     default_track_height: 100.0,
///     min_clip_width: 30.0,
///     track_header_width: 250.0,
///     timeline_height: 40.0,
/// };
/// ```
#[derive(Clone)]
pub struct TrackEditorOptions {
    /// 默认轨道高度（像素）
    pub default_track_height: f32,
    /// 剪辑的最小宽度（像素）
    pub min_clip_width: f32,
    /// 轨道标题栏的宽度（像素）
    pub track_header_width: f32,
    /// 时间轴的高度（像素）
    pub timeline_height: f32,
}

impl Default for TrackEditorOptions {
    fn default() -> Self {
        Self {
            default_track_height: 96.0,  // 80.0 * 1.2
            min_clip_width: 20.0,
            track_header_width: 240.0,  // 200.0 * 1.2
            timeline_height: 30.0,      // 60.0 / 2
        }
    }
}

pub struct TrackEditor {
    tracks: Vec<Track>,
    timeline: TimelineState,
    selected_clips: BTreeSet<ClipId>,
    options: TrackEditorOptions,
    
    // Interaction state
    drag_action: DragAction,
    drag_start_pos: Option<Pos2>,
    drag_start_time: Option<f64>,
    drag_clip_id: Option<ClipId>,
    drag_pointer_offset: Option<Vec2>,  // 拖拽时指针相对于剪辑的偏移量
    editing_clip_name: Option<ClipId>,  // 正在编辑名称的剪辑
    editing_clip_name_value: Option<String>,  // 正在编辑的名称值（用于持久化编辑状态）
    track_context_menu_pos: Option<Pos2>,  // 轨道右键菜单位置
    track_context_menu_open_pos: Option<Pos2>,  // 轨道右键菜单打开时的位置
    track_context_menu_track_id: Option<TrackId>,  // 显示右键菜单的轨道ID
    selection_box_start: Option<Pos2>,
    selection_box_end: Option<Pos2>,
    is_panning: bool,
    pan_start_pos: Option<Pos2>,
    
    // Editor state
    metronome_enabled: bool,
    
    // Playback state
    is_playing: bool,
    last_update: f64,
    
    // Events
    pending_events: Vec<TrackEditorEvent>,
    event_listener: Option<Box<dyn FnMut(&TrackEditorEvent)>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DragAction {
    None,
    MoveClip,
    ResizeClipStart,
    ResizeClipEnd,
    PlayheadSeek,
}

impl TrackEditor {
    /// 将轨道索引转换为 y 坐标（参考 MIDI 编辑器的 note_to_y）
    fn track_to_y(&self, track_index: usize, timeline_height: f32) -> f32 {
        timeline_height + (track_index as f32 * self.timeline.zoom_y) + self.timeline.manual_scroll_y
    }


    /// 绘制虚线垂直线的工具函数（从 MIDI 编辑器复制）
    fn draw_dashed_vertical_line(painter: &Painter, x: f32, top: f32, bottom: f32, stroke: Stroke) {
        let dash_len = 2.0;
        let gap_len = 2.0;
        let mut y = top;
        while y < bottom {
            let next = (y + dash_len).min(bottom);
            painter.line_segment([Pos2::new(x, y), Pos2::new(x, next)], stroke.clone());
            y += dash_len + gap_len;
        }
    }


    pub fn new(options: TrackEditorOptions) -> Self {
        Self {
            tracks: Vec::new(),
            timeline: TimelineState::default(),
            selected_clips: BTreeSet::new(),
            options,
            drag_action: DragAction::None,
            drag_start_pos: None,
            drag_start_time: None,
            drag_clip_id: None,
            drag_pointer_offset: None,
            editing_clip_name: None,
            editing_clip_name_value: None,
            track_context_menu_pos: None,
            track_context_menu_open_pos: None,
            track_context_menu_track_id: None,
            selection_box_start: None,
            selection_box_end: None,
            is_panning: false,
            pan_start_pos: None,
            metronome_enabled: false,
            is_playing: false,
            last_update: 0.0,
            pending_events: Vec::new(),
            event_listener: None,
        }
    }

    /// 设置事件监听器
    ///
    /// 当编辑器发生事件时，会调用此监听器。
    ///
    /// # 参数
    ///
    /// * `listener` - 事件回调函数
    ///
    /// # 示例
    ///
    /// ```rust
    /// use egui_track::{TrackEditor, TrackEditorOptions, TrackEditorEvent};
    ///
    /// let mut editor = TrackEditor::new(TrackEditorOptions::default());
    /// editor.set_event_listener(Box::new(|event| {
    ///     match event {
    ///         TrackEditorEvent::ClipSelected { clip_id } => {
    ///             println!("Clip selected: {:?}", clip_id);
    ///         }
    ///         _ => {}
    ///     }
    /// }));
    /// ```
    pub fn set_event_listener(&mut self, listener: Box<dyn FnMut(&TrackEditorEvent)>) {
        self.event_listener = Some(listener);
    }

    /// 获取并清空待处理的事件列表
    ///
    /// # 返回
    ///
    /// 自上次调用以来累积的所有事件
    ///
    /// # 示例
    ///
    /// ```rust
    /// use egui_track::{TrackEditor, TrackEditorOptions};
    ///
    /// let mut editor = TrackEditor::new(TrackEditorOptions::default());
    /// // ... 用户交互 ...
    /// let events = editor.take_events();
    /// for event in events {
    ///     println!("Event: {:?}", event);
    /// }
    /// ```
    pub fn take_events(&mut self) -> Vec<TrackEditorEvent> {
        std::mem::take(&mut self.pending_events)
    }

    /// 执行编辑命令
    ///
    /// 用于程序化地操作编辑器，例如创建剪辑、移动剪辑等。
    ///
    /// # 参数
    ///
    /// * `command` - 要执行的命令
    ///
    /// # 示例
    ///
    /// ```rust
    /// use egui_track::{TrackEditor, TrackEditorOptions, TrackEditorCommand, ClipType};
    ///
    /// let mut editor = TrackEditor::new(TrackEditorOptions::default());
    /// // 先创建一个轨道
    /// editor.execute_command(TrackEditorCommand::CreateTrack {
    ///     name: "Track 1".to_string(),
    /// });
    /// // 然后创建剪辑
    /// if let Some(track) = editor.tracks().first() {
    ///     editor.execute_command(TrackEditorCommand::CreateClip {
    ///         track_id: track.id,
    ///         start: 0.0,
    ///         duration: 4.0,
    ///         clip_type: ClipType::Midi { midi_data: None },
    ///     });
    /// }
    /// ```
    pub fn execute_command(&mut self, command: TrackEditorCommand) {
        match command {
            TrackEditorCommand::CreateClip { track_id, start, duration, clip_type } => {
                self.create_clip(track_id, start, duration, clip_type);
            }
            TrackEditorCommand::DeleteClip { clip_id } => {
                self.delete_clip(clip_id);
            }
            TrackEditorCommand::MoveClip { clip_id, new_track_id, new_start, disable_snap } => {
                self.move_clip(clip_id, new_track_id, new_start, disable_snap);
            }
            TrackEditorCommand::ResizeClip { clip_id, new_duration, resize_from_start, disable_snap } => {
                self.resize_clip(clip_id, new_duration, resize_from_start, disable_snap);
            }
            TrackEditorCommand::SplitClip { clip_id, split_time } => {
                self.split_clip(clip_id, split_time);
            }
            TrackEditorCommand::RenameClip { clip_id, new_name } => {
                self.rename_clip(clip_id, new_name);
            }
            TrackEditorCommand::CreateTrack { name } => {
                self.create_track(name);
            }
            TrackEditorCommand::DeleteTrack { track_id } => {
                self.delete_track(track_id);
            }
            TrackEditorCommand::RenameTrack { track_id, new_name } => {
                self.rename_track(track_id, new_name);
            }
            TrackEditorCommand::SetPlayhead { position } => {
                self.timeline.playhead_position = position;
                self.emit_event(TrackEditorEvent::PlayheadChanged { position });
            }
            TrackEditorCommand::SetTimeSignature { numer, denom } => {
                self.timeline.time_signature = (numer, denom);
                self.emit_event(TrackEditorEvent::TimeSignatureChanged { numer, denom });
            }
            TrackEditorCommand::SetBPM { bpm } => {
                self.timeline.bpm = bpm.clamp(20.0, 400.0);
                self.emit_event(TrackEditorEvent::BPMChanged { bpm: self.timeline.bpm });
            }
            TrackEditorCommand::SetMetronome { enabled } => {
                self.metronome_enabled = enabled;
                self.emit_event(TrackEditorEvent::MetronomeChanged { enabled });
            }
            TrackEditorCommand::SetSnapEnabled { enabled } => {
                self.timeline.snap_enabled = enabled;
                self.emit_event(TrackEditorEvent::SnapEnabledChanged { enabled });
            }
            TrackEditorCommand::SetSnapInterval { interval } => {
                self.timeline.snap_interval = interval.max(1);
                self.emit_event(TrackEditorEvent::SnapIntervalChanged { interval: self.timeline.snap_interval });
            }
            TrackEditorCommand::SetPlayback { is_playing } => {
                self.is_playing = is_playing;
                if !is_playing {
                    // 暂停时更新 last_update，避免下次播放时出现大跳跃
                    self.last_update = 0.0; // 将在 ui() 中更新为当前时间
                }
                self.emit_event(TrackEditorEvent::PlaybackStateChanged { is_playing });
            }
            TrackEditorCommand::StopPlayback => {
                self.is_playing = false;
                self.timeline.playhead_position = 0.0;
                self.last_update = 0.0; // 将在 ui() 中更新为当前时间
                self.emit_event(TrackEditorEvent::PlaybackStateChanged { is_playing: false });
                self.emit_event(TrackEditorEvent::PlayheadChanged { position: 0.0 });
            }
            TrackEditorCommand::SetTrackMute { track_id, muted } => {
                if let Some(track) = self.tracks.iter_mut().find(|t| t.id == track_id) {
                    track.muted = muted;
                    self.emit_event(TrackEditorEvent::TrackMuteChanged { track_id, muted });
                }
            }
            TrackEditorCommand::SetTrackSolo { track_id, solo } => {
                if let Some(track) = self.tracks.iter_mut().find(|t| t.id == track_id) {
                    track.solo = solo;
                    self.emit_event(TrackEditorEvent::TrackSoloChanged { track_id, solo });
                }
            }
            TrackEditorCommand::SetTrackVolume { track_id, volume } => {
                let new_volume = volume.clamp(0.0, 1.0);
                if let Some(track) = self.tracks.iter_mut().find(|t| t.id == track_id) {
                    track.volume = new_volume;
                }
                self.emit_event(TrackEditorEvent::TrackVolumeChanged { track_id, volume: new_volume });
            }
            TrackEditorCommand::SetTrackPan { track_id, pan } => {
                let new_pan = pan.clamp(-1.0, 1.0);
                if let Some(track) = self.tracks.iter_mut().find(|t| t.id == track_id) {
                    track.pan = new_pan;
                }
                self.emit_event(TrackEditorEvent::TrackPanChanged { track_id, pan: new_pan });
            }
            TrackEditorCommand::SetTrackRecordArm { track_id, armed } => {
                if let Some(track) = self.tracks.iter_mut().find(|t| t.id == track_id) {
                    track.record_arm = armed;
                    self.emit_event(TrackEditorEvent::TrackRecordArmChanged { track_id, armed });
                }
            }
            TrackEditorCommand::SetTrackMonitor { track_id, monitor } => {
                if let Some(track) = self.tracks.iter_mut().find(|t| t.id == track_id) {
                    track.monitor = monitor;
                    self.emit_event(TrackEditorEvent::TrackMonitorChanged { track_id, monitor });
                }
            }
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        // 播放时的自动时间更新（参考 MIDI 编辑器的实现）
        if self.is_playing {
            ui.ctx().request_repaint();
            let now = ui.input(|i| i.time);
            let dt = now - self.last_update;
            self.last_update = now;

            if dt > 0.0 && dt < 1.0 {
                // 避免大跳跃（例如窗口失去焦点后恢复）
                self.timeline.playhead_position += dt;
                self.emit_event(TrackEditorEvent::PlayheadChanged {
                    position: self.timeline.playhead_position,
                });
            }
        } else {
            // 非播放状态时，更新 last_update 以便下次播放时正确计算时间差
            self.last_update = ui.input(|i| i.time);
        }

        let available_size = ui.available_size();
        ui.set_min_size(available_size);

        ui.vertical(|ui| {
            // Toolbar at the top (水平布局，与 MIDI 编辑器一致)
                let mut toolbar = toolbar::Toolbar::new(&self.timeline);
                toolbar.set_metronome(self.metronome_enabled);
            toolbar.set_playing(self.is_playing);
            toolbar.set_current_time(self.timeline.playhead_position);
                toolbar.ui(ui, &mut |cmd| {
                    self.execute_command(cmd);
                });
                
            // 主编辑区域（基于 MIDI 编辑器的 ui_piano_roll）
            self.ui_track_roll(ui);
        });
    }

    /// 主编辑区域（基于 MIDI 编辑器的 ui_piano_roll 函数）
    fn ui_track_roll(&mut self, ui: &mut Ui) {
        let key_width = self.options.track_header_width;
        let timeline_height = self.options.timeline_height;

        // Track Roll ScrollArea（参考 MIDI 编辑器的 Piano Roll ScrollArea）
        ScrollArea::both()
            .auto_shrink([false, false])
            .enable_scrolling(false) // 禁用滚轮滚动，使用中键拖拽
            .show(ui, |ui| {
                let available_size = ui.available_size();
                let (rect, response) =
                    ui.allocate_exact_size(available_size, Sense::click_and_drag());

                // 处理缩放（Ctrl/Alt + 滚轮）
                self.handle_zoom(ui, &rect, key_width, timeline_height);

                // 处理中键拖拽平移（参考 MIDI 编辑器的实现）
                self.handle_panning(ui);

                // 限制垂直滚动
                self.clamp_vertical_scroll(&rect, timeline_height);

                let mut pointer_consumed = false;
                let note_offset_x = rect.min.x + key_width + self.timeline.manual_scroll_x;

                // 坐标转换函数
                let tick_to_x = |tick: u64, zoom_x: f32, ticks_per_beat: u16| -> f32 {
                    (tick as f32 / ticks_per_beat as f32) * zoom_x
                };

                // 处理时间轴交互（播放头定位）
                if let Some(pointer) = response.interact_pointer_pos() {
                    let in_timeline = pointer.y < rect.min.y + timeline_height
                        && pointer.x >= rect.min.x + key_width;
                    
                    if in_timeline {
                        let modifiers = ui.input(|i| i.modifiers);
                        let disable_snap = modifiers.alt;
                        
                        // 将指针位置转换为 tick
                        let mut x = pointer.x - (rect.min.x + key_width);
                        x = (x - self.timeline.manual_scroll_x).max(0.0);
                        let beats = x / self.timeline.zoom_x;
                        let seconds_per_beat = 60.0 / self.timeline.bpm;
                        let seconds_per_tick = seconds_per_beat / self.timeline.ticks_per_beat as f32;
                        let tick = (beats * seconds_per_beat / seconds_per_tick) as i64;
                        let snapped_tick = self.timeline.snap_tick(tick as u64, disable_snap) as i64;
                        
                        // 处理播放头定位
                        if ui.input(|i| i.pointer.primary_pressed()) && !matches!(self.drag_action, DragAction::MoveClip | DragAction::ResizeClipStart | DragAction::ResizeClipEnd) {
                            self.drag_action = DragAction::PlayheadSeek;
                            self.timeline.playhead_position = snapped_tick as f64 * seconds_per_tick as f64;
                            self.emit_event(TrackEditorEvent::PlayheadChanged {
                                position: self.timeline.playhead_position,
                            });
                            pointer_consumed = true;
                        }
                        
                        // 处理拖拽更新
                        if ui.input(|i| i.pointer.primary_down()) {
                            if self.drag_action == DragAction::PlayheadSeek {
                                self.timeline.playhead_position = snapped_tick as f64 * seconds_per_tick as f64;
                                self.emit_event(TrackEditorEvent::PlayheadChanged {
                                    position: self.timeline.playhead_position,
                                });
                                pointer_consumed = true;
                            }
                        }
                        
                        // 处理拖拽结束
                        if ui.input(|i| i.pointer.primary_released()) {
                            if self.drag_action == DragAction::PlayheadSeek {
                                self.drag_action = DragAction::None;
                            }
                        }
                        
                        // 更新光标
                        if self.drag_action != DragAction::PlayheadSeek {
                            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                        }
                    }
                }

                // 坐标转换函数
                let time_to_x = |time: f32, zoom_x: f32| -> f32 {
                    time * zoom_x
                };

                let painter = ui.painter_at(rect);
                let grid_top = rect.min.y + timeline_height;
                let grid_bottom = rect.max.y;
                let measure_line_color = Color32::from_rgb(210, 210, 210);
                let beat_line_color = Color32::from_rgb(140, 140, 140);
                let subdivision_color = Color32::from_rgb(90, 90, 90);
                let horizontal_line_color = Color32::from_rgb(90, 90, 90);
                let separator_color = Color32::from_rgb(130, 130, 130);

                // 绘制垂直网格（小节线、拍线、细分线）
                let tpb = self.timeline.ticks_per_beat.max(1) as u64;
                let denom = self.timeline.time_signature.1.max(1) as u64;
                let numer = self.timeline.time_signature.0.max(1) as u64;
                let ticks_per_measure = (tpb * numer * 4).saturating_div(denom).max(tpb);

                let visible_beats_start = (-self.timeline.manual_scroll_x / self.timeline.zoom_x).floor();
                let visible_beats_end = visible_beats_start + (rect.width() / self.timeline.zoom_x) + 2.0;
                let mut start_tick = (visible_beats_start * tpb as f32).floor() as i64;
                if start_tick < 0 {
                    start_tick = 0;
                }
                let end_tick = (visible_beats_end * tpb as f32).ceil() as i64;

                let subdivision = if self.timeline.zoom_x >= 220.0 {
                    8
                } else if self.timeline.zoom_x >= 90.0 {
                    4
                } else if self.timeline.zoom_x >= 45.0 {
                    2
                } else {
                    1
                };
                let tick_step = (tpb / subdivision).max(1);

                let mut tick = (start_tick / tick_step as i64) * tick_step as i64;
                if tick < 0 {
                    tick = 0;
                }

                while tick <= end_tick {
                    let x = note_offset_x + (tick as f32 / tpb as f32) * self.timeline.zoom_x;
                    if x >= rect.min.x && x <= rect.max.x {
                        if tick as u64 % ticks_per_measure == 0 {
                            painter.line_segment(
                                [Pos2::new(x, grid_top), Pos2::new(x, grid_bottom)],
                                Stroke::new(1.0, measure_line_color),
                            );
                        } else if tick as u64 % tpb == 0 {
                            painter.line_segment(
                                [Pos2::new(x, grid_top), Pos2::new(x, grid_bottom)],
                                Stroke::new(1.0, beat_line_color),
                            );
                        } else {
                            Self::draw_dashed_vertical_line(
                &painter,
                                x,
                                grid_top,
                                grid_bottom,
                                Stroke::new(1.0, subdivision_color),
                            );
                        }
                    }
                    tick += tick_step as i64;
                }

                // 绘制水平网格线（每个轨道一行）
                for (track_index, _track) in self.tracks.iter().enumerate() {
                    let y = rect.min.y + self.track_to_y(track_index, timeline_height);
                    if y > rect.min.y + timeline_height && y < rect.max.y {
                        painter.line_segment(
                            [
                                Pos2::new(rect.min.x + key_width, y),
                                Pos2::new(rect.max.x, y),
                            ],
                            Stroke::new(1.0, horizontal_line_color),
                        );
                    }
                }

                // 绘制最后一个轨道底部的分隔线
                if !self.tracks.is_empty() {
                    let last_track_index = self.tracks.len() - 1;
                    let bottom_y = rect.min.y + self.track_to_y(last_track_index, timeline_height) + self.timeline.zoom_y;
                    if bottom_y > rect.min.y + timeline_height && bottom_y < rect.max.y {
                        painter.line_segment(
                            [
                                Pos2::new(rect.min.x + key_width, bottom_y),
                                Pos2::new(rect.max.x, bottom_y),
                            ],
                            Stroke::new(1.0, horizontal_line_color),
                        );
                    }
                }

                // 绘制剪辑（参考 MIDI 编辑器的音符渲染）
                let clip_offset_y = rect.min.y + timeline_height + self.timeline.manual_scroll_y;
                
                // 计算可见时间范围（用于视口剔除）
                let visible_start_tick = if self.timeline.manual_scroll_x < 0.0 {
                    ((-self.timeline.manual_scroll_x / self.timeline.zoom_x) * self.timeline.ticks_per_beat as f32) as u64
                } else {
                    0
                };
                let visible_end_tick = visible_start_tick.saturating_add(
                    ((rect.width() / self.timeline.zoom_x) * self.timeline.ticks_per_beat as f32) as u64 + 1
                );
                
                // 收集可见剪辑的矩形
                let mut visible_clips: Vec<(ClipId, Rect, usize)> = Vec::new();
                for (track_index, track) in self.tracks.iter().enumerate() {
                    for clip in &track.clips {
                        let clip_start_tick = self.timeline.time_to_tick(clip.start_time);
                        let clip_end_tick = self.timeline.time_to_tick(clip.start_time + clip.duration);
                        
                        // 视口剔除：只处理可见时间范围内的剪辑
                        if clip_end_tick >= visible_start_tick && clip_start_tick <= visible_end_tick {
                            let x = note_offset_x
                                + tick_to_x(clip_start_tick, self.timeline.zoom_x, self.timeline.ticks_per_beat);
                            let y = clip_offset_y + (track_index as f32 * self.timeline.zoom_y);
                            let w = tick_to_x(clip_end_tick - clip_start_tick, self.timeline.zoom_x, self.timeline.ticks_per_beat).max(self.options.min_clip_width);
                            let h = self.timeline.zoom_y;
                            let clip_rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h));
                            
                            if clip_rect.intersects(rect) {
                                visible_clips.push((clip.id, clip_rect, track_index));
                            }
                        }
                    }
                }
                
                // 绘制剪辑
                for (clip_id, clip_rect, track_index) in &visible_clips {
                    let is_selected = self.selected_clips.contains(clip_id);
                    let color = if is_selected {
                        Color32::from_rgb(150, 250, 150)
                    } else {
                        Color32::from_rgb(100, 200, 100)
                    };
                    
                    // 绘制剪辑主体
                    painter.rect_filled(clip_rect.shrink(1.0), 4.0, color);
                    let stroke_width = if is_selected { 4.0 } else { 1.0 };
                    painter.rect_stroke(
                        clip_rect.shrink(1.0),
                        4.0,
                        Stroke::new(stroke_width, Color32::WHITE),
                    );
                    
                    // 绘制标题栏（如果剪辑足够高）
                    if clip_rect.height() > CLIP_TITLE_BAR_HEIGHT + CLIP_TITLE_BAR_MIN_HEIGHT {
                        let title_bar_rect = Rect::from_min_max(
                            Pos2::new(clip_rect.min.x, clip_rect.min.y),
                            Pos2::new(clip_rect.max.x, clip_rect.min.y + CLIP_TITLE_BAR_HEIGHT),
                        );
                        
                        // 标题栏背景（更深的颜色，方便显示文字）
                        let title_bg_color = Color32::from_rgba_premultiplied(
                            (color.r() as f32 * 0.6) as u8,
                            (color.g() as f32 * 0.6) as u8,
                            (color.b() as f32 * 0.6) as u8,
                            255
                        );
                        painter.rect_filled(title_bar_rect, 4.0, title_bg_color);
                        painter.line_segment(
                            [
                                Pos2::new(title_bar_rect.min.x, title_bar_rect.max.y),
                                Pos2::new(title_bar_rect.max.x, title_bar_rect.max.y),
                            ],
                            Stroke::new(1.0, Color32::from_gray(100)),
                        );
                        
                        // 查找剪辑以获取名称
                        if let Some(track) = self.tracks.get(*track_index) {
                            if let Some(clip) = track.clips.iter().find(|c| c.id == *clip_id) {
                                // 绘制剪辑名称
                                let text_pos = title_bar_rect.left_center() + Vec2::new(4.0, 0.0);
                                painter.text(
                                    text_pos,
                                    Align2::LEFT_CENTER,
                                    &clip.name,
                                    FontId::proportional(11.0),
                                    Color32::WHITE,
                                );
                            }
                        }
                    }
                }

                // 处理剪辑交互
                let base_x = rect.min.x + key_width;
                let base_y = rect.min.y + timeline_height;
                let manual_scroll_x = self.timeline.manual_scroll_x;
                let manual_scroll_y = self.timeline.manual_scroll_y;
                let zoom_x = self.timeline.zoom_x;
                let zoom_y = self.timeline.zoom_y;
                let ticks_per_beat = self.timeline.ticks_per_beat as f32;

                let pointer_to_tick = move |pos: Pos2| -> i64 {
                    let rel_x = pos.x - base_x - manual_scroll_x;
                    let beats = rel_x / zoom_x;
                    (beats * ticks_per_beat).round() as i64
                };
                let pointer_to_track = move |pos: Pos2| -> Option<usize> {
                    let keyboard_top = base_y + manual_scroll_y;
                    let rel_y = pos.y - keyboard_top;
                    let track_index = (rel_y / zoom_y).floor() as usize;
                    Some(track_index)
                };

                // 处理剪辑点击和拖拽
                for (clip_id, clip_rect, track_index) in &visible_clips {
                    // 计算标题栏区域
                    let title_bar_rect = if clip_rect.height() > CLIP_TITLE_BAR_HEIGHT + CLIP_TITLE_BAR_MIN_HEIGHT {
                        Some(Rect::from_min_max(
                            Pos2::new(clip_rect.min.x, clip_rect.min.y),
                            Pos2::new(clip_rect.max.x, clip_rect.min.y + CLIP_TITLE_BAR_HEIGHT),
                        ))
                    } else {
                        None
                    };
                    
                        if response.clicked_by(PointerButton::Primary) {
                        if let Some(pointer) = response.interact_pointer_pos() {
                            // 检查是否点击了标题栏
                            if let Some(title_rect) = title_bar_rect {
                                if title_rect.contains(pointer) {
                                    // 双击标题栏开始编辑
                                    if response.double_clicked() {
                                        // 获取当前剪辑名称并初始化编辑状态
                                        if let Some(track) = self.tracks.get(*track_index) {
                                            if let Some(clip) = track.clips.iter().find(|c| c.id == *clip_id) {
                                                self.editing_clip_name = Some(*clip_id);
                                                self.editing_clip_name_value = Some(clip.name.clone());
                                            }
                                        }
                                    }
                                    pointer_consumed = true;
                                } else if clip_rect.contains(pointer) {
                            let modifiers = ui.input(|i| i.modifiers);
                                    self.handle_clip_click(*clip_id, modifiers, clip::ClipHitRegion::Body);
                                    pointer_consumed = true;
                                }
                            } else if clip_rect.contains(pointer) {
                                let modifiers = ui.input(|i| i.modifiers);
                                self.handle_clip_click(*clip_id, modifiers, clip::ClipHitRegion::Body);
                                pointer_consumed = true;
                            }
                        }
                    }

                    if !matches!(self.drag_action, DragAction::MoveClip | DragAction::ResizeClipStart | DragAction::ResizeClipEnd) 
                        && ui.input(|i| i.pointer.primary_pressed()) {
                        if let Some(pointer) = response.interact_pointer_pos() {
                            // 排除标题栏区域
                            let in_title_bar = if clip_rect.height() > CLIP_TITLE_BAR_HEIGHT + CLIP_TITLE_BAR_MIN_HEIGHT {
                                pointer.y >= clip_rect.min.y && pointer.y <= clip_rect.min.y + CLIP_TITLE_BAR_HEIGHT
                            } else {
                                false
                            };
                            
                            if clip_rect.contains(pointer) && !in_title_bar {
                                // 检查是否在边缘（用于调整大小）
                                let hit_region = if (pointer.x - clip_rect.min.x) < CLIP_EDGE_THRESHOLD {
                                    clip::ClipHitRegion::LeftEdge
                                } else if (clip_rect.max.x - pointer.x) < CLIP_EDGE_THRESHOLD {
                                    clip::ClipHitRegion::RightEdge
                                } else {
                                    clip::ClipHitRegion::Body
                                };
                                
                            let drag_action = match hit_region {
                                clip::ClipHitRegion::LeftEdge => DragAction::ResizeClipStart,
                                clip::ClipHitRegion::RightEdge => DragAction::ResizeClipEnd,
                                clip::ClipHitRegion::Body => DragAction::MoveClip,
                            };
                                
                                self.drag_action = drag_action;
                                self.drag_clip_id = Some(*clip_id);
                                self.drag_start_pos = Some(pointer);
                                
                                // 计算指针相对于剪辑的偏移量（用于平滑拖拽）
                                self.drag_pointer_offset = Some(pointer - clip_rect.min);
                                
                                // 找到剪辑的开始时间
                                if let Some(track) = self.tracks.get(*track_index) {
                                    if let Some(clip) = track.clips.iter().find(|c| c.id == *clip_id) {
                                        self.drag_start_time = Some(clip.start_time);
                                    }
                                }
                                
                                pointer_consumed = true;
                            }
                        }
                    }

                    // 移除鼠标图标设置，使用默认鼠标图标
                }

                // 处理剪辑拖拽更新
                if matches!(self.drag_action, DragAction::MoveClip | DragAction::ResizeClipStart | DragAction::ResizeClipEnd) 
                    && ui.input(|i| i.pointer.primary_down()) {
                    // 使用 hover_pos 作为 interact_pointer_pos 的备用
                    let pointer = response.interact_pointer_pos()
                        .or_else(|| response.hover_pos());
                    
                    if let Some(pointer) = pointer {
                        if let Some(clip_id) = self.drag_clip_id {
                            if let Some(_start_time) = self.drag_start_time {
                                // 对于移动操作，使用偏移量计算新位置
                                let pointer_tick = if matches!(self.drag_action, DragAction::MoveClip) {
                                    // 使用偏移量计算，确保拖拽平滑
                                    if let Some(offset) = self.drag_pointer_offset {
                                        let adjusted_pointer = pointer - offset;
                                        pointer_to_tick(adjusted_pointer)
                                    } else {
                                        pointer_to_tick(pointer)
                                    }
                                } else {
                                    pointer_to_tick(pointer)
                                };
                                let new_start_tick = pointer_tick.max(0) as u64;
                                
                                // 对齐到网格
                                let disable_snap = ui.input(|i| i.modifiers.alt);
                                let snapped_tick = self.timeline.snap_tick(new_start_tick, disable_snap);
                                let snapped_time = self.timeline.tick_to_time(snapped_tick);
                                let clamped_time = snapped_time.max(0.0);
                                
                                match self.drag_action {
                                    DragAction::MoveClip => {
                                        // 确定目标轨道（使用原始指针位置，不是调整后的位置）
                                        let target_track_id = if let Some(track_index) = pointer_to_track(pointer) {
                                            if track_index < self.tracks.len() {
                                                Some(self.tracks[track_index].id)
                                            } else {
                                                None
                                            }
                                        } else {
                                            None
                                        };
                                        
                                        // 找到当前剪辑所在的轨道
                                        let mut current_track_id = None;
                                        for track in &self.tracks {
                                            if track.clips.iter().any(|c| c.id == clip_id) {
                                                current_track_id = Some(track.id);
                                                break;
                                            }
                                        }
                                        
                                        let final_track_id = target_track_id.unwrap_or_else(|| current_track_id.unwrap());
                                        
                                        self.execute_command(TrackEditorCommand::MoveClip {
                                            clip_id,
                                            new_track_id: final_track_id,
                                            new_start: clamped_time,
                                            disable_snap,
                                        });
                                    }
                                    DragAction::ResizeClipStart | DragAction::ResizeClipEnd => {
                                        // 找到剪辑
                                        for track in &self.tracks {
                                            if let Some(clip) = track.clips.iter().find(|c| c.id == clip_id) {
                                                let resize_from_start = self.drag_action == DragAction::ResizeClipStart;
                                                
                                                let clip_start_tick = self.timeline.time_to_tick(clip.start_time) as i64;
                                                let clip_end_tick = self.timeline.time_to_tick(clip.start_time + clip.duration) as i64;
                                                
                                                let (new_start_tick, new_end_tick) = if resize_from_start {
                                                    let new_start = pointer_tick.max(0);
                                                    (new_start, clip_end_tick)
                                                } else {
                                                    (clip_start_tick, pointer_tick.max(clip_start_tick + 1))
                                                };
                                                
                                                let snapped_start = self.timeline.snap_tick(new_start_tick as u64, disable_snap) as i64;
                                                let snapped_end = self.timeline.snap_tick(new_end_tick as u64, disable_snap) as i64;
                                                
                                                if resize_from_start {
                                                    let new_start_time = self.timeline.tick_to_time(snapped_start as u64);
                                                    let clamped_start_time = new_start_time.max(0.0);
                                                    let new_duration = self.timeline.tick_to_time(snapped_end as u64) - clamped_start_time;
                                                    
                                                    if clamped_start_time >= 0.0 {
                                                        self.execute_command(TrackEditorCommand::ResizeClip {
                                                            clip_id,
                                                            new_duration: new_duration.max(0.01),
                                                            resize_from_start: true,
                                                            disable_snap,
                                                        });
                                                        if clamped_start_time != new_start_time {
                                                            let mut clip_track_id = None;
                                                            for t in &self.tracks {
                                                                if t.clips.iter().any(|c| c.id == clip_id) {
                                                                    clip_track_id = Some(t.id);
                                                                    break;
                                                                }
                                                            }
                                                            if let Some(track_id) = clip_track_id {
                                                                self.execute_command(TrackEditorCommand::MoveClip {
                                                                    clip_id,
                                                                    new_track_id: track_id,
                                                                    new_start: clamped_start_time,
                                                                    disable_snap,
                                                                });
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    let new_duration = self.timeline.tick_to_time(snapped_end as u64) - clip.start_time;
                                                    self.execute_command(TrackEditorCommand::ResizeClip {
                                                        clip_id,
                                                        new_duration: new_duration.max(0.01),
                                                        resize_from_start: false,
                                                        disable_snap,
                                                    });
                                                }
                                                break;
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }

                // 检测拖拽结束：同时检查 drag_stopped 和 primary_released
                let drag_ended = response.drag_stopped() 
                    || ui.input(|i| i.pointer.primary_released());
                
                if drag_ended {
                    if matches!(self.drag_action, DragAction::MoveClip | DragAction::ResizeClipStart | DragAction::ResizeClipEnd) {
                self.drag_action = DragAction::None;
                self.drag_clip_id = None;
                self.drag_start_pos = None;
                self.drag_start_time = None;
                        self.drag_pointer_offset = None;
                    }
                }

                // 处理选择框
                if !pointer_consumed && ui.input(|i| i.pointer.primary_pressed()) {
                    if let Some(pointer) = response.interact_pointer_pos() {
                        let in_roll = pointer.x > rect.min.x + key_width
                            && pointer.y > rect.min.y + timeline_height;
                        if in_roll {
                            if !matches!(self.drag_action, DragAction::MoveClip | DragAction::ResizeClipStart | DragAction::ResizeClipEnd) {
                                self.selection_box_start = Some(pointer);
                                self.selection_box_end = Some(pointer);
                            }
                        }
                    }
                }

                if let Some(start) = self.selection_box_start {
                    if ui.input(|i| i.pointer.primary_down()) {
                        if let Some(pointer) = response.hover_pos() {
                            self.selection_box_end = Some(pointer);
                        }
                        if let Some(end) = self.selection_box_end {
                            let selection_rect = Rect::from_two_pos(start, end);
                            painter.rect_stroke(
                                selection_rect,
                                0.0,
                                Stroke::new(1.0, Color32::from_rgb(120, 200, 255)),
                            );
                        }
                    }

                    if ui.input(|i| i.pointer.primary_released()) {
                        if let Some(end) = self.selection_box_end {
                            let selection_rect = Rect::from_two_pos(start, end);
                            if !ui.input(|i| i.modifiers.shift) {
                                std::mem::take(&mut self.selected_clips);
                            }
                            
                            // 选择框内的剪辑
                            for (clip_id, clip_rect, _track_index) in &visible_clips {
                                if clip_rect.intersects(selection_rect) {
                                    self.selected_clips.insert(*clip_id);
                                }
                            }
                            
                            self.selection_box_start = None;
                            self.selection_box_end = None;
                        }
                    }
                }

                // 绘制时间轴（顶部栏）
                let timeline_rect =
                    Rect::from_min_size(rect.min, Vec2::new(rect.width(), timeline_height));
                painter.rect_filled(timeline_rect, 0.0, ui.visuals().window_fill());
                painter.line_segment(
                    [timeline_rect.left_bottom(), timeline_rect.right_bottom()],
                    Stroke::new(1.0, separator_color),
                );

                // 绘制时间轴标签（小节标记）
                let mut measure_tick = (start_tick as u64 / ticks_per_measure) * ticks_per_measure;
                while measure_tick as i64 <= end_tick {
                    let x = note_offset_x + (measure_tick as f32 / tpb as f32) * self.timeline.zoom_x;
                    if x >= rect.min.x + key_width - TIMELINE_MEASURE_LINE_OFFSET && x <= rect.max.x {
                        painter.line_segment(
                            [
                                Pos2::new(x, rect.min.y),
                                Pos2::new(x, rect.min.y + timeline_height),
                            ],
                            Stroke::new(1.0, measure_line_color),
                        );
                        let measure_index = (measure_tick / ticks_per_measure) + 1;
                        painter.text(
                            Pos2::new(x + TIMELINE_MEASURE_LABEL_OFFSET_X, rect.min.y + TIMELINE_MEASURE_LABEL_OFFSET_Y),
                            Align2::LEFT_CENTER,
                            format!("{}:1", measure_index),
                            FontId::proportional(11.0),
                            Color32::GRAY,
                        );
                    }
                    measure_tick += ticks_per_measure;
                }

                // 绘制播放头
                let playhead_x = note_offset_x
                    + time_to_x(
                        (self.timeline.playhead_position * self.timeline.bpm as f64 / 60.0) as f32,
                        self.timeline.zoom_x,
                    );
                if playhead_x > rect.min.x + key_width {
                    painter.line_segment(
                        [
                            Pos2::new(playhead_x, rect.min.y),
                            Pos2::new(playhead_x, rect.max.y),
                        ],
                        Stroke::new(2.0, Color32::from_rgba_premultiplied(100, 200, 255, 128)),
                    );
                }

                // 处理剪辑名称编辑（在绘制剪辑之后，使用独立的 UI 区域）
                if let Some(editing_clip_id) = self.editing_clip_name {
                    // 如果点击了其他地方，取消编辑
                    if response.clicked_elsewhere() {
                        self.editing_clip_name = None;
                        self.editing_clip_name_value = None;
                    } else {
                        // 找到正在编辑的剪辑
                        for (clip_id, clip_rect, track_index) in &visible_clips {
                            if *clip_id == editing_clip_id {
                                if clip_rect.height() > CLIP_TITLE_BAR_HEIGHT + CLIP_TITLE_BAR_MIN_HEIGHT {
                                    let title_bar_rect = Rect::from_min_max(
                                        Pos2::new(clip_rect.min.x, clip_rect.min.y),
                                        Pos2::new(clip_rect.max.x, clip_rect.min.y + CLIP_TITLE_BAR_HEIGHT),
                                    );
                                    
                                    // 获取或初始化编辑值
                                    let editing_value = self.editing_clip_name_value.as_mut();
                                    
                                    if let Some(name_value) = editing_value {
                                        // 创建命令收集器和状态标志（避免借用冲突）
                                        let pending_commands: Rc<RefCell<Vec<TrackEditorCommand>>> = Rc::new(RefCell::new(Vec::new()));
                                        let should_finish_editing: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
                                        let should_cancel_editing: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
                                        let new_name_value: Rc<RefCell<String>> = Rc::new(RefCell::new(name_value.clone()));
                                        
                                        let commands = pending_commands.clone();
                                        let finish_flag = should_finish_editing.clone();
                                        let cancel_flag = should_cancel_editing.clone();
                                        let name_ref = new_name_value.clone();
                                        let clip_id_for_edit = editing_clip_id;
                                        
                                        // 获取原始名称用于比较
                                        let original_name = if let Some(track) = self.tracks.get(*track_index) {
                                            track.clips.iter()
                                                .find(|c| c.id == editing_clip_id)
                                                .map(|c| c.name.clone())
                                        } else {
                                            None
                                        };
                                        
                                        // 使用 Area 来创建独立的编辑区域
                                        let edit_area = egui::Area::new(egui::Id::new(("clip_edit", editing_clip_id)))
                                            .fixed_pos(title_bar_rect.min)
                                            .constrain(false);
                                        
                                        edit_area.show(ui.ctx(), move |ui| {
                                            ui.set_clip_rect(title_bar_rect);
                                            ui.set_width(title_bar_rect.width());
                                            ui.set_height(title_bar_rect.height());
                                            
                                            let mut current_edit_value = name_ref.borrow().clone();
                                            let name_response = ui.text_edit_singleline(&mut current_edit_value);
                                            
                                            // 更新编辑值
                                            *name_ref.borrow_mut() = current_edit_value.clone();
                                            
                                            // 如果失去焦点或按 Enter，完成编辑
                                            if name_response.lost_focus() || ui.input(|i| i.key_pressed(Key::Enter)) {
                                                if let Some(ref orig) = original_name {
                                                    if current_edit_value != *orig && !current_edit_value.is_empty() {
                                                        commands.borrow_mut().push(TrackEditorCommand::RenameClip {
                                                            clip_id: clip_id_for_edit,
                                                            new_name: current_edit_value,
                                                        });
                                                    }
                                                }
                                                *finish_flag.borrow_mut() = true;
                                            }
                                            
                                            // 如果按 Escape，取消编辑
                                            if ui.input(|i| i.key_pressed(Key::Escape)) {
                                                *cancel_flag.borrow_mut() = true;
                                            }
                                        });
                                        
                                        // 在闭包外执行收集的命令
                                        for command in pending_commands.borrow_mut().drain(..) {
                                            self.execute_command(command);
                                        }
                                        
                                        // 在闭包外更新编辑状态
                                        if *should_finish_editing.borrow() {
                                            self.editing_clip_name = None;
                                            self.editing_clip_name_value = None;
                                        }
                                        
                                        if *should_cancel_editing.borrow() {
                                            self.editing_clip_name = None;
                                            self.editing_clip_name_value = None;
                                        } else {
                                            // 更新编辑值
                                            self.editing_clip_name_value = Some(new_name_value.borrow().clone());
                                        }
                                    }
                                }
                                break;
                            }
                        }
                    }
                }

                // 绘制轨道左侧面板（类似钢琴键，最后绘制以覆盖播放头和剪辑）
                let sidebar_rect = Rect::from_min_size(
                    Pos2::new(rect.min.x, rect.min.y + timeline_height),
                    Vec2::new(key_width, rect.height() - timeline_height),
                );
                painter.rect_filled(sidebar_rect, 0.0, ui.visuals().window_fill());

                // 收集需要执行的命令（避免借用冲突）
                let pending_commands: Rc<RefCell<Vec<TrackEditorCommand>>> = Rc::new(RefCell::new(Vec::new()));

                // 为每个轨道面板创建交互式 UI
                for (track_index, track) in self.tracks.iter().enumerate() {
                    let y = rect.min.y + self.track_to_y(track_index, timeline_height);
                    let track_panel_rect = Rect::from_min_size(
                        Pos2::new(rect.min.x, y),
                        Vec2::new(key_width, self.timeline.zoom_y),
                    );
                    
                    // 使用 intersects 检查整个面板矩形是否可见（与剪辑的可见性判断一致）
                    if track_panel_rect.intersects(rect) {
                        let track_id = track.id;
                        let track_muted = track.muted;
                        let track_solo = track.solo;
                        let track_record_arm = track.record_arm;
                        let track_monitor = track.monitor;
                        let track_volume = track.volume;
                        let track_pan = track.pan;
                        let track_name = track.name.clone();
                        let track_inserts = track.inserts.clone();
                        let track_sends = track.sends.clone();
                        let commands = pending_commands.clone();
                        let zoom_y = self.timeline.zoom_y;
                        
                        // 检测右键点击，显示上下文菜单
                        let track_response = ui.allocate_rect(track_panel_rect, egui::Sense::click());
                        if track_response.secondary_clicked() {
                            if let Some(pointer) = ui.input(|i| i.pointer.interact_pos()) {
                                self.track_context_menu_pos = Some(pointer);
                                self.track_context_menu_open_pos = Some(pointer);
                                self.track_context_menu_track_id = Some(track_id);
                            }
                        }
                        
                        // 使用 allocate_ui_at_rect 创建交互式 UI 区域（已弃用，但功能正常）
                        // 裁剪到可见区域，防止面板被滚动出区域后遮挡其他组件
                        let visible_rect = rect.intersect(track_panel_rect);
                        if visible_rect.is_positive() {
                            #[allow(deprecated)]
                            ui.allocate_ui_at_rect(track_panel_rect, |ui| {
                                // 使用可见区域裁剪，而不是整个面板矩形
                                ui.set_clip_rect(visible_rect);
                                
                                // 设置背景颜色
                                let bg_color = if track_solo {
                                    Color32::from_gray(50)
                                } else if track_muted {
                                    Color32::from_gray(40)
                                } else {
                                    Color32::from_gray(35)
                                };
                                ui.painter().rect_filled(track_panel_rect, 0.0, bg_color);
                                ui.painter().rect_stroke(track_panel_rect, 0.0, Stroke::new(1.0, Color32::GRAY));

                                // 垂直布局，从上到下
                                ui.vertical(|ui| {
                                    ui.set_width(key_width);
                                    
                                    // 轨道名称（顶部，可编辑）
                                    ui.horizontal(|ui| {
                                        let mut name_value = track_name.clone();
                                        let name_response = ui.text_edit_singleline(&mut name_value);
                                        if name_response.changed() && name_value != track_name {
                                            commands.borrow_mut().push(TrackEditorCommand::RenameTrack {
                                                track_id,
                                                new_name: name_value,
                                            });
                                        }
                                    });
                                    
                                    // 顶部按钮行
                                    ui.horizontal(|ui| {
                                        ui.set_height(20.0);
                                    
                                    // Mute 按钮
                                    let mute_response = if track_muted {
                                        ui.add_sized(
                                            Vec2::new(TRACK_BUTTON_SIZE, TRACK_BUTTON_SIZE),
                                            egui::Button::new("M")
                                                .fill(Color32::from_rgb(200, 100, 100))
                                        )
                                    } else {
                                        ui.add_sized(
                                            Vec2::new(TRACK_BUTTON_SIZE, TRACK_BUTTON_SIZE),
                                            egui::Button::new("M")
                                        )
                                    };
                                    if mute_response.clicked() {
                                        commands.borrow_mut().push(TrackEditorCommand::SetTrackMute {
                                            track_id,
                                            muted: !track_muted,
                                        });
                                    }

                                    // Solo 按钮
                                    let solo_response = if track_solo {
                                        ui.add_sized(
                                            Vec2::new(TRACK_BUTTON_SIZE, TRACK_BUTTON_SIZE),
                                            egui::Button::new("S")
                                                .fill(Color32::from_rgb(100, 150, 200))
                                        )
                                    } else {
                                        ui.add_sized(
                                            Vec2::new(TRACK_BUTTON_SIZE, TRACK_BUTTON_SIZE),
                                            egui::Button::new("S")
                                        )
                                    };
                                    if solo_response.clicked() {
                                        commands.borrow_mut().push(TrackEditorCommand::SetTrackSolo {
                                            track_id,
                                            solo: !track_solo,
                                        });
                                    }

                                    // Record Arm 按钮
                                    let arm_response = if track_record_arm {
                                        ui.add_sized(
                                            Vec2::new(TRACK_BUTTON_SIZE, TRACK_BUTTON_SIZE),
                                            egui::Button::new("R")
                                                .fill(Color32::from_rgb(255, 50, 50))
                                        )
                                    } else {
                                        ui.add_sized(
                                            Vec2::new(TRACK_BUTTON_SIZE, TRACK_BUTTON_SIZE),
                                            egui::Button::new("R")
                                        )
                                    };
                                    if arm_response.clicked() {
                                        commands.borrow_mut().push(TrackEditorCommand::SetTrackRecordArm {
                                            track_id,
                                            armed: !track_record_arm,
                                        });
                                    }

                                    // Monitor 按钮（使用英文首字母）
                                    let monitor_response = if track_monitor {
                                        ui.add_sized(
                                            Vec2::new(TRACK_MONITOR_BUTTON_WIDTH, TRACK_BUTTON_SIZE),
                                            egui::Button::new("Mon")
                                                .fill(Color32::from_rgb(150, 200, 100))
                                        )
                                    } else {
                                        ui.add_sized(
                                            Vec2::new(TRACK_MONITOR_BUTTON_WIDTH, TRACK_BUTTON_SIZE),
                                            egui::Button::new("Mon")
                                        )
                                    };
                                    if monitor_response.clicked() {
                                        commands.borrow_mut().push(TrackEditorCommand::SetTrackMonitor {
                                            track_id,
                                            monitor: !track_monitor,
                                        });
                                    }
                                    });

                                    // 音量滑块（水平）
                                    ui.horizontal(|ui| {
                                        ui.label("Vol");
                                    let mut volume_value = track_volume;
                                    // 计算 dB 值（0.0-1.0 映射到 -∞ 到 0 dB，简化处理）
                                    let db_value = if volume_value > 0.0 {
                                        (volume_value.ln() * 8.685889638065035).max(-60.0) // 20 * log10(volume)
                                    } else {
                                        -60.0
                                    };
                                    let vol_response = ui.add_sized(
                                        Vec2::new(TRACK_VOLUME_SLIDER_WIDTH, TRACK_CONTROL_SLIDER_HEIGHT),
                                        egui::Slider::new(&mut volume_value, 0.0..=1.0)
                                            .text(format!("{:.0}dB", db_value))
                                    );
                                    if vol_response.changed() {
                                        commands.borrow_mut().push(TrackEditorCommand::SetTrackVolume {
                                            track_id,
                                            volume: volume_value,
                                        });
                                    }
                                    });

                                    // 声像控制（水平滑块）
                                    ui.horizontal(|ui| {
                                        ui.label("Pan");
                                    let mut pan_value = track_pan;
                                    let pan_label = if pan_value < -0.1 {
                                        format!("L{:.0}", pan_value.abs() * 100.0)
                                    } else if pan_value > 0.1 {
                                        format!("R{:.0}", pan_value * 100.0)
                                    } else {
                                        "C".to_string()
                                    };
                                    let pan_response = ui.add_sized(
                                        Vec2::new(TRACK_PAN_SLIDER_WIDTH, TRACK_CONTROL_SLIDER_HEIGHT),
                                        egui::Slider::new(&mut pan_value, -1.0..=1.0)
                                            .text(pan_label)
                                    );
                                    if pan_response.changed() {
                                        commands.borrow_mut().push(TrackEditorCommand::SetTrackPan {
                                            track_id,
                                            pan: pan_value,
                                        });
                                    }
                                    });

                                    // 效果器区域（如果空间足够）
                                    if zoom_y > 100.0 {
                                    // Insert 插槽
                                    if !track_inserts.is_empty() {
                                        ui.collapsing("Ins", |ui| {
                                            for insert in &track_inserts {
                                                ui.label(insert);
                                            }
                                        });
                                    }

                                    // Send 列表
                                    if !track_sends.is_empty() {
                                        ui.collapsing("Send", |ui| {
                                            for (send_name, send_level) in &track_sends {
                                                ui.horizontal(|ui| {
                                                    ui.label(send_name);
                                                    ui.label(format!("{:.1}", send_level));
                                                });
                                            }
                                        });
                                    }
                                    }
                                });
                            });
                        }
                    }
                }
                
                // 显示轨道右键菜单（参考 MIDI 编辑器的实现）
                if let Some(menu_pos) = self.track_context_menu_pos {
                    if let Some(menu_track_id) = self.track_context_menu_track_id {
                        let menu_response = egui::Area::new(egui::Id::new("track_context_menu"))
                            .order(egui::Order::Foreground)
                            .fixed_pos(menu_pos)
                            .show(ui.ctx(), |ui| {
                                egui::Frame::popup(ui.style()).show(ui, |ui| {
                                    ui.set_min_width(150.0);
                                    
                                    if ui.button("Delete Track").clicked() {
                                        pending_commands.borrow_mut().push(TrackEditorCommand::DeleteTrack {
                                            track_id: menu_track_id,
                                        });
                                        self.track_context_menu_pos = None;
                                        self.track_context_menu_open_pos = None;
                                        self.track_context_menu_track_id = None;
                                    }
                                });
                            });
                        
                        // 关闭菜单逻辑（参考 MIDI 编辑器）
                        let ctx = ui.ctx();
                        if ctx.input(|i| i.pointer.primary_clicked() || i.pointer.secondary_clicked()) {
                            if let Some(click_pos) = ctx.input(|i| i.pointer.interact_pos()) {
                                let menu_rect = menu_response.response.rect;
                                
                                // 忽略打开菜单时的点击（相同位置，阈值内）
                                let ignore_click = if let Some(open_pos) = self.track_context_menu_open_pos {
                                    click_pos.distance(open_pos) < TRACK_CONTEXT_MENU_THRESHOLD
                                } else {
                                    false
                                };
                                
                                // 如果点击不在菜单区域内，关闭菜单
                                if !ignore_click && !menu_rect.contains(click_pos) {
                                    self.track_context_menu_pos = None;
                                    self.track_context_menu_open_pos = None;
                                    self.track_context_menu_track_id = None;
                                }
                            } else {
                                // 无法确定点击位置，关闭菜单
                                self.track_context_menu_pos = None;
                                self.track_context_menu_open_pos = None;
                                self.track_context_menu_track_id = None;
                            }
                        }
                    }
                }

                // Add "New Track" button (below the last track)
                let add_track_button_height = 30.0;
                let add_track_button_y = if !self.tracks.is_empty() {
                    let last_track_index = self.tracks.len() - 1;
                    rect.min.y + self.track_to_y(last_track_index, timeline_height) + self.timeline.zoom_y
                } else {
                    rect.min.y + timeline_height
                };
                
                let add_track_button_rect = Rect::from_min_size(
                    Pos2::new(rect.min.x, add_track_button_y),
                    Vec2::new(key_width, add_track_button_height),
                );
                
                if add_track_button_rect.intersects(rect) {
                    let add_commands = pending_commands.clone();
                    #[allow(deprecated)]
                    ui.allocate_ui_at_rect(add_track_button_rect, |ui| {
                        ui.set_clip_rect(add_track_button_rect);
                        
                        // Draw button background
                        ui.painter().rect_filled(add_track_button_rect, 0.0, ui.visuals().window_fill());
                        ui.painter().rect_stroke(add_track_button_rect, 0.0, Stroke::new(1.0, Color32::GRAY));
                        
                        // Add button
                        let add_button = ui.button("+ Add Track");
                        if add_button.clicked() {
                            let track_name = format!("Track {}", self.tracks.len() + 1);
                            add_commands.borrow_mut().push(TrackEditorCommand::CreateTrack {
                                name: track_name,
                            });
                        }
                    });
                }

                // 执行收集的命令（包括按钮添加的命令）
                for command in pending_commands.borrow_mut().drain(..) {
                    self.execute_command(command);
                }
            });
    }


    /// 处理缩放操作（Ctrl/Alt + 滚轮）
    fn handle_zoom(&mut self, ui: &Ui, rect: &Rect, key_width: f32, timeline_height: f32) {
        let scroll_delta = ui.input(|i| i.raw_scroll_delta);
        if scroll_delta == Vec2::ZERO {
            return;
        }

        if ui.input(|i| i.modifiers.ctrl) {
            // 水平缩放
            if scroll_delta.y != 0.0 {
                let old_zoom = self.timeline.zoom_x;
                let new_zoom = (self.timeline.zoom_x
                    * if scroll_delta.y > 0.0 { 1.1 } else { 0.9 })
                .clamp(10.0, 500.0);

                if let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos()) {
                    let rel_x = mouse_pos.x - (rect.min.x + key_width);
                    let beats_at_mouse = (rel_x - self.timeline.manual_scroll_x) / old_zoom;
                    self.timeline.manual_scroll_x = rel_x - beats_at_mouse * new_zoom;
                }

                self.timeline.zoom_x = new_zoom;
                self.clamp_scroll_to_minus_one_beat();
            }
        } else if ui.input(|i| i.modifiers.alt) {
            // 垂直缩放（轨道高度）
            if scroll_delta.y != 0.0 {
                let old_zoom = self.timeline.zoom_y;
                let new_zoom = (self.timeline.zoom_y
                    * if scroll_delta.y > 0.0 { 1.1 } else { 0.9 })
                .clamp(20.0, 200.0);

                if let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos()) {
                    let rel_y = mouse_pos.y - (rect.min.y + timeline_height);
                    let value_at_mouse = (rel_y - self.timeline.manual_scroll_y) / old_zoom;
                    self.timeline.manual_scroll_y = rel_y - value_at_mouse * new_zoom;
                }

                self.timeline.zoom_y = new_zoom;
            }
        }
    }

    /// 处理中键拖拽平移
    fn handle_panning(&mut self, ui: &Ui) {
        if ui.input(|i| i.pointer.middle_down()) {
            if self.is_panning {
                if let Some(start) = self.pan_start_pos {
                    if let Some(curr) = ui.input(|i| i.pointer.hover_pos()) {
                        let delta = curr - start;
                        self.timeline.manual_scroll_x += delta.x;
                        self.timeline.manual_scroll_y += delta.y;
                        self.clamp_scroll_to_minus_one_beat();
                        self.pan_start_pos = Some(curr);
                        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
                    }
                }
            } else {
                self.is_panning = true;
                self.pan_start_pos = ui.input(|i| i.pointer.hover_pos());
            }
        } else {
            self.is_panning = false;
            self.pan_start_pos = None;
        }
    }

    /// 限制垂直滚动
    fn clamp_vertical_scroll(&mut self, rect: &Rect, timeline_height: f32) {
        let total_content_height = self.tracks.len() as f32 * self.timeline.zoom_y;
        let view_height = rect.height() - timeline_height;
        // 允许最后一个轨道滚动到视图的1/2处，而不是底部
        let min_scroll_y = if total_content_height > view_height {
            // 最后一个轨道的位置
            let last_track_y = (self.tracks.len() - 1) as f32 * self.timeline.zoom_y;
            // 允许滚动到最后一个轨道在视图中间位置
            view_height / 2.0 - last_track_y
        } else {
            0.0
        };
        self.timeline.manual_scroll_y = self.timeline.manual_scroll_y.clamp(min_scroll_y, 0.0);
    }

    /// 限制滚动，确保最多只能看到 -0.25 拍的位置
    fn clamp_scroll_to_minus_one_beat(&mut self) {
        let visible_earliest_beat = self.timeline.scroll_x - (self.timeline.manual_scroll_x / self.timeline.zoom_x) as f64;
        if visible_earliest_beat < -0.25 {
            // 限制到 -0.25 拍：manual_scroll_x = (scroll_x + 0.25) * zoom_x
            self.timeline.manual_scroll_x = ((self.timeline.scroll_x + 0.25) * self.timeline.zoom_x as f64) as f32;
        }
    }


    fn handle_clip_click(&mut self, clip_id: ClipId, modifiers: Modifiers, hit_region: clip::ClipHitRegion) {
        match hit_region {
            clip::ClipHitRegion::Body => {
                if modifiers.ctrl || modifiers.command {
                    // Toggle selection
                    if self.selected_clips.contains(&clip_id) {
                        self.selected_clips.remove(&clip_id);
                    } else {
                        self.selected_clips.insert(clip_id);
                    }
                } else if modifiers.shift {
                    // Extend selection (simplified - just add to selection)
                    self.selected_clips.insert(clip_id);
                } else {
                    // Single select
                    self.selected_clips.clear();
                    self.selected_clips.insert(clip_id);
                }
                self.emit_event(TrackEditorEvent::ClipSelected { clip_id });
            }
            _ => {
                // Edge clicks don't change selection
            }
        }
    }



    // Command implementations
    fn create_clip(&mut self, track_id: TrackId, start: f64, duration: f64, clip_type: ClipType) {
        if let Some(track) = self.tracks.iter_mut().find(|t| t.id == track_id) {
            // 限制：不允许将剪辑创建到小于 0 的位置
            let clamped_start = start.max(0.0);
            let name = match &clip_type {
                ClipType::Midi { .. } => "MIDI Clip".to_string(),
                ClipType::Audio { .. } => "Audio Clip".to_string(),
            };
            let clip = match clip_type {
                ClipType::Midi { .. } => Clip::new_midi(track_id, clamped_start, duration, name),
                ClipType::Audio { .. } => Clip::new_audio(track_id, clamped_start, duration, name),
            };
            track.clips.push(clip);
        }
    }

    fn delete_clip(&mut self, clip_id: ClipId) {
        for track in &mut self.tracks {
            if let Some(pos) = track.clips.iter().position(|c| c.id == clip_id) {
                track.clips.remove(pos);
                self.selected_clips.remove(&clip_id);
                return;
            }
        }
    }

    fn move_clip(&mut self, clip_id: ClipId, new_track_id: TrackId, new_start: f64, disable_snap: bool) {
        // Find and remove clip from old track
        let mut clip = None;
        for track in &mut self.tracks {
            if let Some(pos) = track.clips.iter().position(|c| c.id == clip_id) {
                clip = Some(track.clips.remove(pos));
                break;
            }
        }

        if let Some(mut clip) = clip {
            // Update clip position
            // 限制：不允许将剪辑移动到小于 0 的位置
            let clamped_start = new_start.max(0.0);
            clip.track_id = new_track_id;
            // 根据 disable_snap 决定是否吸附
            clip.start_time = if disable_snap {
                clamped_start
            } else {
                self.timeline.snap_time(clamped_start)
            };

            // Add to new track
            if let Some(track) = self.tracks.iter_mut().find(|t| t.id == new_track_id) {
                track.clips.push(clip);
            }
        }
    }

    fn resize_clip(&mut self, clip_id: ClipId, new_duration: f64, resize_from_start: bool, disable_snap: bool) {
        for track in &mut self.tracks {
            if let Some(clip) = track.clips.iter_mut().find(|c| c.id == clip_id) {
                // 根据 disable_snap 决定是否吸附
                let snapped_duration = if disable_snap {
                    new_duration
                } else {
                    self.timeline.snap_time(new_duration)
                }.max(0.1);
                
                if resize_from_start {
                    let old_start = clip.start_time;
                    let calculated_start = old_start + clip.duration - snapped_duration;
                    // 根据 disable_snap 决定是否吸附开始位置
                    let new_start = if disable_snap {
                        calculated_start
                    } else {
                        self.timeline.snap_time(calculated_start)
                    };
                    // 限制：不允许将剪辑调整到小于 0 的位置
                    clip.start_time = new_start.max(0.0);
                }
                clip.duration = snapped_duration;
                self.emit_event(TrackEditorEvent::ClipResized {
                    clip_id,
                    new_duration: snapped_duration,
                });
                return;
            }
        }
    }

    fn split_clip(&mut self, clip_id: ClipId, split_time: f64) {
        for track in &mut self.tracks {
            if let Some(pos) = track.clips.iter().position(|c| c.id == clip_id) {
                let clip = &track.clips[pos];
                let relative_split = split_time - clip.start_time;
                
                if relative_split > 0.1 && relative_split < clip.duration - 0.1 {
                    let mut new_clip = clip.clone();
                    new_clip.id = ClipId::next();
                    new_clip.start_time = split_time;
                    new_clip.duration = clip.duration - relative_split;
                    
                    track.clips[pos].duration = relative_split;
                    track.clips.insert(pos + 1, new_clip);
                }
                return;
            }
        }
    }

    fn rename_clip(&mut self, clip_id: ClipId, new_name: String) {
        for track in &mut self.tracks {
            if let Some(clip) = track.clips.iter_mut().find(|c| c.id == clip_id) {
                clip.name = new_name.clone();
                self.emit_event(TrackEditorEvent::ClipRenamed {
                    clip_id,
                    new_name,
                });
                return;
            }
        }
    }

    fn create_track(&mut self, name: String) {
        let track = Track::new(name);
        let track_id = track.id;
        self.tracks.push(track);
        self.emit_event(TrackEditorEvent::TrackCreated { track_id });
    }

    fn delete_track(&mut self, track_id: TrackId) {
        if let Some(pos) = self.tracks.iter().position(|t| t.id == track_id) {
            self.tracks.remove(pos);
            self.emit_event(TrackEditorEvent::TrackDeleted { track_id });
        }
    }

    fn rename_track(&mut self, track_id: TrackId, new_name: String) {
        if let Some(track) = self.tracks.iter_mut().find(|t| t.id == track_id) {
            track.name = new_name;
        }
    }

    fn emit_event(&mut self, event: TrackEditorEvent) {
        if let Some(ref mut listener) = self.event_listener {
            listener(&event);
        }
        self.pending_events.push(event);
    }

    // Public getters

    /// 获取所有轨道的只读引用
    ///
    /// # 返回
    ///
    /// 轨道列表的切片
    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    /// 获取时间轴状态的只读引用
    ///
    /// # 返回
    ///
    /// 时间轴状态，包含缩放、滚动位置、播放头等信息
    pub fn timeline(&self) -> &TimelineState {
        &self.timeline
    }

    /// 获取当前选中的剪辑 ID 集合
    ///
    /// # 返回
    ///
    /// 选中剪辑 ID 的有序集合
    pub fn selected_clips(&self) -> &BTreeSet<ClipId> {
        &self.selected_clips
    }
}

//! UI 模块
//!
//! 包含音轨编辑器的主要 UI 组件，包括时间轴、轨道和剪辑的渲染和交互。

mod timeline;
mod track_lane;
mod clip;
mod renderer;
mod toolbar;
mod statusbar;

use crate::editor::{TrackEditorCommand, TrackEditorEvent};
use crate::structure::{Track, Clip, TrackId, ClipId, TimelineState, ClipType};
use egui::*;
use std::collections::BTreeSet;

#[derive(Clone)]
pub struct TrackEditorOptions {
    pub default_track_height: f32,
    pub min_clip_width: f32,
    pub track_header_width: f32,
    pub timeline_height: f32,
}

impl Default for TrackEditorOptions {
    fn default() -> Self {
        Self {
            default_track_height: 80.0,
            min_clip_width: 20.0,
            track_header_width: 200.0,
            timeline_height: 60.0,
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
    selection_box_start: Option<Pos2>,
    selection_box_end: Option<Pos2>,
    pan_start_pos: Option<Pos2>,
    pan_start_scroll_x: Option<f64>,
    pan_start_scroll_y: Option<f32>,
    
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
    #[allow(dead_code)]
    CreateClip,
    SelectBox,
    #[allow(dead_code)]
    Pan,
}

impl TrackEditor {
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
            selection_box_start: None,
            selection_box_end: None,
            pan_start_pos: None,
            pan_start_scroll_x: None,
            pan_start_scroll_y: None,
            metronome_enabled: false,
            is_playing: false,
            last_update: 0.0,
            pending_events: Vec::new(),
            event_listener: None,
        }
    }

    pub fn set_event_listener(&mut self, listener: Box<dyn FnMut(&TrackEditorEvent)>) {
        self.event_listener = Some(listener);
    }

    pub fn take_events(&mut self) -> Vec<TrackEditorEvent> {
        std::mem::take(&mut self.pending_events)
    }

    pub fn execute_command(&mut self, command: TrackEditorCommand) {
        match command {
            TrackEditorCommand::CreateClip { track_id, start, duration, clip_type } => {
                self.create_clip(track_id, start, duration, clip_type);
            }
            TrackEditorCommand::DeleteClip { clip_id } => {
                self.delete_clip(clip_id);
            }
            TrackEditorCommand::MoveClip { clip_id, new_track_id, new_start } => {
                self.move_clip(clip_id, new_track_id, new_start);
            }
            TrackEditorCommand::ResizeClip { clip_id, new_duration, resize_from_start } => {
                self.resize_clip(clip_id, new_duration, resize_from_start);
            }
            TrackEditorCommand::SplitClip { clip_id, split_time } => {
                self.split_clip(clip_id, split_time);
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
            
            // Timeline below toolbar
            let mut timeline_ui = timeline::Timeline::new(&self.timeline, self.options.track_header_width);
            timeline_ui.ui(ui, self.options.timeline_height);
            
            // Handle timeline interactions (click to set playhead)
            // 使用基于 tick 的坐标系统（与 MIDI 编辑器一致）
            if let Some(pointer_pos) = ui.input(|i| i.pointer.hover_pos()) {
                if ui.rect_contains_pointer(Rect::from_min_size(
                    ui.cursor().left_top(),
                    Vec2::new(ui.available_width(), self.options.timeline_height),
                )) {
                    if ui.input(|i| i.pointer.primary_clicked()) {
                        let clicked_tick = self.timeline.x_to_tick(pointer_pos.x, self.options.track_header_width);
                        let clicked_time = self.timeline.tick_to_time(clicked_tick);
                        self.execute_command(TrackEditorCommand::SetPlayhead {
                            position: clicked_time.max(0.0),
                        });
                    }
                }
            }

            ui.separator();

            // Tracks area with scroll
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let tracks_len = self.tracks.len();
                    for i in 0..tracks_len {
                        self.ui_track(ui, i);
                        ui.add_space(2.0);
                    }
                });

            // Middle mouse button panning will be handled in handle_interactions
        });

        self.handle_interactions(ui);
    }

    fn ui_track(&mut self, ui: &mut Ui, track_index: usize) {
        if track_index >= self.tracks.len() {
            return;
        }
        
        // Collect data before entering closure
        let track_height = self.tracks[track_index].height;
        let track_clips: Vec<_> = self.tracks[track_index].clips.iter().cloned().collect();
        let timeline = self.timeline.clone();
        let options = self.options.clone();
        let selected_clips = self.selected_clips.clone();
        let selection_box_start = self.selection_box_start;
        let selection_box_end = self.selection_box_end;
        
        let mut track_header = track_lane::TrackLaneHeader::new(&self.tracks[track_index], options.track_header_width);
        ui.horizontal(|ui| {
            // Track header
            track_header.ui(ui);

            // Track content area
            let track_content_size = ui.available_size();
            let track_content_response = ui.allocate_response(
                Vec2::new(track_content_size.x, track_height),
                Sense::click_and_drag(),
            );

            // Use painter_at to restrict drawing to track content area only
            // This ensures clips don't draw over the track header panel
            let track_content_rect = track_content_response.rect;
            let painter = ui.painter_at(track_content_rect);
            
            // Draw track background
            painter.rect_filled(
                track_content_rect,
                0.0,
                Color32::from_gray(30),
            );

            // Draw grid lines in track content area (using unified grid system)
            // This ensures grid lines align perfectly with timeline grid
            renderer::draw_unified_grid(
                &painter,
                track_content_rect.min.y,
                track_content_rect.max.y,
                &timeline,
                options.track_header_width,
                track_content_rect.min.x,
                track_content_rect.max.x,
            );

            // Draw clips (with viewport culling for performance)
            // 使用基于节拍的可见性检查（与 MIDI 编辑器一致）
            let visible_beats_start = (-timeline.manual_scroll_x / timeline.zoom_x).floor();
            let visible_beats_end = visible_beats_start + (track_content_rect.width() / timeline.zoom_x) + 2.0;
            let tpb = timeline.ticks_per_beat.max(1) as u64;
            let visible_start_tick = (visible_beats_start * tpb as f32).floor() as u64;
            let visible_end_tick = (visible_beats_end * tpb as f32).ceil() as u64;
            
            // Filter clips that are potentially visible
            let visible_clips: Vec<_> = track_clips.iter()
                .filter(|clip| {
                    let clip_start_tick = timeline.time_to_tick(clip.start_time);
                    let clip_end_tick = timeline.time_to_tick(clip.start_time + clip.duration);
                    // Clip is visible if it overlaps with visible tick range
                    clip_end_tick >= visible_start_tick && clip_start_tick <= visible_end_tick
                })
                .collect();
            
            for clip in visible_clips {
                let is_selected = selected_clips.contains(&clip.id);
                let mut renderer = clip::ClipRenderer::new(clip, &timeline, &options, options.track_header_width);
                renderer.set_selected(is_selected);
                // Calculate clip position using tick-based coordinates
                let clip_start_tick = timeline.time_to_tick(clip.start_time);
                let clip_x = timeline.tick_to_x(clip_start_tick, options.track_header_width);
                let duration_tick = timeline.time_to_tick(clip.start_time + clip.duration) - clip_start_tick;
                let clip_width = ((duration_tick as f32 / tpb as f32) * timeline.zoom_x).max(options.min_clip_width);
                // Only render if clip overlaps with track content area (not in header)
                if clip_x + clip_width > track_content_rect.min.x && clip_x < track_content_rect.max.x {
                    // Clip is visible in track content area, render it
                    renderer.render(&painter, track_content_rect.min.y);
                }
            }

            // Draw playhead in track content area (参考 MIDI 编辑器的实现)
            // 使用与时间轴相同的坐标转换，确保播放头位置一致
            let playhead_tick = timeline.time_to_tick(timeline.playhead_position);
            let playhead_x = timeline.tick_to_x(playhead_tick, options.track_header_width);
            if playhead_x >= track_content_rect.min.x && playhead_x <= track_content_rect.max.x {
                // 使用与 MIDI 编辑器一致的样式：半透明蓝色，宽度 2.0
                painter.line_segment(
                    [
                        Pos2::new(playhead_x, track_content_rect.min.y),
                        Pos2::new(playhead_x, track_content_rect.max.y),
                    ],
                    Stroke::new(2.0, Color32::from_rgba_premultiplied(100, 200, 255, 128)),
                );
            }

            // Draw selection box (using global painter since it might span multiple tracks)
            if let (Some(start), Some(end)) = (selection_box_start, selection_box_end) {
                let box_rect = Rect::from_two_pos(start, end);
                if box_rect.intersects(track_content_rect) {
                    // Use the restricted painter to ensure selection box doesn't draw over header
                    let selection_painter = ui.painter_at(track_content_rect);
                    crate::ui::renderer::draw_selection_box(&selection_painter, box_rect);
                }
            }

            // Handle track content interactions
            self.handle_track_content_interaction(ui, track_index, &track_content_response);
        });
    }

    fn handle_track_content_interaction(
        &mut self,
        ui: &mut Ui,
        track_index: usize,
        response: &Response,
    ) {
        if track_index >= self.tracks.len() {
            return;
        }
        
        // Collect data before processing interactions
        let track_clips: Vec<_> = self.tracks[track_index].clips.iter().cloned().collect();
        let timeline = self.timeline.clone();
        let options = self.options.clone();
        let track_y = response.rect.min.y;
        let pointer_pos = response.interact_pointer_pos();
        
        // Calculate visible time range for viewport culling (使用基于节拍的检查)
        let track_content_rect = response.rect;
        let visible_beats_start = (-timeline.manual_scroll_x / timeline.zoom_x).floor();
        let visible_beats_end = visible_beats_start + (track_content_rect.width() / timeline.zoom_x) + 2.0;
        let tpb = timeline.ticks_per_beat.max(1) as u64;
        let visible_start_tick = (visible_beats_start * tpb as f32).floor() as u64;
        let visible_end_tick = (visible_beats_end * tpb as f32).ceil() as u64;
        
        // Filter clips that are potentially visible (with some margin for partial visibility)
        let visible_clips: Vec<_> = track_clips.iter()
            .filter(|clip| {
                let clip_start_tick = timeline.time_to_tick(clip.start_time);
                let clip_end_tick = timeline.time_to_tick(clip.start_time + clip.duration);
                // Clip is visible if it overlaps with visible tick range
                clip_end_tick >= visible_start_tick && clip_start_tick <= visible_end_tick
            })
            .collect();
        
        // Collect events to process after borrow ends
        let mut click_events: Vec<(ClipId, Modifiers, clip::ClipHitRegion)> = Vec::new();
        let mut double_click_events: Vec<ClipId> = Vec::new();
        let mut drag_start_events: Vec<(ClipId, Pos2, f64, DragAction)> = Vec::new();
        let mut cursor_icon: Option<CursorIcon> = None;

        // Handle clip interactions (only for visible clips)
        for clip in visible_clips {
            let renderer = clip::ClipRenderer::new(clip, &timeline, &options, options.track_header_width);
            if let Some(pos) = pointer_pos {
                // Only test hit if pointer is within track content area
                if track_content_rect.contains(pos) {
                    if let Some(hit_region) = renderer.hit_test(pos, track_y) {
                        // Update cursor based on hit region
                        cursor_icon = Some(match hit_region {
                            clip::ClipHitRegion::LeftEdge | clip::ClipHitRegion::RightEdge => {
                                CursorIcon::ResizeHorizontal
                            }
                            clip::ClipHitRegion::Body => CursorIcon::Grab,
                        });

                        // Collect click events
                        if response.clicked_by(PointerButton::Primary) {
                            let modifiers = ui.input(|i| i.modifiers);
                            click_events.push((clip.id, modifiers, hit_region));
                        }

                        // Collect double click events
                        if response.double_clicked_by(PointerButton::Primary) {
                            double_click_events.push(clip.id);
                        }

                        // Collect drag start events
                        if response.drag_started_by(PointerButton::Primary) {
                            let drag_action = match hit_region {
                                clip::ClipHitRegion::LeftEdge => DragAction::ResizeClipStart,
                                clip::ClipHitRegion::RightEdge => DragAction::ResizeClipEnd,
                                clip::ClipHitRegion::Body => DragAction::MoveClip,
                            };
                            drag_start_events.push((clip.id, pos, clip.start_time, drag_action));
                        }
                    }
                }
            }
        }

        // Apply cursor icon
        if let Some(icon) = cursor_icon {
            ui.output_mut(|o| o.cursor_icon = icon);
        }

        // Process click events
        for (clip_id, modifiers, hit_region) in click_events {
            self.handle_clip_click(clip_id, modifiers, hit_region);
        }

        // Process double click events
        for clip_id in double_click_events {
            self.emit_event(TrackEditorEvent::ClipDoubleClicked { clip_id });
        }

        // Process drag start events
        for (clip_id, pos, start_time, drag_action) in drag_start_events {
            self.drag_action = drag_action;
            self.drag_clip_id = Some(clip_id);
            self.drag_start_pos = Some(pos);
            self.drag_start_time = Some(start_time);
        }

        // Handle empty area clicks (deselect or create clip)
        if response.clicked_by(PointerButton::Primary) {
            if pointer_pos.is_some() && self.drag_clip_id.is_none() {
                let modifiers = ui.input(|i| i.modifiers);
                if !modifiers.ctrl && !modifiers.shift {
                    self.selected_clips.clear();
                }
            }
        }

        // Handle drag to create selection box
        if response.drag_started_by(PointerButton::Primary) && self.drag_clip_id.is_none() {
            if let Some(pos) = pointer_pos {
                self.drag_action = DragAction::SelectBox;
                self.selection_box_start = Some(pos);
                self.selection_box_end = Some(pos);
            }
        }
    }

    /// 限制滚动，确保最多只能看到 -0.25 拍的位置
    fn clamp_scroll_to_minus_one_beat(&mut self) {
        let visible_earliest_beat = self.timeline.scroll_x - (self.timeline.manual_scroll_x / self.timeline.zoom_x) as f64;
        if visible_earliest_beat < -0.25 {
            // 限制到 -0.25 拍：manual_scroll_x = (scroll_x + 0.25) * zoom_x
            self.timeline.manual_scroll_x = ((self.timeline.scroll_x + 0.25) * self.timeline.zoom_x as f64) as f32;
        }
    }

    fn handle_interactions(&mut self, ui: &mut Ui) {
        // Handle middle mouse button panning (使用 manual_scroll_x/y，与 MIDI 编辑器一致)
        if ui.input(|i| i.pointer.middle_down()) {
            if let Some(pointer_pos) = ui.input(|i| i.pointer.hover_pos()) {
                // Start panning if not already panning
                if self.pan_start_pos.is_none() {
                    self.pan_start_pos = Some(pointer_pos);
                    self.pan_start_scroll_x = Some(self.timeline.manual_scroll_x as f64);
                    self.pan_start_scroll_y = Some(self.timeline.manual_scroll_y);
                    self.drag_action = DragAction::Pan;
                    // Set cursor to indicate panning
                    ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
                } else if let (Some(start_pos), Some(start_scroll_x), Some(start_scroll_y)) = 
                    (self.pan_start_pos, self.pan_start_scroll_x, self.pan_start_scroll_y) {
                    // Continue panning
                    let delta = pointer_pos - start_pos;
                    // Horizontal pan: 直接更新 manual_scroll_x（像素单位）
                    let new_manual_scroll_x = start_scroll_x as f32 + delta.x;
                    
                    // 限制：最多只能看到 -1 拍的位置
                    self.timeline.manual_scroll_x = new_manual_scroll_x;
                    self.clamp_scroll_to_minus_one_beat();
                    
                    // Vertical pan: 直接更新 manual_scroll_y（像素单位）
                    self.timeline.manual_scroll_y = start_scroll_y + delta.y;
                    // Keep cursor as grabbing during pan
                    ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
                }
            }
        } else {
            // End panning when middle button is released
            if self.drag_action == DragAction::Pan {
                self.pan_start_pos = None;
                self.pan_start_scroll_x = None;
                self.pan_start_scroll_y = None;
                self.drag_action = DragAction::None;
            }
        }

        // Handle ongoing drags (for clips and selection)
        if self.drag_action != DragAction::None && self.drag_action != DragAction::Pan {
            if let Some(pointer_pos) = ui.input(|i| i.pointer.hover_pos()) {
                match self.drag_action {
                    DragAction::MoveClip => {
                        if let Some(clip_id) = self.drag_clip_id {
                            self.handle_clip_drag(ui, clip_id, pointer_pos);
                        }
                    }
                    DragAction::ResizeClipStart | DragAction::ResizeClipEnd => {
                        if let Some(clip_id) = self.drag_clip_id {
                            self.handle_clip_resize(ui, clip_id, pointer_pos);
                        }
                    }
                    DragAction::SelectBox => {
                        self.selection_box_end = Some(pointer_pos);
                        self.update_selection_from_box();
                    }
                    _ => {}
                }
            }

            // Check if drag ended
            if !ui.input(|i| i.pointer.primary_down()) {
                self.drag_action = DragAction::None;
                self.drag_clip_id = None;
                self.drag_start_pos = None;
                self.drag_start_time = None;
                self.selection_box_start = None;
                self.selection_box_end = None;
            }
        }

        // Handle keyboard shortcuts
        ui.input(|i| {
            if i.key_pressed(Key::A) && i.modifiers.ctrl {
                // Select all clips
                self.selected_clips.clear();
                for track in &self.tracks {
                    for clip in &track.clips {
                        self.selected_clips.insert(clip.id);
                    }
                }
            }

            if (i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace)) && !self.selected_clips.is_empty() {
                let clips_to_delete: Vec<ClipId> = self.selected_clips.iter().copied().collect();
                for clip_id in clips_to_delete {
                    self.execute_command(TrackEditorCommand::DeleteClip { clip_id });
                }
            }

            // Zoom with mouse wheel + Ctrl
            if i.modifiers.ctrl {
                let scroll_delta = i.raw_scroll_delta.y;
                if scroll_delta != 0.0 {
                    let zoom_factor = 1.0 + scroll_delta * 0.001;
                    self.timeline.zoom_x = (self.timeline.zoom_x * zoom_factor).max(10.0).min(1000.0);
                    // 缩放后，确保不会看到小于 -1 拍的位置
                    self.clamp_scroll_to_minus_one_beat();
                }
            }

            // Mouse wheel horizontal scroll is now handled by middle button drag
            // Vertical scroll is handled by ScrollArea widget for track list
        });
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

    fn handle_clip_drag(&mut self, _ui: &mut Ui, clip_id: ClipId, pointer_pos: Pos2) {
        if let Some(_start_pos) = self.drag_start_pos {
            if let Some(start_time) = self.drag_start_time {
                // 使用基于 tick 的坐标转换（与 MIDI 编辑器一致）
                let base_x = self.options.track_header_width;
                let rel_x = pointer_pos.x - base_x - self.timeline.manual_scroll_x;
                let beats = rel_x / self.timeline.zoom_x;
                let tpb = self.timeline.ticks_per_beat.max(1) as f32;
                let pointer_tick = (beats * tpb).round() as i64;

                // 计算原始剪辑的 tick
                let original_start_tick = self.timeline.time_to_tick(start_time) as i64;
                
                // 计算偏移量（tick）
                let delta_tick = pointer_tick - original_start_tick;
                let new_start_tick = (original_start_tick + delta_tick).max(0) as u64;

                // 对齐到网格（与 MIDI 编辑器一致）
                let disable_snap = false; // 可以从 UI 输入获取（如 Alt 键）
                let snapped_tick = self.timeline.snap_tick(new_start_tick, disable_snap);
                let snapped_time = self.timeline.tick_to_time(snapped_tick);

                // 限制：不允许将剪辑拖动到小于 0 的位置
                let clamped_time = snapped_time.max(0.0);

                // Find the clip and its current track
                let mut clip_track_id = None;
                for track in &self.tracks {
                    if track.clips.iter().any(|c| c.id == clip_id) {
                        clip_track_id = Some(track.id);
                        break;
                    }
                }

                if let Some(track_id) = clip_track_id {
                    self.execute_command(TrackEditorCommand::MoveClip {
                        clip_id,
                        new_track_id: track_id,
                        new_start: clamped_time,
                    });
                }
            }
        }
    }

    fn handle_clip_resize(&mut self, _ui: &mut Ui, clip_id: ClipId, pointer_pos: Pos2) {
        if let Some(_start_pos) = self.drag_start_pos {
            if let Some(_start_time) = self.drag_start_time {
                // 使用基于 tick 的坐标转换（与 MIDI 编辑器一致）
                let base_x = self.options.track_header_width;
                let rel_x = pointer_pos.x - base_x - self.timeline.manual_scroll_x;
                let beats = rel_x / self.timeline.zoom_x;
                let tpb = self.timeline.ticks_per_beat.max(1) as f32;
                let pointer_tick = (beats * tpb).round() as i64;

                // Find the clip
                for track in &self.tracks {
                    if let Some(clip) = track.clips.iter().find(|c| c.id == clip_id) {
                        let resize_from_start = self.drag_action == DragAction::ResizeClipStart;
                        
                        // 计算剪辑的 tick 范围
                        let clip_start_tick = self.timeline.time_to_tick(clip.start_time) as i64;
                        let clip_end_tick = self.timeline.time_to_tick(clip.start_time + clip.duration) as i64;
                        
                        let (new_start_tick, new_end_tick) = if resize_from_start {
                            // 从开始调整：移动开始位置
                            let new_start = pointer_tick.max(0);
                            (new_start, clip_end_tick)
                        } else {
                            // 从结束调整：移动结束位置
                            (clip_start_tick, pointer_tick.max(clip_start_tick + 1))
                        };
                        
                        // 对齐到网格
                        let disable_snap = false;
                        let snapped_start = self.timeline.snap_tick(new_start_tick as u64, disable_snap) as i64;
                        let snapped_end = self.timeline.snap_tick(new_end_tick as u64, disable_snap) as i64;
                        
                        if resize_from_start {
                            // 调整开始时间
                            let new_start_time = self.timeline.tick_to_time(snapped_start as u64);
                            // 限制：不允许将剪辑拖动到小于 0 的位置
                            let clamped_start_time = new_start_time.max(0.0);
                            let new_duration = self.timeline.tick_to_time(snapped_end as u64) - clamped_start_time;
                            
                            // 需要同时更新开始时间和持续时间
                            // 这里简化处理，只更新持续时间，开始时间在 move_clip 中处理
                            // 但我们需要确保开始时间 >= 0
                            if clamped_start_time >= 0.0 {
                                self.execute_command(TrackEditorCommand::ResizeClip {
                                    clip_id,
                                    new_duration: new_duration.max(0.01),
                                    resize_from_start: true,
                                });
                                // 如果开始时间被限制，需要移动剪辑
                                if clamped_start_time != new_start_time {
                                    // 找到轨道 ID
                                    let mut clip_track_id = None;
                                    for track in &self.tracks {
                                        if track.clips.iter().any(|c| c.id == clip_id) {
                                            clip_track_id = Some(track.id);
                                            break;
                                        }
                                    }
                                    if let Some(track_id) = clip_track_id {
                                        self.execute_command(TrackEditorCommand::MoveClip {
                                            clip_id,
                                            new_track_id: track_id,
                                            new_start: clamped_start_time,
                                        });
                                    }
                                }
                            }
                        } else {
                            // 调整结束时间（持续时间）
                            let new_duration = self.timeline.tick_to_time(snapped_end as u64) - clip.start_time;
                            
                            self.execute_command(TrackEditorCommand::ResizeClip {
                                clip_id,
                                new_duration: new_duration.max(0.01),
                                resize_from_start: false,
                            });
                        }
                        break;
                    }
                }
            }
        }
    }

    fn update_selection_from_box(&mut self) {
        if let (Some(start), Some(end)) = (self.selection_box_start, self.selection_box_end) {
            let box_rect = Rect::from_two_pos(start, end);
            
            // Clear selection if not holding Ctrl
            // (This would need to track initial modifiers, simplified for now)
            
            // Select clips that intersect with selection box
            for track in &self.tracks {
                for clip in &track.clips {
                    // 使用基于 tick 的坐标计算（与 MIDI 编辑器一致）
                    let clip_start_tick = self.timeline.time_to_tick(clip.start_time);
                    let clip_end_tick = self.timeline.time_to_tick(clip.start_time + clip.duration);
                    let clip_center_tick = clip_start_tick + (clip_end_tick - clip_start_tick) / 2;
                    let clip_x = self.timeline.tick_to_x(clip_center_tick, self.options.track_header_width);
                    let clip_y = track.height / 2.0; // Approximate
                    let clip_pos = Pos2::new(clip_x, clip_y);
                    
                    if box_rect.contains(clip_pos) {
                        self.selected_clips.insert(clip.id);
                    }
                }
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

    fn move_clip(&mut self, clip_id: ClipId, new_track_id: TrackId, new_start: f64) {
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
            clip.start_time = self.timeline.snap_time(clamped_start);

            // Add to new track
            if let Some(track) = self.tracks.iter_mut().find(|t| t.id == new_track_id) {
                track.clips.push(clip);
            }
        }
    }

    fn resize_clip(&mut self, clip_id: ClipId, new_duration: f64, resize_from_start: bool) {
        for track in &mut self.tracks {
            if let Some(clip) = track.clips.iter_mut().find(|c| c.id == clip_id) {
                let snapped_duration = self.timeline.snap_time(new_duration).max(0.1);
                if resize_from_start {
                    let old_start = clip.start_time;
                    let new_start = self.timeline.snap_time(old_start + clip.duration - snapped_duration);
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
    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    pub fn timeline(&self) -> &TimelineState {
        &self.timeline
    }

    pub fn selected_clips(&self) -> &BTreeSet<ClipId> {
        &self.selected_clips
    }
}

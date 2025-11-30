//! UI 模块
//!
//! 包含音轨编辑器的主要 UI 组件，包括时间轴、轨道和剪辑的渲染和交互。

mod timeline;
mod track_lane;
mod clip;
mod renderer;

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
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        let available_size = ui.available_size();
        ui.set_min_size(available_size);

        ui.vertical(|ui| {
            // Timeline
            let mut timeline_ui = timeline::Timeline::new(&self.timeline, self.options.track_header_width);
            timeline_ui.ui(ui, self.options.timeline_height);
            
            // Handle timeline interactions (click to set playhead)
            if let Some(pointer_pos) = ui.input(|i| i.pointer.hover_pos()) {
                if ui.rect_contains_pointer(Rect::from_min_size(
                    ui.cursor().left_top(),
                    Vec2::new(ui.available_width(), self.options.timeline_height),
                )) {
                    if ui.input(|i| i.pointer.primary_clicked()) {
                        let clicked_x = pointer_pos.x - self.options.track_header_width;
                        let clicked_time = self.timeline.x_to_time(clicked_x);
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

            // Handle horizontal scrolling
            if let Some(pointer_pos) = ui.input(|i| i.pointer.hover_pos()) {
                let scroll_area_size = ui.available_size();
                let scroll_area_rect = Rect::from_min_size(ui.cursor().left_top(), scroll_area_size);
                if scroll_area_rect.contains(pointer_pos) {
                    if ui.input(|i| i.pointer.middle_down()) {
                        // Pan with middle mouse button
                        let delta = ui.input(|i| i.pointer.delta());
                        let pan_amount = -delta.x as f64 / self.timeline.zoom_x as f64;
                        self.timeline.scroll_x = (self.timeline.scroll_x + pan_amount).max(0.0);
                    }
                }
            }
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

            // Draw track background
            let painter = ui.painter();
            painter.rect_filled(
                track_content_response.rect,
                0.0,
                Color32::from_gray(30),
            );

            // Draw grid first (behind clips)
            crate::ui::renderer::draw_track_grid(
                painter,
                track_content_response.rect,
                &timeline,
                options.track_header_width,
            );

            // Draw clips
            for clip in &track_clips {
                let is_selected = selected_clips.contains(&clip.id);
                let mut renderer = clip::ClipRenderer::new(clip, &timeline, &options);
                renderer.set_selected(is_selected);
                renderer.render(painter, track_content_response.rect.min.y, options.track_header_width);
            }

            // Draw selection box
            if let (Some(start), Some(end)) = (selection_box_start, selection_box_end) {
                let box_rect = Rect::from_two_pos(start, end);
                if box_rect.intersects(track_content_response.rect) {
                    crate::ui::renderer::draw_selection_box(painter, box_rect);
                }
            }

            // Handle track content interactions
            self.handle_track_content_interaction(ui, track_index, &track_content_response);
            
            // Update selection from box for this track
            if let (Some(start), Some(end)) = (selection_box_start, selection_box_end) {
                let box_rect = Rect::from_two_pos(start, end);
                if box_rect.intersects(track_content_response.rect) {
                    for clip in &track_clips {
                        let clip_x = timeline.time_to_x(clip.start_time) + options.track_header_width;
                        let clip_width = (clip.duration * timeline.zoom_x as f64).max(options.min_clip_width as f64) as f32;
                        let clip_y = track_content_response.rect.min.y + 10.0;
                        let clip_height = 60.0;
                        
                        let clip_rect = Rect::from_min_size(
                            Pos2::new(clip_x, clip_y),
                            Vec2::new(clip_width, clip_height),
                        );
                        
                        if clip_rect.intersects(box_rect) {
                            self.selected_clips.insert(clip.id);
                        }
                    }
                }
            }
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
        
        // Collect events to process after borrow ends
        let mut click_events: Vec<(ClipId, Modifiers, clip::ClipHitRegion)> = Vec::new();
        let mut double_click_events: Vec<ClipId> = Vec::new();
        let mut drag_start_events: Vec<(ClipId, Pos2, f64, DragAction)> = Vec::new();
        let mut cursor_icon: Option<CursorIcon> = None;

        // Handle clip interactions
        for clip in &track_clips {
            let renderer = clip::ClipRenderer::new(clip, &timeline, &options);
            if let Some(pos) = pointer_pos {
                if let Some(hit_region) = renderer.hit_test(pos, track_y, options.track_header_width) {
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

    fn handle_interactions(&mut self, ui: &mut Ui) {
        // Handle ongoing drags
        if self.drag_action != DragAction::None {
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
                        // Selection box update will be handled per-track in ui_track
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
                }
            }

            // Pan with mouse wheel (horizontal scroll)
            if !i.modifiers.ctrl {
                let scroll_delta = i.raw_scroll_delta.x;
                if scroll_delta != 0.0 {
                    let pan_amount = scroll_delta as f64 / self.timeline.zoom_x as f64;
                    self.timeline.scroll_x = (self.timeline.scroll_x - pan_amount).max(0.0);
                }
            }
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
        if let Some(start_pos) = self.drag_start_pos {
            if let Some(start_time) = self.drag_start_time {
                // 计算相对于轨道内容区域的 delta（减去 header_width）
                let delta_x = pointer_pos.x - start_pos.x;
                let delta_time = delta_x as f64 / self.timeline.zoom_x as f64;
                let new_time = start_time + delta_time;

                // Find the clip and its current track
                let mut clip_track_id = None;
                for track in &self.tracks {
                    if track.clips.iter().any(|c| c.id == clip_id) {
                        clip_track_id = Some(track.id);
                        break;
                    }
                }

                if let Some(track_id) = clip_track_id {
                    // Apply snapping if enabled
                    let snapped_time = if self.timeline.snap_enabled {
                        self.timeline.snap_time(new_time).max(0.0)
                    } else {
                        new_time.max(0.0)
                    };
                    
                    self.execute_command(TrackEditorCommand::MoveClip {
                        clip_id,
                        new_track_id: track_id,
                        new_start: snapped_time,
                    });
                }
            }
        }
    }

    fn handle_clip_resize(&mut self, _ui: &mut Ui, clip_id: ClipId, pointer_pos: Pos2) {
        if let Some(start_pos) = self.drag_start_pos {
            if let Some(_start_time) = self.drag_start_time {
                // 计算相对于轨道内容区域的 delta
                let delta_x = pointer_pos.x - start_pos.x;
                let delta_time = delta_x as f64 / self.timeline.zoom_x as f64;

                // Find the clip
                for track in &self.tracks {
                    if let Some(clip) = track.clips.iter().find(|c| c.id == clip_id) {
                        let resize_from_start = self.drag_action == DragAction::ResizeClipStart;
                        let mut new_duration = if resize_from_start {
                            clip.duration - delta_time
                        } else {
                            clip.duration + delta_time
                        };
                        
                        // Apply snapping to duration
                        if self.timeline.snap_enabled {
                            new_duration = self.timeline.snap_time(new_duration);
                        }
                        
                        // Ensure minimum duration
                        new_duration = new_duration.max(0.1);

                        self.execute_command(TrackEditorCommand::ResizeClip {
                            clip_id,
                            new_duration,
                            resize_from_start,
                        });
                        break;
                    }
                }
            }
        }
    }

    fn update_selection_from_box(&mut self) {
        // Selection box update is now handled per-track in ui_track method
        // This method is kept for compatibility but does nothing
    }

    // Command implementations
    fn create_clip(&mut self, track_id: TrackId, start: f64, duration: f64, clip_type: ClipType) {
        if let Some(track) = self.tracks.iter_mut().find(|t| t.id == track_id) {
            let name = match &clip_type {
                ClipType::Midi { .. } => "MIDI Clip".to_string(),
                ClipType::Audio { .. } => "Audio Clip".to_string(),
            };
            let clip = match clip_type {
                ClipType::Midi { .. } => Clip::new_midi(track_id, start, duration, name),
                ClipType::Audio { .. } => Clip::new_audio(track_id, start, duration, name),
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
            clip.track_id = new_track_id;
            clip.start_time = self.timeline.snap_time(new_start);

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
                    clip.start_time = self.timeline.snap_time(old_start + clip.duration - snapped_duration);
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

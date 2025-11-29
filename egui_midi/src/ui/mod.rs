use crate::audio::{PlaybackBackend, PlaybackObserver};
use crate::editor::{EditorCommand, EditorEvent, MidiEditorOptions, SnapMode, TransportState};
use crate::structure::{BatchTransformType, CurveLaneId, CurvePointId, CurveLaneType, MidiState, Note, NoteId};
use egui::*;
use midly::Smf;
use std::collections::BTreeSet;
use std::sync::Arc;

type PlaybackHandle = Arc<dyn PlaybackBackend>;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DragAction {
    None,
    Move,
    ResizeStart,
    ResizeEnd,
    Create,
    LoopEdit,
    PlayheadSeek,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LoopEditMode {
    Start,
    End,
    Move,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum LaneType {
    Pitch,
    Velocity,
}

#[allow(dead_code)]
struct LaneEditState {
    lane: LaneType,
    anchor: NoteId,
    originals: Vec<(NoteId, Note)>,
}

pub struct MidiEditor {
    pub state: MidiState,
    pub playback: Option<PlaybackHandle>,
    pub playback_observer: Option<Arc<dyn PlaybackObserver>>,

    // View state
    pub zoom_x: f32,
    pub zoom_y: f32,
    pub manual_scroll_x: f32, // Manual scroll offset
    pub manual_scroll_y: f32,

    // Playback state
    pub is_playing: bool,
    pub current_time: f32, // in seconds
    pub last_update: f64,
    pub last_tick: u64, // For sequencer tracking

    // Interaction state
    pub selected_notes: BTreeSet<NoteId>,
    pub selection_box_start: Option<Pos2>,
    pub selection_box_end: Option<Pos2>,
    pub drag_start_pos: Option<Pos2>,
    pub is_dragging_note: bool,
    pub is_resizing_note: bool, // True if resizing (dragging right edge)
    pub active_key_note: Option<u8>, // Track which key is currently being pressed for preview
    pub is_panning: bool,
    pub pan_start: Option<Pos2>,
    pub pan_start_scroll: Option<Vec2>,
    pub drag_action: DragAction,
    pub drag_preview_key: Option<u8>,
    pub drag_original_start: Option<u64>,
    pub drag_original_duration: Option<u64>,
    pub drag_original_key: Option<u8>,
    pub drag_pointer_offset_ticks: Option<i64>,
    pub drag_original_notes: Vec<(NoteId, Note)>,
    pub drag_primary_anchor: Option<NoteId>,
    pub drag_original_loop_start: Option<u64>,
    pub drag_original_loop_end: Option<u64>,
    loop_edit_mode: Option<LoopEditMode>,

    // Config
    pub snap_interval: u64, // Ticks (e.g., 480 for quarter note)
    pub snap_mode: SnapMode,
    pub swing_ratio: f32,
    pub volume: f32,
    pub preview_pitch_shift: f32,
    pub loop_enabled: bool,
    pub loop_start_tick: u64,
    pub loop_end_tick: u64,

    // Integration
    pub transport_override: Option<TransportState>,
    pub pending_events: Vec<EditorEvent>,
    event_listener: Option<Box<dyn FnMut(&EditorEvent)>>,
    pub clipboard: Vec<Note>,
    pub undo_stack: Vec<MidiState>,
    pub redo_stack: Vec<MidiState>,
    pub drag_changed_note: bool,
    #[allow(dead_code)]
    lane_edit_state: Option<LaneEditState>,
    #[allow(dead_code)]
    lane_edit_changed: bool,
    
    // Curve editing state
    pub selected_curve_lane: Option<CurveLaneId>,
    pub dragging_curve_point: Option<(CurveLaneId, CurvePointId)>,
    pub curve_lane_height: f32,
    pub curve_lane_visible: bool,
    pub dragging_splitter: bool,
    
    // Batch transform dialog state
    pub show_batch_transform_dialog: bool,
    pub batch_transform_type: crate::structure::BatchTransformType,
    pub batch_transform_value: f64,
    pub swing_menu_ratio: f32,
    pub swing_original_notes: Vec<(NoteId, u64)>, // Store original positions when starting swing adjustment
    
    // Context menu state
    pub context_menu_pos: Option<Pos2>,
    pub context_menu_open_pos: Option<Pos2>, // Track the position where menu was opened
    pub splitter_ratio: f32, // Ratio of piano roll height (0.0-1.0)
    
    // Playback settings dialog
    pub show_playback_settings: bool,
    
    // Shortcut configuration
    pub enable_space_playback: bool,
}

impl MidiEditor {
    pub fn new(playback: Option<PlaybackHandle>) -> Self {
        Self::with_state_and_options(MidiState::default(), playback, MidiEditorOptions::default())
    }

    pub fn with_state(state: MidiState, playback: Option<PlaybackHandle>) -> Self {
        Self::with_state_and_options(state, playback, MidiEditorOptions::default())
    }

    pub fn with_state_and_options(
        state: MidiState,
        playback: Option<PlaybackHandle>,
        options: MidiEditorOptions,
    ) -> Self {
        let mut editor = Self::base_with_state(state, playback);
        editor.apply_options(&options);
        editor
    }

    fn base_with_state(state: MidiState, playback: Option<PlaybackHandle>) -> Self {
        let loop_default = (state.ticks_per_beat as u64)
            .saturating_mul(4)
            .max(state.ticks_per_beat as u64);
        Self {
            state,
            playback,
            playback_observer: None,
            zoom_x: 100.0,
            zoom_y: 20.0,
            manual_scroll_x: 0.0,
            manual_scroll_y: 0.0,
            is_playing: false,
            current_time: 0.0,
            last_update: 0.0,
            last_tick: 0,
            selected_notes: BTreeSet::new(),
            selection_box_start: None,
            selection_box_end: None,
            drag_start_pos: None,
            is_dragging_note: false,
            is_resizing_note: false,
            active_key_note: None,
            is_panning: false,
            pan_start: None,
            pan_start_scroll: None,
            drag_action: DragAction::None,
            drag_preview_key: None,
            drag_original_start: None,
            drag_original_duration: None,
            drag_original_key: None,
            drag_pointer_offset_ticks: None,
            drag_original_notes: Vec::new(),
            drag_primary_anchor: None,
            drag_original_loop_start: None,
            drag_original_loop_end: None,
            loop_edit_mode: None,
            snap_interval: 120,
            snap_mode: SnapMode::Absolute,
            swing_ratio: 0.0,
            volume: 0.5,
            preview_pitch_shift: 0.0,
            loop_enabled: false,
            loop_start_tick: 0,
            loop_end_tick: loop_default,
            transport_override: None,
            pending_events: Vec::new(),
            event_listener: None,
            clipboard: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            drag_changed_note: false,
            lane_edit_state: None,
            lane_edit_changed: false,
            selected_curve_lane: None,
            dragging_curve_point: None,
            curve_lane_height: 120.0,
            curve_lane_visible: true,
            dragging_splitter: false,
            splitter_ratio: 0.7, // 70% for piano roll, 30% for curve editor
            show_batch_transform_dialog: false,
            batch_transform_type: BatchTransformType::VelocityOffset,
            batch_transform_value: 0.0,
            swing_menu_ratio: 0.0,
            swing_original_notes: Vec::new(),
            context_menu_pos: None,
            context_menu_open_pos: None,
            show_playback_settings: false,
            enable_space_playback: true, // Default enabled
        }
    }

    pub fn apply_options(&mut self, options: &MidiEditorOptions) {
        self.zoom_x = options.zoom_x;
        self.zoom_y = options.zoom_y;
        self.manual_scroll_x = options.manual_scroll_x;
        self.manual_scroll_y = options.manual_scroll_y;
        self.snap_interval = options.snap_interval.max(1);
        self.snap_mode = options.snap_mode;
        // TODO: Implement swing rhythm feature
        self.swing_ratio = options.swing_ratio.clamp(0.0, 2.0);
        self.volume = options.volume.clamp(0.0, 1.0);
        self.preview_pitch_shift = options.preview_pitch_shift.clamp(-24.0, 24.0);
        self.loop_enabled = options.loop_enabled;
        self.loop_start_tick = options.loop_start_tick;
        self.loop_end_tick = options.loop_end_tick.max(self.loop_start_tick + 1);
        if let Some(playback) = &self.playback {
            playback.set_volume(self.volume * 2.0);
            playback.set_pitch_shift(self.preview_pitch_shift);
        }
        if let Some(key) = options.center_on_key {
            self.center_on_key(key);
        }
        self.enable_space_playback = options.enable_space_playback;
    }

    pub fn set_event_listener<F>(&mut self, listener: F)
    where
        F: FnMut(&EditorEvent) + 'static,
    {
        self.event_listener = Some(Box::new(listener));
    }

    pub fn replace_state(&mut self, state: MidiState) {
        self.state = state;
        self.selected_notes.clear();
        self.emit_state_replaced();
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    pub fn snapshot_state(&self) -> MidiState {
        self.state.clone()
    }

    pub fn midi_state(&self) -> &MidiState {
        &self.state
    }

    pub fn edit_state<F: FnOnce(&mut MidiState)>(&mut self, f: F) {
        self.push_undo_snapshot();
        f(&mut self.state);
        self.emit_state_replaced();
    }

    pub fn set_bpm(&mut self, bpm: f32) {
        let clamped = bpm.clamp(20.0, 400.0);
        if (self.state.bpm - clamped).abs() > f32::EPSILON {
            self.push_undo_snapshot();
            self.state.bpm = clamped;
            self.pending_events
                .push(EditorEvent::StateReplaced(self.state.clone()));
        }
    }

    pub fn set_time_signature(&mut self, numer: u8, denom: u8) {
        let numer = numer.max(1);
        let denom = denom.max(1);
        if self.state.time_signature != (numer, denom) {
            self.push_undo_snapshot();
            self.state.time_signature = (numer, denom);
            self.pending_events
                .push(EditorEvent::StateReplaced(self.state.clone()));
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        let normalized = volume.clamp(0.0, 1.0);
        if (self.volume - normalized).abs() > f32::EPSILON {
            self.volume = normalized;
            if let Some(playback) = &self.playback {
                playback.set_volume(self.volume * 2.0);
            }
        }
    }

    pub fn set_snap_interval(&mut self, tick_span: u64) {
        if tick_span != 0 {
            self.snap_interval = tick_span;
        }
    }

    pub fn load_from_smf(&mut self, smf: &Smf) {
        self.replace_state(MidiState::from_smf(smf));
    }

    pub fn export_smf(&self) -> Smf<'static> {
        self.state.to_smf()
    }

    pub fn insert_note(&mut self, note: Note) -> NoteId {
        self.push_undo_snapshot();
        self.state.notes.push(note);
        self.sort_notes();
        self.emit_note_added(note);
        note.id
    }

    pub fn remove_notes(&mut self, ids: impl IntoIterator<Item = NoteId>) {
        use std::collections::HashSet;
        let targets: HashSet<_> = ids.into_iter().collect();
        if targets.is_empty() {
            return;
        }
        self.push_undo_snapshot();
        let mut removed = Vec::new();
        self.state.notes.retain(|note| {
            if targets.contains(&note.id) {
                removed.push(*note);
                false
            } else {
                true
            }
        });
        for note in removed {
            self.emit_note_deleted(note);
            self.selected_notes.remove(&note.id);
        }
    }

    pub fn clear(&mut self) {
        if self.state.notes.is_empty() {
            return;
        }
        self.push_undo_snapshot();
        // Collect notes to emit events before clearing to avoid unnecessary clone
        let notes_to_delete: Vec<Note> = self.state.notes.iter().copied().collect();
        for note in notes_to_delete {
            self.emit_note_deleted(note);
        }
        self.state.notes.clear();
        self.selected_notes.clear();
    }

    pub fn undo(&mut self) -> bool {
        if let Some(previous) = self.undo_stack.pop() {
            self.redo_stack.push(self.state.clone());
            self.state = previous;
            self.emit_state_replaced();
            self.selected_notes.clear();
            return true;
        }
        false
    }

    pub fn redo(&mut self) -> bool {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.state.clone());
            self.state = next;
            self.emit_state_replaced();
            self.selected_notes.clear();
            return true;
        }
        false
    }

    pub fn set_playback_backend(&mut self, backend: Option<PlaybackHandle>) {
        self.playback = backend;
        if let Some(playback) = &self.playback {
            playback.set_volume(self.volume * 2.0);
            playback.set_pitch_shift(self.preview_pitch_shift);
        }
    }

    pub fn set_playback_observer(&mut self, observer: Option<Arc<dyn PlaybackObserver>>) {
        self.playback_observer = observer;
    }

    pub fn take_events(&mut self) -> Vec<EditorEvent> {
        std::mem::take(&mut self.pending_events)
    }

    pub fn set_transport_state(&mut self, state: Option<TransportState>) {
        self.transport_override = state;
    }

    pub fn center_on_c4(&mut self) {
        self.center_on_key(60);
    }

    pub fn center_on_key(&mut self, key: u8) {
        let key = key.min(127);
        let position_from_top = (127 - key) as f32 * self.zoom_y;
        let approximate_view_half = 300.0;
        let desired_offset = position_from_top - approximate_view_half;
        self.manual_scroll_y = -desired_offset.max(0.0);
    }

    fn seek_to_seconds(&mut self, seconds: f32) {
        let seconds = seconds.max(0.0);
        self.current_time = seconds;
        if self.state.ticks_per_beat > 0 {
            let seconds_per_beat = 60.0 / self.state.bpm.max(1.0);
            let seconds_per_tick = seconds_per_beat / self.state.ticks_per_beat as f32;
            self.last_tick = (self.current_time / seconds_per_tick) as u64;
        } else {
            self.last_tick = 0;
        }
        self.emit_transport_event();
    }

    fn stop_playback_backend(&mut self) {
        if let Some(playback) = &self.playback {
            playback.all_notes_off();
        }
    }

    fn emit_event(&mut self, event: EditorEvent) {
        if let Some(listener) = &mut self.event_listener {
            listener(&event);
        }
        self.pending_events.push(event);
    }

    fn emit_state_replaced(&mut self) {
        self.emit_event(EditorEvent::StateReplaced(self.state.clone()));
    }

    fn emit_transport_event(&mut self) {
        let loop_progress = if self.loop_enabled && self.loop_end_tick > self.loop_start_tick {
            let loop_duration = self.loop_end_tick - self.loop_start_tick;
            if loop_duration > 0 {
                let position_in_loop = if self.last_tick >= self.loop_start_tick {
                    self.last_tick - self.loop_start_tick
                } else {
                    0
                };
                (position_in_loop as f32 / loop_duration as f32).clamp(0.0, 1.0)
            } else {
                0.0
            }
        } else {
            0.0
        };
        
        self.emit_event(EditorEvent::TransportChanged {
            current_time: self.current_time,
            current_tick: self.last_tick,
            loop_enabled: self.loop_enabled,
            loop_start_tick: self.loop_start_tick,
            loop_end_tick: self.loop_end_tick,
            loop_progress,
        });
    }

    pub fn apply_command(&mut self, command: EditorCommand) {
        match command {
            EditorCommand::ReplaceState(state) => self.replace_state(state),
            EditorCommand::SetNotes(notes) => {
                self.edit_state(|state| {
                    state.notes = notes;
                    state.notes.sort_by_key(|n| n.start);
                });
            }
            EditorCommand::AppendNotes(mut notes) => {
                if notes.is_empty() {
                    return;
                }
                self.edit_state(|state| {
                    state.notes.append(&mut notes);
                    state.notes.sort_by_key(|n| n.start);
                });
            }
            EditorCommand::ClearNotes => self.clear(),
            EditorCommand::SeekSeconds(seconds) => {
                self.seek_to_seconds(seconds);
            }
            EditorCommand::SetPlayback(is_playing) => {
                if self.is_playing != is_playing {
                    self.is_playing = is_playing;
                    if !self.is_playing {
                        self.stop_playback_backend();
                        self.notify_playback_stopped();
                    } else {
                        self.notify_playback_started();
                    }
                    self.emit_event(EditorEvent::PlaybackStateChanged {
                        is_playing: self.is_playing,
                    });
                }
            }
            EditorCommand::CenterOnKey(key) => self.center_on_key(key),
            EditorCommand::SetBpm(bpm) => self.set_bpm(bpm),
            EditorCommand::SetTimeSignature(numer, denom) => self.set_time_signature(numer, denom),
            EditorCommand::SetVolume(volume) => self.set_volume(volume),
            EditorCommand::SetLoop {
                enabled,
                start_tick,
                end_tick,
            } => {
                self.loop_enabled = enabled;
                self.loop_start_tick = start_tick;
                self.loop_end_tick = end_tick.max(start_tick + 1);
            }
            EditorCommand::SetSnap { interval, mode } => {
                self.snap_interval = interval.max(1);
                self.snap_mode = mode;
            }
            EditorCommand::OverrideTransport(state) => {
                self.set_transport_state(state);
            }
            EditorCommand::AddCurvePoint { lane_id, tick, value } => {
                self.push_undo_snapshot();
                if let Some(lane) = self.state.curves.iter_mut().find(|c| c.id == lane_id) {
                    let point = lane.insert_point(tick, value);
                    self.emit_event(EditorEvent::CurvePointAdded {
                        lane_id,
                        point_id: point.id,
                    });
                }
            }
            EditorCommand::UpdateCurvePoint {
                lane_id,
                point_id,
                tick,
                value,
            } => {
                self.push_undo_snapshot();
                if let Some(lane) = self.state.curves.iter_mut().find(|c| c.id == lane_id) {
                    if lane.update_point(point_id, tick, value).is_some() {
                        self.emit_event(EditorEvent::CurvePointUpdated {
                            lane_id,
                            point_id,
                        });
                    }
                }
            }
            EditorCommand::RemoveCurvePoint { lane_id, point_id } => {
                self.push_undo_snapshot();
                if let Some(lane) = self.state.curves.iter_mut().find(|c| c.id == lane_id) {
                    if lane.remove_point(point_id).is_some() {
                        self.emit_event(EditorEvent::CurvePointRemoved {
                            lane_id,
                            point_id,
                        });
                    }
                }
            }
            EditorCommand::ToggleCurveLaneEnabled { lane_id } => {
                self.push_undo_snapshot();
                if let Some(lane) = self.state.curves.iter_mut().find(|c| c.id == lane_id) {
                    lane.enabled = !lane.enabled;
                }
            }
            EditorCommand::HumanizeNotes {
                time_range,
                velocity_range,
            } => {
                if !self.selected_notes.is_empty() {
                    self.push_undo_snapshot();
                    let note_ids: Vec<NoteId> = self.selected_notes.iter().copied().collect();
                    self.state.humanize_notes(&note_ids, time_range, velocity_range);
                    self.emit_state_replaced();
                }
            }
            EditorCommand::BatchTransform {
                transform_type,
                value,
            } => {
                if !self.selected_notes.is_empty() {
                    self.push_undo_snapshot();
                    let note_ids: Vec<NoteId> = self.selected_notes.iter().copied().collect();
                    self.state.batch_transform_notes(&note_ids, transform_type, value);
                    self.emit_state_replaced();
                }
            }
        }
    }

    fn preview_note_on(&mut self, key: u8, velocity: u8) {
        if let Some(playback) = &self.playback {
            if let Some(prev) = self.drag_preview_key.take() {
                playback.note_off(prev);
            }
            playback.note_on(key, velocity);
            self.drag_preview_key = Some(key);
        }
    }

    fn preview_note_off(&mut self) {
        if let Some(prev) = self.drag_preview_key.take() {
            if let Some(playback) = &self.playback {
                playback.note_off(prev);
            }
        }
    }

    fn emit_note_added(&mut self, note: Note) {
        self.emit_event(EditorEvent::NoteAdded(note));
    }

    fn emit_note_deleted(&mut self, note: Note) {
        self.emit_event(EditorEvent::NoteDeleted(note));
    }

    fn emit_note_updated(&mut self, before: Note, after: Note) {
        if before != after {
            self.emit_event(EditorEvent::NoteUpdated { before, after });
        }
    }

    fn finalize_note_drag_if_needed(&mut self) {
        if self.drag_changed_note {
            let originals = self.drag_original_notes.clone();
            for (id, before) in originals {
                if let Some(idx) = self.note_index_by_id(id) {
                    let after = self.state.notes[idx];
                    self.emit_note_updated(before, after);
                }
            }
        }
        self.drag_original_notes.clear();
        self.drag_primary_anchor = None;
        self.drag_changed_note = false;
    }

    #[allow(dead_code)]
    fn finalize_lane_edit(&mut self) {
        if let Some(state) = self.lane_edit_state.take() {
            if self.lane_edit_changed {
                for (id, before) in state.originals {
                    if let Some(idx) = self.note_index_by_id(id) {
                        let after = self.state.notes[idx];
                        if before != after {
                            self.emit_note_updated(before, after);
                        }
                    }
                }
            }
        }
        self.lane_edit_changed = false;
    }

    fn notify_selection_changed(&mut self, previous: BTreeSet<NoteId>) {
        if previous != self.selected_notes {
            self.emit_event(EditorEvent::SelectionChanged(
                self.selected_notes.iter().copied().collect(),
            ));
        }
    }

    fn set_single_selection(&mut self, note_id: NoteId) {
        let prev = self.selected_notes.clone();
        self.selected_notes.clear();
        self.selected_notes.insert(note_id);
        self.notify_selection_changed(prev);
    }

    fn toggle_selection(&mut self, note_id: NoteId) {
        let prev = self.selected_notes.clone();
        if !self.selected_notes.insert(note_id) {
            self.selected_notes.remove(&note_id);
        }
        self.notify_selection_changed(prev);
    }

    fn extend_selection(&mut self, note_id: NoteId) {
        if self.selected_notes.contains(&note_id) {
            return;
        }
        let prev = self.selected_notes.clone();
        self.selected_notes.insert(note_id);
        self.notify_selection_changed(prev);
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        let total_height = ui.available_height();
        ui.set_min_height(total_height);

        ui.horizontal(|ui| {
            ui.set_min_height(total_height);
            ui.vertical(|ui| {
                ui.set_min_height(total_height);
                self.ui_toolbar(ui);
                ui.separator();
                
                // Allocate space for piano roll and curve lanes with draggable splitter
                // Account for bottom status bar (typically 25-30 pixels)
                let status_bar_height = 25.0;
                let available_height_raw = ui.available_height();
                let remaining_height = (available_height_raw - status_bar_height).max(0.0);
                let min_piano_height = 200.0;
                let min_curve_height = 80.0;
                
                // Calculate heights based on splitter ratio
                let piano_roll_height = if self.curve_lane_visible {
                    (remaining_height * self.splitter_ratio).max(min_piano_height).min(remaining_height)
                } else {
                    remaining_height
                };
                let curve_height = if self.curve_lane_visible {
                    (remaining_height - piano_roll_height).max(0.0)
                } else {
                    0.0
                };
                
                // Piano roll area
                let piano_rect = ui.allocate_ui_with_layout(
                    Vec2::new(ui.available_width(), piano_roll_height),
                    Layout::top_down(Align::LEFT),
                    |ui| {
                        self.ui_piano_roll(ui);
                    }
                ).response.rect;
                
                // Draggable splitter (only if curve editor is visible)
                if self.curve_lane_visible {
                    let splitter_height = 4.0;
                    let splitter_rect = Rect::from_min_size(
                        Pos2::new(piano_rect.min.x, piano_rect.max.y),
                        Vec2::new(piano_rect.width(), splitter_height)
                    );
                    
                    let splitter_response = ui.allocate_rect(splitter_rect, Sense::click_and_drag());
                    let painter = ui.painter_at(splitter_rect);
                    
                    // Draw splitter
                    painter.rect_filled(splitter_rect, 0.0, Color32::from_rgb(100, 100, 100));
                    painter.rect_stroke(splitter_rect, 0.0, Stroke::new(1.0, Color32::from_rgb(150, 150, 150)));
                    
                    // Handle dragging
                    if splitter_response.drag_started() {
                        self.dragging_splitter = true;
                    }
                    if self.dragging_splitter {
                        if let Some(pointer) = ui.input(|i| i.pointer.hover_pos()) {
                            let new_ratio = ((pointer.y - piano_rect.min.y) / remaining_height)
                                .clamp(min_piano_height / remaining_height, 1.0 - min_curve_height / remaining_height);
                            self.splitter_ratio = new_ratio;
                        }
                        if ui.input(|i| i.pointer.any_released()) {
                            self.dragging_splitter = false;
                        }
                        ui.ctx().set_cursor_icon(CursorIcon::ResizeVertical);
                    } else if splitter_response.hovered() {
                        ui.ctx().set_cursor_icon(CursorIcon::ResizeVertical);
                    }
                    
                    // Curve lanes area
                    ui.allocate_ui_with_layout(
                        Vec2::new(ui.available_width(), curve_height),
                        Layout::top_down(Align::LEFT),
                        |ui| {
                            self.ui_curve_lanes(ui);
                        }
                    );
                }
            });
            ui.separator();
            self.ui_inspector(ui, total_height);
        });

        // Handle playback logic (only if Space key is enabled)
        if self.enable_space_playback && ui.input(|i| i.key_pressed(Key::Space)) {
            self.is_playing = !self.is_playing;
            if self.is_playing {
                self.last_update = ui.input(|i| i.time);
                let seconds_per_beat = 60.0 / self.state.bpm;
                let seconds_per_tick = seconds_per_beat / self.state.ticks_per_beat as f32;
                self.last_tick = (self.current_time / seconds_per_tick) as u64;
                self.notify_playback_started();
            } else {
                self.stop_playback_backend();
                self.notify_playback_stopped();
            }
            self.emit_event(EditorEvent::PlaybackStateChanged {
                is_playing: self.is_playing,
            });
        }

        if self.is_playing {
            ui.ctx().request_repaint();
            let now = ui.input(|i| i.time);
            let dt = now - self.last_update;
            self.last_update = now;

            if dt > 0.0 && dt < 1.0 {
                // Avoid large jumps
                self.current_time += dt as f32;
                self.update_sequencer();
            }
        } else {
            self.last_update = ui.input(|i| i.time);
            // Update last_tick to match current_time so when we start playing we don't skip or retrigger weirdly
            // But if we scrub, we might want to silence notes.
        }

        self.handle_shortcuts(ui.ctx());
        
        // Context menu for piano roll
        if let Some(menu_pos) = self.context_menu_pos {
            let menu_response = egui::Area::new(egui::Id::new("piano_roll_context_menu"))
                .order(egui::Order::Foreground)
                .fixed_pos(menu_pos)
                .show(ui.ctx(), |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        let has_selection = !self.selected_notes.is_empty();
                        
                        // Set minimum width for all buttons to ensure consistent width
                        ui.set_min_width(200.0);
                        
                        // Quantize to snap grid
                        if ui.add_enabled(has_selection && self.snap_interval > 0, egui::Button::new("Quantize to snap grid")
                            .min_size(egui::Vec2::new(200.0, 0.0))).clicked() {
                            self.swing_original_notes.clear();
                            self.swing_menu_ratio = 0.0;
                            self.quantize_selected_notes();
                            self.context_menu_pos = None;
                            self.context_menu_open_pos = None;
                        }
                        
                        ui.separator();
                        
                        // Snap Mode and Snap Interval buttons side by side
                        ui.horizontal(|ui| {
                            // Snap Mode submenu (adaptive width)
                            ui.menu_button("Snap Mode", |ui| {
                                if ui.selectable_label(self.snap_mode == SnapMode::Absolute, "Absolute").clicked() {
                                    self.apply_command(EditorCommand::SetSnap {
                                        interval: self.snap_interval,
                                        mode: SnapMode::Absolute,
                                    });
                                    self.context_menu_pos = None;
                                    self.context_menu_open_pos = None;
                                }
                                if ui.selectable_label(self.snap_mode == SnapMode::Relative, "Relative").clicked() {
                                    self.apply_command(EditorCommand::SetSnap {
                                        interval: self.snap_interval,
                                        mode: SnapMode::Relative,
                                    });
                                    self.context_menu_pos = None;
                                    self.context_menu_open_pos = None;
                                }
                            });
                            
                            // Snap Interval submenu (adaptive width)
                            ui.menu_button("Snap Interval", |ui| {
                                let intervals = vec![
                                    (480 * 4, "1/1"),
                                    (480 * 2, "1/2"),
                                    (480, "1/4"),
                                    (240, "1/8"),
                                    (120, "1/16"),
                                    (0, "Free"),
                                ];
                                
                                for (interval, label) in intervals {
                                    let is_selected = self.snap_interval == interval;
                                    if ui.selectable_label(is_selected, label).clicked() {
                                        self.swing_original_notes.clear();
                                        self.swing_menu_ratio = 0.0;
                                        self.apply_command(EditorCommand::SetSnap {
                                            interval: interval.max(1),
                                            mode: self.snap_mode,
                                        });
                                        self.context_menu_pos = None;
                                        self.context_menu_open_pos = None;
                                    }
                                }
                            });
                        });
                        
                        ui.separator();
                        
                        // Humanize
                        if ui.add_enabled(has_selection, egui::Button::new("Humanize")
                            .min_size(egui::Vec2::new(200.0, 0.0))).clicked() {
                            self.swing_original_notes.clear();
                            self.swing_menu_ratio = 0.0;
                            let time_range = (self.snap_interval / 12).max(1).min(20);
                            let velocity_range = 5;
                            self.apply_command(EditorCommand::HumanizeNotes {
                                time_range,
                                velocity_range,
                            });
                            self.context_menu_pos = None;
                            self.context_menu_open_pos = None;
                        }
                        
                        // Batch Transform
                        if ui.add_enabled(has_selection, egui::Button::new("Batch Transform...")
                            .min_size(egui::Vec2::new(200.0, 0.0))).clicked() {
                            self.swing_original_notes.clear();
                            self.swing_menu_ratio = 0.0;
                            self.show_batch_transform_dialog = true;
                            self.context_menu_pos = None;
                            self.context_menu_open_pos = None;
                        }
                        
                        ui.separator();
                        
                        // Swing - directly in menu
                        if has_selection {
                            ui.label("Swing:");
                            // Check if selection changed - if so, reinitialize
                            let current_selection: Vec<NoteId> = self.selected_notes.iter().copied().collect();
                            let selection_changed = self.swing_original_notes.is_empty() 
                                || self.swing_original_notes.len() != current_selection.len()
                                || !self.swing_original_notes.iter().all(|(id, _)| current_selection.contains(id));
                            
                            if selection_changed {
                                // Restore original positions if we had previous swing applied
                                if !self.swing_original_notes.is_empty() && self.swing_menu_ratio > 0.0 {
                                    let original_notes = self.swing_original_notes.clone();
                                    for (id, original_start) in &original_notes {
                                        if let Some(note) = self.note_mut_by_id(*id) {
                                            note.start = *original_start;
                                        }
                                    }
                                }
                                
                                // Initialize with current selection
                                self.swing_original_notes = self.selected_notes
                                    .iter()
                                    .filter_map(|&id| {
                                        self.note_by_id(id).map(|note| (id, note.start))
                                    })
                                    .collect();
                                self.swing_menu_ratio = 0.0;
                                // Create undo snapshot when starting swing adjustment
                                self.push_undo_snapshot();
                            }
                            
                            ui.horizontal(|ui| {
                                let mut swing = self.swing_menu_ratio;
                                
                                // Slider for 0-100% range
                                let slider_response = ui.add(
                                    Slider::new(&mut swing, 0.0..=1.0)
                                        .text("%")
                                        .custom_formatter(|n, _| format!("{:.0}%", n * 100.0)),
                                );
                                
                                // Custom input for values beyond 100% (up to 200%)
                                let drag_response = ui.add(
                                    DragValue::new(&mut swing)
                                        .range(0.0..=2.0)
                                        .speed(0.01)
                                        .suffix("%")
                                        .custom_formatter(|n, _| format!("{:.1}", n * 100.0)),
                                );
                                
                                if slider_response.changed() || drag_response.changed() {
                                    self.swing_menu_ratio = swing.clamp(0.0, 2.0);
                                    // Apply swing in real-time
                                    self.apply_swing_to_selected_notes_realtime(self.swing_menu_ratio);
                                }
                            });
                        }
                    });
                });
            
            // Close menu if user clicks elsewhere
            // Logic: if click is NOT in the menu rect, close the menu
            // Valid buttons (Quantize, Humanize, Batch Transform) already set context_menu_pos = None
            // Submenu buttons (Snap Mode, Snap Interval) don't close the menu (egui handles this)
            let ctx = ui.ctx();
            if ctx.input(|i| i.pointer.primary_clicked() || i.pointer.secondary_clicked()) {
                if let Some(click_pos) = ctx.input(|i| i.pointer.interact_pos()) {
                    // Get the actual menu rect from the Area response
                    let menu_rect = menu_response.response.rect;
                    
                    // Ignore the click that opened the menu (same position within a small threshold)
                    let ignore_click = if let Some(open_pos) = self.context_menu_open_pos {
                        let threshold = 5.0; // pixels
                        click_pos.distance(open_pos) < threshold
                    } else {
                        false
                    };
                    
                    // Simple logic: if click is NOT in menu rect, close menu
                    // Submenus (menu_button popups) are handled by egui automatically
                    // If user clicks outside the main menu, close it
                    if !ignore_click && !menu_rect.contains(click_pos) {
                        // Clear swing adjustment state when closing menu
                        self.swing_original_notes.clear();
                        self.swing_menu_ratio = 0.0;
                        self.context_menu_pos = None;
                        self.context_menu_open_pos = None;
                    }
                    // If click is inside menu_rect, don't close (menu item click will handle it)
                } else {
                    // Can't determine click position, close menu to be safe
                    // Clear swing adjustment state when closing menu
                    self.swing_original_notes.clear();
                    self.swing_menu_ratio = 0.0;
                    self.context_menu_pos = None;
                    self.context_menu_open_pos = None;
                }
            }
        }
        
        // Playback settings dialog
        if self.show_playback_settings {
            egui::Window::new("Playback Settings")
                .collapsible(false)
                .resizable(true)
                .show(ui.ctx(), |ui| {
                    ui.set_min_width(300.0);
                    
                    ui.label("Volume:");
                    let mut volume = self.volume;
                    if ui
                        .add(
                            Slider::new(&mut volume, 0.0..=1.0)
                                .text("%")
                                .custom_formatter(|n, _| format!("{:.0}%", n * 200.0)),
                        )
                        .changed()
                    {
                        self.set_volume(volume);
                    } else {
                        self.volume = volume;
                    }

                    ui.separator();
                    ui.label("Pitch:");
                    let mut pitch = self.preview_pitch_shift;
                    if ui
                        .add(Slider::new(&mut pitch, -12.0..=12.0).text("± semitone"))
                        .changed()
                    {
                        self.preview_pitch_shift = pitch;
                        if let Some(playback) = &self.playback {
                            playback.set_pitch_shift(pitch);
                        }
                    }

                    ui.separator();
                    ui.checkbox(&mut self.loop_enabled, "Loop");
                    if self.loop_enabled {
                        ui.horizontal(|ui| {
                            ui.label("Start:");
                            let mut loop_start = self.loop_start_tick as i64;
                            if ui
                                .add(DragValue::new(&mut loop_start).speed(1.0))
                                .changed()
                            {
                                self.loop_start_tick = loop_start.max(0) as u64;
                                if self.loop_start_tick >= self.loop_end_tick {
                                    self.loop_end_tick = self.loop_start_tick + 1;
                                }
                            }
                            ui.label("End:");
                            let mut loop_end = self.loop_end_tick as i64;
                            if ui
                                .add(DragValue::new(&mut loop_end).speed(1.0))
                                .changed()
                            {
                                self.loop_end_tick = loop_end.max((self.loop_start_tick + 1) as i64) as u64;
                            }
                        });
                    }

                    ui.separator();
                    ui.label("Snap Interval:");
                    let mut snap = self.snap_interval;
                    let snap_label = if snap == 0 {
                        "Free".to_owned()
                    } else {
                        format!("1/{}", (480 * 4 / snap).max(1))
                    };
                    ComboBox::from_id_salt("snap_combo_dialog")
                        .selected_text(snap_label)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut snap, 480 * 4, "1/1");
                            ui.selectable_value(&mut snap, 480 * 2, "1/2");
                            ui.selectable_value(&mut snap, 480, "1/4");
                            ui.selectable_value(&mut snap, 240, "1/8");
                            ui.selectable_value(&mut snap, 120, "1/16");
                            ui.selectable_value(&mut snap, 0, "Free");
                        });
                    if snap != self.snap_interval {
                        self.set_snap_interval(snap);
                    }

                    ui.separator();
                    ui.label("Snap Mode:");
                    ComboBox::from_id_salt("snap_mode_dialog")
                        .selected_text(match self.snap_mode {
                            SnapMode::Absolute => "Absolute",
                            SnapMode::Relative => "Relative",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.snap_mode, SnapMode::Absolute, "Absolute");
                            ui.selectable_value(&mut self.snap_mode, SnapMode::Relative, "Relative");
                        });

                    ui.separator();
                    if ui.button("Close").clicked() {
                        self.show_playback_settings = false;
                    }
                });
        }
        
        // Batch transform dialog
        if self.show_batch_transform_dialog {
            egui::Window::new("Batch Transform")
                .collapsible(false)
                .resizable(true)
                .show(ui.ctx(), |ui| {
                    ui.vertical(|ui| {
                        ui.label("Transform Type:");
                        ui.horizontal(|ui| {
                            if ui.selectable_label(
                                self.batch_transform_type == BatchTransformType::VelocityOffset,
                                "Velocity Offset",
                            ).clicked() {
                                self.batch_transform_type = BatchTransformType::VelocityOffset;
                            }
                            if ui.selectable_label(
                                self.batch_transform_type == BatchTransformType::DurationScale,
                                "Duration Scale",
                            ).clicked() {
                                self.batch_transform_type = BatchTransformType::DurationScale;
                            }
                            if ui.selectable_label(
                                self.batch_transform_type == BatchTransformType::PitchOffset,
                                "Pitch Offset",
                            ).clicked() {
                                self.batch_transform_type = BatchTransformType::PitchOffset;
                            }
                        });
                        
                        ui.add_space(10.0);
                        
                        match self.batch_transform_type {
                            BatchTransformType::VelocityOffset => {
                                ui.label("Velocity offset (-127 to +127):");
                                ui.add(egui::Slider::new(&mut self.batch_transform_value, -127.0..=127.0));
                            }
                            BatchTransformType::DurationScale => {
                                ui.label("Duration scale factor (0.1 to 10.0):");
                                ui.add(egui::Slider::new(&mut self.batch_transform_value, 0.1..=10.0));
                            }
                            BatchTransformType::PitchOffset => {
                                ui.label("Pitch offset (semitones, -127 to +127):");
                                ui.add(egui::Slider::new(&mut self.batch_transform_value, -127.0..=127.0));
                            }
                        }
                        
                        ui.add_space(10.0);
                        
                        ui.horizontal(|ui| {
                            if ui.button("Apply").clicked() {
                                if !self.selected_notes.is_empty() {
                                    self.apply_command(EditorCommand::BatchTransform {
                                        transform_type: self.batch_transform_type,
                                        value: self.batch_transform_value,
                                    });
                                }
                                self.show_batch_transform_dialog = false;
                            }
                            if ui.button("Cancel").clicked() {
                                self.show_batch_transform_dialog = false;
                            }
                        });
                    });
                });
        }
        
    }

    fn update_sequencer(&mut self) {
        if self.state.ticks_per_beat == 0 || self.state.bpm <= 0.0 {
            return;
        }

        let seconds_per_beat = 60.0 / self.state.bpm;
        let seconds_per_tick = seconds_per_beat / self.state.ticks_per_beat as f32;

        let current_tick = (self.current_time / seconds_per_tick) as u64;

        if let Some(playback) = &self.playback {
            for note in &self.state.notes {
                // Check for Note On: start lies between last_tick (exclusive) and current_tick (inclusive)
                // Note: We use > last_tick to ensure we don't retrigger if we paused exactly on start.
                // But for FIRST note starting at 0, last_tick might be 0.
                // So special case: if last_tick is 0, include 0.

                let should_trigger_start = if self.last_tick == 0 {
                    note.start >= self.last_tick && note.start <= current_tick
                } else {
                    note.start > self.last_tick && note.start <= current_tick
                };

                if should_trigger_start {
                    let velocity = self.state.apply_velocity_curve_to_note(note);
                    playback.note_on(note.key, velocity);
                }

                // Check for Note Off: end lies between last_tick and current_tick
                let end = note.start + note.duration;
                if end > self.last_tick && end <= current_tick {
                    playback.note_off(note.key);
                }
            }
        }

        // Handle loop playback
        if self.loop_enabled && self.is_playing {
            let loop_duration_ticks = self.loop_end_tick.saturating_sub(self.loop_start_tick);
            if loop_duration_ticks > 0 && current_tick >= self.loop_end_tick {
                // Jump back to loop start
                let _loop_duration_seconds = loop_duration_ticks as f32 * seconds_per_tick;
                self.current_time = self.loop_start_tick as f32 * seconds_per_tick;
                // Set last_tick to one less than loop_start to ensure notes at loop_start are triggered
                // If loop_start is 0, we use 0 (which is handled specially in the trigger logic)
                self.last_tick = self.loop_start_tick.saturating_sub(1);
                // Stop all notes when looping
                if let Some(playback) = &self.playback {
                    playback.all_notes_off();
                }
                // Don't update last_tick to current_tick after loop jump, use the value we set above
                self.emit_transport_event();
                return;
            } else if current_tick < self.loop_start_tick {
                // If somehow before loop start, jump to loop start
                self.current_time = self.loop_start_tick as f32 * seconds_per_tick;
                // Same logic as above for ensuring notes at loop_start are triggered
                self.last_tick = self.loop_start_tick.saturating_sub(1);
                // Don't update last_tick to current_tick after loop jump, use the value we set above
                self.emit_transport_event();
                return;
            }
        }

        self.last_tick = current_tick;
        self.emit_transport_event();
    }

    fn ui_inspector(&mut self, ui: &mut Ui, min_height: f32) {
        ui.set_min_width(240.0);
        ui.set_min_height(min_height);
        ui.vertical(|ui| {
            ui.heading("Inspector");
            ui.separator();
            let selection_len = self.selected_notes.len();
            ui.label(format!("Selected notes: {selection_len}"));
            ui.add_space(4.0);
            if selection_len == 0 {
                if ui
                    .add_enabled(!self.clipboard.is_empty(), Button::new("Paste at playhead"))
                    .clicked()
                {
                    let tick = self.current_tick_position();
                    self.paste_clipboard_at(tick);
                }
                ui.label("Tip: Hold Shift to box-select inside the piano roll.");
                return;
            }

            ui.horizontal(|ui| {
                if ui.button("Copy").clicked() {
                    self.copy_selection();
                }
                if ui.button("Cut").clicked() {
                    self.cut_selection();
                }
                if ui
                    .add_enabled(!self.clipboard.is_empty(), Button::new("Paste"))
                    .clicked()
                {
                    let tick = self.current_tick_position();
                    self.paste_clipboard_at(tick);
                }
                if ui.button("Delete").clicked() {
                    self.delete_selected_notes();
                }
            });

            if ui
                .add_enabled(self.snap_interval > 0, Button::new("Quantize to snap grid"))
                .clicked()
            {
                self.quantize_selected_notes();
            }

            ui.separator();
            ui.label("Advanced Tools");
            ui.horizontal(|ui| {
                if ui.button("Humanize").clicked() {
                    // Default: ±10 ticks time, ±5 velocity
                    let time_range = (self.snap_interval / 12).max(1).min(20);
                    let velocity_range = 5;
                    self.apply_command(EditorCommand::HumanizeNotes {
                        time_range,
                        velocity_range,
                    });
                }
                if ui.button("Batch Transform...").clicked() {
                    self.show_batch_transform_dialog = true;
                }
            });

            if selection_len == 1 {
                if let Some(note) = self.first_selected_note() {
                    self.draw_single_note_inspector(ui, note);
                }
            } else {
                self.draw_multi_note_summary(ui);
            }
        });
    }

    fn draw_single_note_inspector(&mut self, ui: &mut Ui, note: Note) {
        ui.separator();
        ui.label("Single note properties");
        let note_id = note.id;

        let mut start = note.start as i64;
        ui.horizontal(|ui| {
            ui.label("Start");
            if ui
                .add(DragValue::new(&mut start).speed(self.snap_interval.max(1) as f64))
                .changed()
            {
                let start = start.max(0) as u64;
                self.edit_note_by_id(note_id, |n| n.start = start);
            }
        });

        let mut duration = note.duration as i64;
        ui.horizontal(|ui| {
            ui.label("Duration");
            if ui
                .add(DragValue::new(&mut duration).speed(self.snap_interval.max(1) as f64))
                .changed()
            {
                let duration = duration.max(1) as u64;
                self.edit_note_by_id(note_id, |n| n.duration = duration);
            }
        });

        let mut key = note.key as i32;
        if ui
            .add(Slider::new(&mut key, 0..=127).text("Pitch"))
            .changed()
        {
            let key = key as u8;
            self.edit_note_by_id(note_id, |n| n.key = key);
        }

        let mut velocity = note.velocity as i32;
        if ui
            .add(Slider::new(&mut velocity, 1..=127).text("Velocity"))
            .changed()
        {
            let velocity = velocity as u8;
            self.edit_note_by_id(note_id, |n| n.velocity = velocity);
        }
    }

    fn draw_multi_note_summary(&self, ui: &mut Ui) {
        let snapshot = self.selected_notes_snapshot();
        if snapshot.is_empty() {
            return;
        }
        let min_start = snapshot.iter().map(|n| n.start).min().unwrap_or(0);
        let max_start = snapshot.iter().map(|n| n.start).max().unwrap_or(0);
        let min_duration = snapshot.iter().map(|n| n.duration).min().unwrap_or(0);
        let max_duration = snapshot.iter().map(|n| n.duration).max().unwrap_or(0);
        let avg_velocity =
            snapshot.iter().map(|n| n.velocity as u32).sum::<u32>() / snapshot.len() as u32;

        ui.separator();
        ui.label("Multi-note summary");
        ui.label(format!("Start range: {min_start} - {max_start}"));
        ui.label(format!("Duration range: {min_duration} - {max_duration}"));
        ui.label(format!("Average velocity: {avg_velocity}"));
    }

    fn ui_toolbar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // Time display
            let seconds_per_beat = 60.0 / self.state.bpm.max(1.0);
            let seconds_per_tick = seconds_per_beat / self.state.ticks_per_beat.max(1) as f32;
            let _current_tick = (self.current_time / seconds_per_tick) as u64;
            let total_seconds = self.current_time;
            let minutes = (total_seconds / 60.0) as u32;
            let seconds = (total_seconds % 60.0) as u32;
            let milliseconds = ((total_seconds % 1.0) * 1000.0) as u32;
            let time_display = format!("{:02}:{:02}.{:03}", minutes, seconds, milliseconds);
            ui.label(format!("Time: {}", time_display));
            ui.separator();
            
            if ui
                .button(if self.is_playing {
                    "⏸ Pause"
                } else {
                    "▶ Play"
                })
                .clicked()
            {
                self.is_playing = !self.is_playing;
                if self.is_playing {
                    self.last_update = ui.input(|i| i.time);
                    // Reset last_tick to avoid mass triggering if we jumped
                    let seconds_per_beat = 60.0 / self.state.bpm;
                    let seconds_per_tick = seconds_per_beat / self.state.ticks_per_beat as f32;
                    // If we are at 0, set last_tick to 0 to include start notes.
                    // If we are later, set to just before current to avoid retriggering current tick notes?
                    // Let's just set it exact.
                    self.last_tick = (self.current_time / seconds_per_tick) as u64;
                    // If we just started from 0, last_tick is 0. Our logic in update_sequencer handles 0 specially?
                    // No, logic says if last_tick == 0, include 0.
                    // So if we seek to 0 and play, it works.
                    // If we pause at 0 and play, it re-triggers. That's probably fine.
                } else {
                    self.stop_playback_backend();
                }
                self.emit_event(EditorEvent::PlaybackStateChanged {
                    is_playing: self.is_playing,
                });
                if self.is_playing {
                    self.notify_playback_started();
                } else {
                    self.notify_playback_stopped();
                }
            }
            if ui.button("⏹ Stop").clicked() {
                self.is_playing = false;
                self.current_time = 0.0;
                self.last_tick = 0;
                self.stop_playback_backend();
                self.notify_playback_stopped();
                self.emit_event(EditorEvent::PlaybackStateChanged { is_playing: false });
                self.emit_transport_event();
            }

            ui.separator();

            if ui
                .add_enabled(!self.undo_stack.is_empty(), Button::new("↺"))
                .clicked()
            {
                self.undo();
            }
            if ui
                .add_enabled(!self.redo_stack.is_empty(), Button::new("↻"))
                .clicked()
            {
                self.redo();
            }

            ui.separator();

            ui.label("Sig:");
            ui.horizontal(|ui| {
                let mut numer = self.state.time_signature.0;
                let mut denom = self.state.time_signature.1;
                let numer_changed = ui
                    .add(DragValue::new(&mut numer).speed(0.1).range(1..=32))
                    .changed();
                ui.label("/");
                let denom_changed = ui
                    .add(DragValue::new(&mut denom).speed(0.1).range(1..=32))
                    .changed();
                if numer_changed || denom_changed {
                    self.set_time_signature(numer, denom);
                }
            });

            ui.separator();

            ui.label("BPM:");
            let mut bpm = self.state.bpm;
            if ui
                .add(DragValue::new(&mut bpm).speed(1.0).range(20.0..=400.0))
                .changed()
            {
                self.set_bpm(bpm);
            }

            ui.separator();

            // Display loop status and playback position
            if self.loop_enabled {
                ui.horizontal(|ui| {
                    ui.label("🔁 Loop:");
                    let seconds_per_beat = 60.0 / self.state.bpm;
                    let seconds_per_tick = seconds_per_beat / self.state.ticks_per_beat as f32;
                    let loop_start_seconds = self.loop_start_tick as f32 * seconds_per_tick;
                    let loop_end_seconds = self.loop_end_tick as f32 * seconds_per_tick;
                    ui.label(format!(
                        "{:.2}s - {:.2}s",
                        loop_start_seconds,
                        loop_end_seconds
                    ));
                });
            }

            ui.horizontal(|ui| {
                ui.label("Position:");
                let current_beat = self.current_time * self.state.bpm / 60.0;
                let current_measure = (current_beat / self.state.time_signature.0 as f32).floor() + 1.0;
                let beat_in_measure = (current_beat % self.state.time_signature.0 as f32) + 1.0;
                ui.label(format!(
                    "{:.2}s ({:.1}:{:.1})",
                    self.current_time,
                    current_measure,
                    beat_in_measure
                ));
            });

            ui.separator();

            if ui.button("⚙ Playback Settings").clicked() {
                self.show_playback_settings = true;
            }
        });
    }

    fn ui_piano_roll(&mut self, ui: &mut Ui) {
        let key_width = 60.0;
        let timeline_height = 30.0;

        // Piano Roll ScrollArea
        // We disable built-in scrolling since we handle it manually via middle mouse
        ScrollArea::both()
            .auto_shrink([false, false])
            .enable_scrolling(false) // Disable wheel scroll
            .show(ui, |ui| {
                let available_size = ui.available_size();
                let (rect, response) =
                    ui.allocate_exact_size(available_size, Sense::click_and_drag());

                // Handle Zoom (Ctrl/Alt + Scroll)
                let scroll_delta = ui.input(|i| i.raw_scroll_delta);
                if scroll_delta != Vec2::ZERO {
                    if ui.input(|i| i.modifiers.ctrl) {
                        // Zoom X (Horizontal) around mouse pointer
                        if scroll_delta.y != 0.0 {
                            let old_zoom = self.zoom_x;
                            let new_zoom = (self.zoom_x
                                * if scroll_delta.y > 0.0 { 1.1 } else { 0.9 })
                            .clamp(10.0, 500.0);

                            // Calculate mouse position relative to timeline start (in beats)
                            // mouse_x = start_x + beats * zoom + scroll
                            // beats = (mouse_x - start_x - scroll) / zoom
                            if let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos()) {
                                let rel_x = mouse_pos.x - (rect.min.x + key_width);
                                let beats_at_mouse = (rel_x - self.manual_scroll_x) / old_zoom;

                                // We want beats_at_mouse to stay at rel_x after zoom
                                // rel_x = beats_at_mouse * new_zoom + new_scroll
                                // new_scroll = rel_x - beats_at_mouse * new_zoom
                                self.manual_scroll_x = rel_x - beats_at_mouse * new_zoom;
                            }

                            self.zoom_x = new_zoom;
                        }
                    } else if ui.input(|i| i.modifiers.alt) {
                        // Zoom Y (Vertical) around mouse pointer
                        if scroll_delta.y != 0.0 {
                            let old_zoom = self.zoom_y;
                            let new_zoom = (self.zoom_y
                                * if scroll_delta.y > 0.0 { 1.1 } else { 0.9 })
                            .clamp(5.0, 50.0);

                            if let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos()) {
                                let rel_y = mouse_pos.y - (rect.min.y + timeline_height);
                                // keys_from_top = (rel_y - scroll) / zoom
                                // Note: Our y calc is more complex: note_to_y = (127 - note) * zoom
                                // But linear mapping is: y = value * zoom + scroll.
                                // So logic is same.
                                let value_at_mouse = (rel_y - self.manual_scroll_y) / old_zoom;
                                self.manual_scroll_y = rel_y - value_at_mouse * new_zoom;
                            }

                            self.zoom_y = new_zoom;
                        }
                    }
                }

                // Handle Middle Mouse Pan
                // Note: interact_pos() returns None if not interacting with specific widget?
                // For global pointer, use pointer.interact_pos() or pointer.pos().
                if ui.input(|i| i.pointer.middle_down()) {
                    if self.is_panning {
                        if let Some(start) = self.pan_start {
                            // use pointer.pos() for raw screen coordinates, consistent for dragging
                            if let Some(curr) = ui.input(|i| i.pointer.hover_pos()) {
                                let delta = curr - start;
                                // Apply delta to scroll.
                                // ui.scroll_with_delta(delta); // SCROLLAREA LOGIC REMOVED
                                // Manually update scroll offset
                                self.manual_scroll_x += delta.x;
                                self.manual_scroll_y += delta.y;

                                // Update start for continuous delta
                                self.pan_start = Some(curr);
                            }
                        }
                    } else {
                        self.is_panning = true;
                        self.pan_start = ui.input(|i| i.pointer.hover_pos());
                    }
                } else {
                    self.is_panning = false;
                    self.pan_start = None;
                }

                // Apply Scroll Constraints
                // Limit horizontal scroll: can't scroll past 0 (can't see negative time)
                // Note: manual_scroll_x is an offset. Content moves right when scroll_x is positive?
                // Wait, usually scroll is negative offset to move content left (to see right side).
                // Let's assume manual_scroll_x <= 0 means we have scrolled right.
                // If manual_scroll_x > 0, we see negative time gap.
                // So max x is 0.

                if self.manual_scroll_x > 0.0 {
                    self.manual_scroll_x = 0.0;
                }

                // Limit vertical scroll
                let total_content_height = 128.0 * self.zoom_y;
                let view_height = rect.height() - timeline_height;
                // If content < view, pin to top.
                // If content > view, allow scrolling down to (view - content).

                let min_scroll_y = if total_content_height > view_height {
                    view_height - total_content_height
                } else {
                    0.0
                };

                // Clamp scroll y between min_scroll_y and 0.
                self.manual_scroll_y = self.manual_scroll_y.clamp(min_scroll_y, 0.0);

                let mut pointer_consumed = false;
                let note_offset_x = rect.min.x + key_width + self.manual_scroll_x;

                // Coordinate transforms (defined early for use in interactions)
                let tick_to_x = |tick: u64, zoom_x: f32, ticks_per_beat: u16| -> f32 {
                    (tick as f32 / ticks_per_beat as f32) * zoom_x
                };

                // Handle timeline interactions (playhead seek and loop editing)
                if let Some(pointer) = response.interact_pointer_pos() {
                    let in_timeline = pointer.y < rect.min.y + timeline_height
                        && pointer.x >= rect.min.x + key_width;
                    
                    if in_timeline {
                        let modifiers = ui.input(|i| i.modifiers);
                        let is_shift = modifiers.shift;
                        let disable_snap = modifiers.alt;
                        
                        // Convert pointer position to tick
                        let mut x = pointer.x - (rect.min.x + key_width);
                        x = (x - self.manual_scroll_x).max(0.0);
                        let beats = x / self.zoom_x;
                        let seconds_per_beat = 60.0 / self.state.bpm;
                        let seconds_per_tick = seconds_per_beat / self.state.ticks_per_beat as f32;
                        let tick = (beats * seconds_per_beat / seconds_per_tick) as i64;
                        let snapped_tick = self.snap_tick(tick, None, disable_snap);
                        
                        // Handle drag start
                        if ui.input(|i| i.pointer.primary_pressed()) && !self.is_dragging_note {
                            if is_shift {
                                // Shift + 左键：开始循环边界编辑
                                if self.loop_enabled {
                                    // Determine edit mode based on pointer position relative to loop region
                                    let loop_start_x = note_offset_x
                                        + tick_to_x(self.loop_start_tick, self.zoom_x, self.state.ticks_per_beat);
                                    let loop_end_x = note_offset_x
                                        + tick_to_x(self.loop_end_tick, self.zoom_x, self.state.ticks_per_beat);
                                    
                                    let loop_width = loop_end_x - loop_start_x;
                                    let relative_x = pointer.x - loop_start_x;
                                    
                                    let edit_mode = if loop_width > 0.0 {
                                        if relative_x < loop_width / 3.0 {
                                            LoopEditMode::Start
                                        } else if relative_x > loop_width * 2.0 / 3.0 {
                                            LoopEditMode::End
                                        } else {
                                            LoopEditMode::Move
                                        }
                                    } else {
                                        LoopEditMode::Move
                                    };
                                    
                                    self.drag_action = DragAction::LoopEdit;
                                    self.loop_edit_mode = Some(edit_mode);
                                    self.drag_original_loop_start = Some(self.loop_start_tick);
                                    self.drag_original_loop_end = Some(self.loop_end_tick);
                                    self.drag_start_pos = Some(pointer);
                                    pointer_consumed = true;
                                } else {
                                    // Loop not enabled: enable it and set initial region
                                    self.loop_enabled = true;
                                    self.drag_action = DragAction::LoopEdit;
                                    self.loop_edit_mode = Some(LoopEditMode::Move);
                                    self.loop_start_tick = snapped_tick;
                                    self.loop_end_tick = (snapped_tick + 1920).max(snapped_tick + 1);
                                    self.drag_original_loop_start = Some(self.loop_start_tick);
                                    self.drag_original_loop_end = Some(self.loop_end_tick);
                                    self.drag_start_pos = Some(pointer);
                                    pointer_consumed = true;
                                }
                            } else {
                                // 单独左键：开始播放位置调整
                                self.drag_action = DragAction::PlayheadSeek;
                                self.current_time = snapped_tick as f32 * seconds_per_tick;
                                self.last_tick = snapped_tick;
                                self.is_dragging_note = false;
                                self.emit_transport_event();
                                pointer_consumed = true;
                            }
                        }
                        
                        // Handle drag update
                        if ui.input(|i| i.pointer.primary_down()) {
                            match self.drag_action {
                                DragAction::LoopEdit => {
                                    if let Some(edit_mode) = self.loop_edit_mode {
                                        match edit_mode {
                                            LoopEditMode::Start => {
                                                self.loop_start_tick = snapped_tick.min(self.loop_end_tick.saturating_sub(1));
                                            }
                                            LoopEditMode::End => {
                                                self.loop_end_tick = snapped_tick.max(self.loop_start_tick + 1);
                                            }
                                            LoopEditMode::Move => {
                                                if let (Some(original_start), Some(original_end), Some(start_pos)) = 
                                                    (self.drag_original_loop_start, self.drag_original_loop_end, self.drag_start_pos) {
                                                    let delta_x = pointer.x - start_pos.x;
                                                    let delta_beats = delta_x / self.zoom_x;
                                                    let delta_ticks = (delta_beats * seconds_per_beat / seconds_per_tick) as i64;
                                                    
                                                    let loop_duration = original_end - original_start;
                                                    let new_start_raw = (original_start as i64 + delta_ticks).max(0) as i64;
                                                    let new_start = self.snap_tick(new_start_raw, None, disable_snap);
                                                    let new_end = new_start + loop_duration;
                                                    
                                                    self.loop_start_tick = new_start;
                                                    self.loop_end_tick = new_end;
                                                }
                                            }
                                        }
                                    }
                                    pointer_consumed = true;
                                }
                                DragAction::PlayheadSeek => {
                                    self.current_time = snapped_tick as f32 * seconds_per_tick;
                                    self.last_tick = snapped_tick;
                                    self.emit_transport_event();
                                    pointer_consumed = true;
                                }
                                _ => {}
                            }
                        }
                        
                        // Handle drag end
                        if ui.input(|i| i.pointer.primary_released()) {
                            if matches!(self.drag_action, DragAction::LoopEdit | DragAction::PlayheadSeek) {
                                self.drag_action = DragAction::None;
                                self.loop_edit_mode = None;
                                self.drag_original_loop_start = None;
                                self.drag_original_loop_end = None;
                                self.drag_start_pos = None;
                            }
                        }
                        
                        // Update cursor based on hover state
                        if !self.is_dragging_note {
                            if is_shift {
                                if self.loop_enabled {
                                    let loop_start_x = note_offset_x
                                        + tick_to_x(self.loop_start_tick, self.zoom_x, self.state.ticks_per_beat);
                                    let loop_end_x = note_offset_x
                                        + tick_to_x(self.loop_end_tick, self.zoom_x, self.state.ticks_per_beat);
                                    
                                    let loop_width = loop_end_x - loop_start_x;
                                    let relative_x = pointer.x - loop_start_x;
                                    
                                    if loop_width > 0.0 {
                                        if relative_x < loop_width / 3.0 {
                                            ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
                                        } else if relative_x > loop_width * 2.0 / 3.0 {
                                            ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
                                        } else {
                                            ui.ctx().set_cursor_icon(CursorIcon::Grab);
                                        }
                                    } else {
                                        ui.ctx().set_cursor_icon(CursorIcon::Grab);
                                    }
                                } else {
                                    ui.ctx().set_cursor_icon(CursorIcon::Grab);
                                }
                            } else {
                                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                            }
                        }
                    }
                }

                // Coordinate transforms
                let time_to_x = |time: f32, zoom_x: f32| -> f32 {
                    // time in beats
                    time * zoom_x
                };

                let tick_to_x = |tick: u64, zoom_x: f32, ticks_per_beat: u16| -> f32 {
                    (tick as f32 / ticks_per_beat as f32) * zoom_x
                };

                let note_to_y = |note: u8, zoom_y: f32| -> f32 {
                    // High notes at top, low notes at bottom.
                    (127 - note) as f32 * zoom_y
                };

                let painter = ui.painter_at(rect);
                let grid_top = rect.min.y + timeline_height;
                let grid_bottom = rect.max.y;
                let measure_line_color = Color32::from_rgb(210, 210, 210);
                let beat_line_color = Color32::from_rgb(140, 140, 140);
                let subdivision_color = Color32::from_rgb(90, 90, 90);
                let horizontal_line_color = Color32::from_rgb(90, 90, 90);
                let separator_color = Color32::from_rgb(130, 130, 130);

                // Draw Vertical Grid (Beats / Measures / Subdivisions)
                let tpb = self.state.ticks_per_beat.max(1) as u64;
                let denom = self.state.time_signature.1.max(1) as u64;
                let numer = self.state.time_signature.0.max(1) as u64;
                let ticks_per_measure = (tpb * numer * 4).saturating_div(denom).max(tpb);

                let visible_beats_start = (-self.manual_scroll_x / self.zoom_x).floor();
                let visible_beats_end = visible_beats_start + (rect.width() / self.zoom_x) + 2.0;
                let mut start_tick = (visible_beats_start * tpb as f32).floor() as i64;
                if start_tick < 0 {
                    start_tick = 0;
                }
                let end_tick = (visible_beats_end * tpb as f32).ceil() as i64;

                let subdivision = if self.zoom_x >= 220.0 {
                    8
                } else if self.zoom_x >= 90.0 {
                    4
                } else if self.zoom_x >= 45.0 {
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
                    let x = note_offset_x + (tick as f32 / tpb as f32) * self.zoom_x;
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

                // Draw Grid (Horizontal/Notes)
                for i in 0..=127 {
                    let y = rect.min.y
                        + timeline_height
                        + note_to_y((127 - i) as u8, self.zoom_y)
                        + self.manual_scroll_y;

                    // Only draw if visible (and maybe clip)
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

                // Handle Note Off if we released mouse anywhere
                if ui.input(|i| i.pointer.primary_released()) {
                    if let Some(note) = self.active_key_note {
                        if let Some(playback) = &self.playback {
                            playback.note_off(note);
                        }
                        self.active_key_note = None;
                    }
                }

                let base_x = rect.min.x + key_width;
                let base_y = rect.min.y + timeline_height;
                let manual_scroll_x = self.manual_scroll_x;
                let manual_scroll_y = self.manual_scroll_y;
                let zoom_x = self.zoom_x;
                let zoom_y = self.zoom_y;
                let ticks_per_beat = self.state.ticks_per_beat as f32;

                let pointer_to_tick = move |pos: Pos2| -> i64 {
                    let rel_x = pos.x - base_x - manual_scroll_x;
                    let beats = rel_x / zoom_x;
                    (beats * ticks_per_beat).round() as i64
                };
                let pointer_to_key = move |pos: Pos2| -> u8 {
                    let keyboard_top = base_y + manual_scroll_y;
                    let rel_y = pos.y - keyboard_top;
                    let key_val = 127.0 - rel_y / zoom_y;
                    key_val.clamp(0.0, 127.0).round() as u8
                };

                // Draw Notes with viewport culling for performance
                let note_offset_y = rect.min.y + timeline_height + self.manual_scroll_y;
                
                // Calculate visible time range for culling (notes are sorted by start time)
                let visible_start_tick = if self.manual_scroll_x < 0.0 {
                    ((-self.manual_scroll_x / self.zoom_x) * self.state.ticks_per_beat as f32) as u64
                } else {
                    0
                };
                let visible_end_tick = visible_start_tick.saturating_add(
                    ((rect.width() / self.zoom_x) * self.state.ticks_per_beat as f32) as u64 + 1
                );
                
                // Use binary search to find notes in visible time range
                let notes_snapshot = &self.state.notes;
                let start_idx = notes_snapshot.partition_point(|n| n.start + n.duration < visible_start_tick);
                let end_idx = notes_snapshot.partition_point(|n| n.start <= visible_end_tick);
                
                // Collect note IDs and rects first to avoid borrow conflicts
                let visible_notes: Vec<(NoteId, Rect)> = notes_snapshot[start_idx..end_idx.min(notes_snapshot.len())]
                    .iter()
                    .map(|note| {
                        let x = note_offset_x
                            + tick_to_x(note.start, self.zoom_x, self.state.ticks_per_beat);
                        let y = note_offset_y + note_to_y(note.key, self.zoom_y);
                        let w =
                            tick_to_x(note.duration, self.zoom_x, self.state.ticks_per_beat).max(5.0);
                        let h = self.zoom_y;
                        let note_rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h));
                        (note.id, note_rect)
                    })
                    .filter(|(_, note_rect)| note_rect.intersects(rect))
                    .collect();
                
                // Now draw and handle interactions
                for (note_id, note_rect) in &visible_notes {
                    let is_selected = self.selected_notes.contains(note_id);
                    let color = if is_selected {
                        Color32::from_rgb(150, 250, 150)
                    } else {
                        Color32::from_rgb(100, 200, 100)
                    };
                    painter.rect_filled(note_rect.shrink(1.0), 2.0, color);
                    // Draw stroke: 4x thicker white stroke for selected notes, normal for others
                    let stroke_width = if is_selected { 4.0 } else { 1.0 };
                    painter.rect_stroke(
                        note_rect.shrink(1.0),
                        2.0,
                        Stroke::new(stroke_width, Color32::WHITE),
                    );
                }
                
                // Handle interactions (need to find note by ID)
                for (note_id, note_rect) in &visible_notes {
                    if response.clicked_by(PointerButton::Primary) {
                        if let Some(pointer) = response.interact_pointer_pos() {
                            if note_rect.contains(pointer) {
                                let modifiers = ui.input(|i| i.modifiers);
                                self.handle_note_click(*note_id, modifiers);
                                pointer_consumed = true;
                            }
                        }
                    }

                    if !self.is_dragging_note && ui.input(|i| i.pointer.primary_pressed()) {
                        if let Some(pointer) = response.interact_pointer_pos() {
                            if note_rect.contains(pointer) {
                                let modifiers = ui.input(|i| i.modifiers);
                                self.prepare_selection_for_drag(*note_id, modifiers);
                                let action = self.resolve_drag_action(pointer, *note_rect);
                                let pointer_tick = pointer_to_tick(pointer);
                                self.begin_note_drag(*note_id, pointer, pointer_tick, action);
                                pointer_consumed = true;
                            }
                        }
                    }

                    if let Some(pointer) = response.hover_pos() {
                        let action = self.resolve_drag_action(pointer, *note_rect);
                        if matches!(action, DragAction::ResizeStart | DragAction::ResizeEnd)
                            && note_rect.contains(pointer)
                        {
                            ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
                        }
                    }

                    if response.clicked_by(PointerButton::Secondary) {
                        if let Some(pointer) = response.interact_pointer_pos() {
                            if note_rect.contains(pointer) {
                                let modifiers = ui.input(|i| i.modifiers);
                                if modifiers.shift {
                                    // Shift+右键：删除音符
                                    self.push_undo_snapshot();
                                    self.delete_note_by_id(*note_id);
                                    pointer_consumed = true;
                                } else {
                                    // 普通右键：显示上下文菜单
                                    self.context_menu_pos = Some(pointer);
                                    self.context_menu_open_pos = Some(pointer);
                                    pointer_consumed = true;
                                }
                            }
                        }
                    }
                }

                if self.is_dragging_note && ui.input(|i| i.pointer.primary_down()) {
                    if let Some(pointer) = response.interact_pointer_pos() {
                        let modifiers = ui.input(|i| i.modifiers);
                        self.update_note_drag(
                            pointer,
                            &pointer_to_tick,
                            &pointer_to_key,
                            modifiers,
                        );
                    }
                }

                if response.drag_stopped() {
                    self.preview_note_off();
                    self.finalize_note_drag_if_needed();
                    self.is_dragging_note = false;
                    self.is_resizing_note = false;
                    self.drag_action = DragAction::None;
                    self.drag_start_pos = None;
                    self.drag_original_start = None;
                    self.drag_original_duration = None;
                    self.drag_original_key = None;
                    self.drag_pointer_offset_ticks = None;
                    self.drag_original_notes.clear();
                    self.drag_primary_anchor = None;
                }

                if !pointer_consumed && ui.input(|i| i.pointer.primary_pressed()) {
                    if let Some(pointer) = response.interact_pointer_pos() {
                        let in_roll = pointer.x > rect.min.x + key_width
                            && pointer.y > rect.min.y + timeline_height;
                        if in_roll {
                            let modifiers = ui.input(|i| i.modifiers);
                            if modifiers.shift {
                                // Shift+左键：创建新音符
                                if !self.is_dragging_note {
                                    self.create_note_at_pointer(
                                        pointer,
                                        &pointer_to_tick,
                                        &pointer_to_key,
                                    );
                                }
                            } else {
                                // 直接左键：开始框选（如果不在音符上）
                                if !self.is_dragging_note {
                                    self.selection_box_start = Some(pointer);
                                    self.selection_box_end = Some(pointer);
                                }
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
                            let previous = if ui.input(|i| i.modifiers.shift) {
                                self.selected_notes.clone()
                            } else {
                                std::mem::take(&mut self.selected_notes)
                            };
                            // Optimize box selection with viewport culling
                            let selection_start_tick = if self.manual_scroll_x < 0.0 {
                                (((selection_rect.min.x - note_offset_x) / self.zoom_x) * self.state.ticks_per_beat as f32) as u64
                            } else {
                                0
                            };
                            let selection_end_tick = selection_start_tick.saturating_add(
                                ((selection_rect.width() / self.zoom_x) * self.state.ticks_per_beat as f32) as u64 + 1
                            );
                            
                            let notes_snapshot = &self.state.notes;
                            let sel_start_idx = notes_snapshot.partition_point(|n| n.start + n.duration < selection_start_tick);
                            let sel_end_idx = notes_snapshot.partition_point(|n| n.start <= selection_end_tick);
                            
                            for note in &notes_snapshot[sel_start_idx..sel_end_idx.min(notes_snapshot.len())] {
                                let x = note_offset_x
                                    + tick_to_x(note.start, self.zoom_x, self.state.ticks_per_beat);
                                let y = note_offset_y + note_to_y(note.key, self.zoom_y);
                                let w = tick_to_x(
                                    note.duration,
                                    self.zoom_x,
                                    self.state.ticks_per_beat,
                                )
                                .max(5.0);
                                let h = self.zoom_y;
                                let note_rect =
                                    Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h));
                                if note_rect.intersects(selection_rect) {
                                    self.selected_notes.insert(note.id);
                                }
                            }
                            self.notify_selection_changed(previous);
                        }
                        self.selection_box_start = None;
                        self.selection_box_end = None;
                    }
                }

                // Handle right-click on piano roll area (not on notes)
                if !pointer_consumed && response.clicked_by(PointerButton::Secondary) {
                    if let Some(pointer) = response.interact_pointer_pos() {
                        let in_roll = pointer.x > rect.min.x + key_width
                            && pointer.y > rect.min.y + timeline_height;
                        if in_roll {
                            let modifiers = ui.input(|i| i.modifiers);
                            if modifiers.shift {
                                // Shift+右键：删除选中音符
                                if !self.selected_notes.is_empty() {
                                    self.delete_selected_notes();
                                }
                            } else {
                                // 普通右键：显示上下文菜单
                                self.context_menu_pos = Some(pointer);
                                self.context_menu_open_pos = Some(pointer);
                            }
                        }
                    }
                }

                // Draw Timeline (Top Bar) - Drawn AFTER Notes
                // Fill timeline background
                let timeline_rect =
                    Rect::from_min_size(rect.min, Vec2::new(rect.width(), timeline_height));
                painter.rect_filled(timeline_rect, 0.0, ui.visuals().window_fill());
                painter.line_segment(
                    [timeline_rect.left_bottom(), timeline_rect.right_bottom()],
                    Stroke::new(1.0, separator_color), // Separator line
                );

                // Draw Timeline Labels (per measure)
                let mut measure_tick = (start_tick as u64 / ticks_per_measure) * ticks_per_measure;
                while measure_tick as i64 <= end_tick {
                    let x = note_offset_x + (measure_tick as f32 / tpb as f32) * self.zoom_x;
                    if x >= rect.min.x + key_width - 5.0 && x <= rect.max.x {
                        painter.line_segment(
                            [
                                Pos2::new(x, rect.min.y),
                                Pos2::new(x, rect.min.y + timeline_height),
                            ],
                            Stroke::new(1.0, measure_line_color),
                        );
                        let measure_index = (measure_tick / ticks_per_measure) + 1;
                        painter.text(
                            Pos2::new(x + 4.0, rect.min.y + 15.0),
                            Align2::LEFT_CENTER,
                            format!("{}", measure_index),
                            FontId::proportional(11.0),
                            Color32::GRAY,
                        );
                    }
                    measure_tick += ticks_per_measure;
                }

                // Draw Loop Markers on Timeline (if enabled)
                if self.loop_enabled {
                    let loop_start_x = note_offset_x
                        + tick_to_x(self.loop_start_tick, self.zoom_x, self.state.ticks_per_beat);
                    let loop_end_x = note_offset_x
                        + tick_to_x(self.loop_end_tick, self.zoom_x, self.state.ticks_per_beat);
                    
                    // Draw loop start marker
                    if loop_start_x >= rect.min.x + key_width && loop_start_x <= rect.max.x {
                        painter.add(Shape::convex_polygon(
                            vec![
                                Pos2::new(loop_start_x, rect.min.y),
                                Pos2::new(loop_start_x - 4.0, rect.min.y + 8.0),
                                Pos2::new(loop_start_x + 4.0, rect.min.y + 8.0),
                            ],
                            Color32::from_rgb(100, 150, 255),
                            Stroke::NONE,
                        ));
                        painter.text(
                            Pos2::new(loop_start_x, rect.min.y + 20.0),
                            Align2::CENTER_TOP,
                            "L",
                            FontId::proportional(9.0),
                            Color32::from_rgb(100, 150, 255),
                        );
                    }
                    
                    // Draw loop end marker
                    if loop_end_x >= rect.min.x + key_width && loop_end_x <= rect.max.x {
                        painter.add(Shape::convex_polygon(
                            vec![
                                Pos2::new(loop_end_x, rect.min.y),
                                Pos2::new(loop_end_x - 4.0, rect.min.y + 8.0),
                                Pos2::new(loop_end_x + 4.0, rect.min.y + 8.0),
                            ],
                            Color32::from_rgb(100, 150, 255),
                            Stroke::NONE,
                        ));
                        painter.text(
                            Pos2::new(loop_end_x, rect.min.y + 20.0),
                            Align2::CENTER_TOP,
                            "R",
                            FontId::proportional(9.0),
                            Color32::from_rgb(100, 150, 255),
                        );
                    }
                }

                // Draw Loop Region (if enabled) - before playhead
                if self.loop_enabled {
                    let loop_start_x = note_offset_x
                        + tick_to_x(self.loop_start_tick, self.zoom_x, self.state.ticks_per_beat);
                    let loop_end_x = note_offset_x
                        + tick_to_x(self.loop_end_tick, self.zoom_x, self.state.ticks_per_beat);
                    
                    if loop_end_x > rect.min.x + key_width && loop_start_x < rect.max.x {
                        let loop_rect = Rect::from_min_max(
                            Pos2::new(loop_start_x.max(rect.min.x + key_width), rect.min.y),
                            Pos2::new(loop_end_x.min(rect.max.x), rect.max.y),
                        );
                        // Semi-transparent overlay
                        painter.rect_filled(
                            loop_rect,
                            0.0,
                            Color32::from_rgba_unmultiplied(100, 150, 255, 60),
                        );
                        // Loop boundaries
                        if loop_start_x >= rect.min.x + key_width {
                            painter.line_segment(
                                [Pos2::new(loop_start_x, rect.min.y), Pos2::new(loop_start_x, rect.max.y)],
                                Stroke::new(2.0, Color32::from_rgb(100, 150, 255)),
                            );
                        }
                        if loop_end_x <= rect.max.x {
                            painter.line_segment(
                                [Pos2::new(loop_end_x, rect.min.y), Pos2::new(loop_end_x, rect.max.y)],
                                Stroke::new(2.0, Color32::from_rgb(100, 150, 255)),
                            );
                        }
                    }
                }

                // Draw Playhead (Timeline portion + Line)
                // Drawn AFTER notes (so it's on top of notes)
                let playhead_x = note_offset_x
                    + time_to_x(
                        (self.current_time * self.state.bpm / 60.0) as f32,
                        self.zoom_x,
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

                // Draw Piano Keys (Sidebar) - Drawn LAST so they cover playhead and notes
                // Fill background for sidebar to occlude content
                let sidebar_rect = Rect::from_min_size(
                    Pos2::new(rect.min.x, rect.min.y + timeline_height),
                    Vec2::new(key_width, rect.height() - timeline_height),
                );
                painter.rect_filled(sidebar_rect, 0.0, ui.visuals().window_fill());

                for i in 0..=127 {
                    let y = rect.min.y
                        + timeline_height
                        + note_to_y((127 - i) as u8, self.zoom_y)
                        + self.manual_scroll_y;

                    // Only draw if visible
                    if y > rect.min.y + timeline_height && y < rect.max.y {
                        let note_idx = 127 - i;
                        let is_black = [1, 3, 6, 8, 10].contains(&(note_idx % 12));
                        let key_color = if is_black {
                            Color32::BLACK
                        } else {
                            Color32::WHITE
                        };
                        let text_color = if is_black {
                            Color32::WHITE
                        } else {
                            Color32::BLACK
                        };

                        let key_rect = Rect::from_min_size(
                            Pos2::new(rect.min.x, y),
                            Vec2::new(key_width, self.zoom_y),
                        );

                        painter.rect_filled(key_rect, 0.0, key_color);
                        painter.rect_stroke(key_rect, 0.0, Stroke::new(1.0, Color32::GRAY));

                        // C notes label
                        if note_idx >= 12 && (note_idx - 12) % 12 == 0 {
                            painter.text(
                                key_rect.left_center() + Vec2::new(2.0, 0.0),
                                Align2::LEFT_CENTER,
                                format!("C{}", (note_idx / 12) as i32 - 1),
                                FontId::proportional(10.0),
                                text_color,
                            );
                        } else if note_idx < 12 && note_idx % 12 == 0 {
                            painter.text(
                                key_rect.left_center() + Vec2::new(2.0, 0.0),
                                Align2::LEFT_CENTER,
                                format!("C{}", (note_idx as i32 / 12) - 1),
                                FontId::proportional(10.0),
                                text_color,
                            );
                        }

                        // Interaction: Click Key to preview
                        if ui.rect_contains_pointer(key_rect) {
                            if ui.input(|i| i.pointer.primary_pressed()) {
                                self.active_key_note = Some(note_idx);
                                if let Some(playback) = &self.playback {
                                    playback.note_on(note_idx, 100);
                                }
                            }
                        }
                    }
                }
            });
    }

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

    fn note_index_by_id(&self, id: NoteId) -> Option<usize> {
        self.state.notes.iter().position(|n| n.id == id)
    }

    fn sort_notes(&mut self) {
        self.state
            .notes
            .sort_by(|a, b| a.start.cmp(&b.start).then_with(|| a.id.0.cmp(&b.id.0)));
    }

    fn push_undo_snapshot(&mut self) {
        const MAX_HISTORY: usize = 64;
        self.undo_stack.push(self.state.clone());
        if self.undo_stack.len() > MAX_HISTORY {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    fn note_mut_by_id(&mut self, id: NoteId) -> Option<&mut Note> {
        let idx = self.note_index_by_id(id)?;
        self.state.notes.get_mut(idx)
    }

    fn note_by_id(&self, id: NoteId) -> Option<Note> {
        self.state.notes.iter().copied().find(|n| n.id == id)
    }

    fn first_selected_note(&self) -> Option<Note> {
        self.selected_notes
            .iter()
            .next()
            .and_then(|id| self.note_by_id(*id))
    }

    fn selected_notes_snapshot(&self) -> Vec<Note> {
        self.selected_notes
            .iter()
            .filter_map(|id| self.note_by_id(*id))
            .collect()
    }

    fn edit_note_by_id<F>(&mut self, id: NoteId, mut edit: F)
    where
        F: FnMut(&mut Note),
    {
        if let Some(idx) = self.note_index_by_id(id) {
            self.push_undo_snapshot();
            let before = self.state.notes[idx];
            edit(&mut self.state.notes[idx]);
            self.state.notes[idx].duration = self.state.notes[idx].duration.max(1);
            let after = self.state.notes[idx];
            self.sort_notes();
            self.emit_note_updated(before, after);
        }
    }

    fn copy_selection(&mut self) {
        self.clipboard = self.selected_notes_snapshot();
        self.clipboard.sort_by_key(|n| n.start);
    }

    fn cut_selection(&mut self) {
        if self.selected_notes.is_empty() {
            return;
        }
        self.copy_selection();
        let ids: Vec<_> = self.selected_notes.iter().copied().collect();
        self.remove_notes(ids);
    }

    fn paste_clipboard_at(&mut self, target_tick: u64) {
        if self.clipboard.is_empty() {
            return;
        }
        let min_start = self
            .clipboard
            .iter()
            .map(|n| n.start)
            .min()
            .unwrap_or(target_tick);
        let offset = target_tick.saturating_sub(min_start);
        let templates = self.clipboard.clone();
        self.push_undo_snapshot();
        let previous = self.selected_notes.clone();
        self.selected_notes.clear();
        for template in templates {
            let new_note = Note::new(
                template.start + offset,
                template.duration,
                template.key,
                template.velocity,
            );
            self.state.notes.push(new_note);
            self.emit_note_added(new_note);
            self.selected_notes.insert(new_note.id);
        }
        self.sort_notes();
        self.notify_selection_changed(previous);
    }

    fn delete_selected_notes(&mut self) {
        if self.selected_notes.is_empty() {
            return;
        }
        let ids: Vec<_> = self.selected_notes.iter().copied().collect();
        self.remove_notes(ids);
    }

    fn quantize_selected_notes(&mut self) {
        if self.selected_notes.is_empty() || self.snap_interval == 0 {
            return;
        }
        self.push_undo_snapshot();
        let ids: Vec<_> = self.selected_notes.iter().copied().collect();
        for id in ids {
            let start_tick = self.note_by_id(id).map(|n| n.start).unwrap_or(0);
            let snapped = self.snap_tick(start_tick as i64, None, false);
            if let Some((before, after)) = self.note_mut_by_id(id).map(|note| {
                let before = *note;
                note.start = snapped;
                let after = *note;
                (before, after)
            }) {
                self.emit_note_updated(before, after);
            }
        }
        self.sort_notes();
    }

    #[allow(dead_code)]
    fn apply_swing_to_selected_notes(&mut self, swing_ratio: f32) {
        if self.selected_notes.is_empty() || swing_ratio <= 0.0 {
            return;
        }
        
        self.push_undo_snapshot();
        // Save original positions
        self.swing_original_notes = self.selected_notes
            .iter()
            .filter_map(|&id| {
                self.note_by_id(id).map(|note| (id, note.start))
            })
            .collect();
        
        self.apply_swing_to_selected_notes_realtime(swing_ratio);
    }

    fn apply_swing_to_selected_notes_realtime(&mut self, swing_ratio: f32) {
        if self.selected_notes.is_empty() || self.swing_original_notes.is_empty() {
            return;
        }
        
        let tpb = self.state.ticks_per_beat as u64;
        let delay_ticks = (tpb as f32 * 0.5 * swing_ratio) as u64;
        
        // Clone original notes to avoid borrow checker issues
        let original_notes = self.swing_original_notes.clone();
        
        // Restore original positions first, then apply swing
        for (id, original_start) in &original_notes {
            if let Some(note) = self.note_mut_by_id(*id) {
                note.start = *original_start;
            }
        }
        
        // Now apply swing based on original positions
        for (id, original_start) in &original_notes {
            if let Some((before, after)) = self.note_mut_by_id(*id).map(|note| {
                let before = *note;
                let beat_position = original_start / tpb;
                let is_even_beat = (beat_position % 2) == 1;
                
                if is_even_beat {
                    note.start = original_start.saturating_add(delay_ticks);
                } else {
                    note.start = *original_start;
                }
                let after = *note;
                (before, after)
            }) {
                if before.start != after.start {
                    self.emit_note_updated(before, after);
                }
            }
        }
        
        self.sort_notes();
        self.emit_state_replaced();
    }

    #[allow(dead_code)]
    fn note_near_tick(&self, tick: u64) -> Option<Note> {
        self.state.notes.iter().copied().min_by_key(|note| {
            if tick >= note.start && tick <= note.start + note.duration {
                0
            } else if tick < note.start {
                (note.start - tick) as u64
            } else {
                (tick - (note.start + note.duration)) as u64
            }
        })
    }

    #[allow(dead_code)]
    fn begin_lane_edit(&mut self, lane: LaneType, targets: Vec<NoteId>, anchor: NoteId) {
        let mut uniques = BTreeSet::new();
        let mut originals = Vec::new();
        for id in targets {
            if uniques.insert(id) {
                if let Some(idx) = self.note_index_by_id(id) {
                    originals.push((id, self.state.notes[idx]));
                }
            }
        }
        if originals.is_empty() {
            return;
        }
        self.push_undo_snapshot();
        self.lane_edit_state = Some(LaneEditState {
            lane,
            anchor,
            originals,
        });
        self.lane_edit_changed = false;
    }

    #[allow(dead_code)]
    fn apply_lane_velocity(&mut self, value: u8, relative: bool) {
        let originals = self.lane_edit_state.as_ref().map(|s| s.originals.clone());
        if let Some(state) = &self.lane_edit_state {
            let anchor_original = state
                .originals
                .iter()
                .find(|(id, _)| *id == state.anchor)
                .map(|(_, note)| *note);
            if let Some(anchor) = anchor_original {
                let delta = value as i32 - anchor.velocity as i32;
                if let Some(originals) = originals {
                    for (id, original) in &originals {
                        if let Some(note) = self.note_mut_by_id(*id) {
                            let mut new_velocity = if relative {
                                (original.velocity as i32 + delta).clamp(1, 127) as u8
                            } else {
                                value
                            };
                            new_velocity = new_velocity.clamp(1, 127);
                            if note.velocity != new_velocity {
                                note.velocity = new_velocity;
                                self.lane_edit_changed = true;
                            }
                        }
                    }
                }
            }
        }
    }

    #[allow(dead_code)]
    fn apply_lane_pitch(&mut self, value: u8, relative: bool) {
        let originals = self.lane_edit_state.as_ref().map(|s| s.originals.clone());
        if let Some(state) = &self.lane_edit_state {
            let anchor_original = state
                .originals
                .iter()
                .find(|(id, _)| *id == state.anchor)
                .map(|(_, note)| *note);
            if let Some(anchor) = anchor_original {
                let delta = value as i32 - anchor.key as i32;
                if let Some(originals) = originals {
                    for (id, original) in &originals {
                        if let Some(note) = self.note_mut_by_id(*id) {
                            let mut new_key = if relative {
                                (original.key as i32 + delta).clamp(0, 127) as u8
                            } else {
                                value
                            };
                            new_key = new_key.clamp(0, 127);
                            if note.key != new_key {
                                note.key = new_key;
                                self.lane_edit_changed = true;
                            }
                        }
                    }
                }
            }
        }
    }

    fn current_tick_position(&self) -> u64 {
        if self.state.ticks_per_beat == 0 {
            return 0;
        }
        let seconds_per_beat = 60.0 / self.state.bpm.max(1.0);
        let seconds_per_tick = seconds_per_beat / self.state.ticks_per_beat as f32;
        (self.current_time / seconds_per_tick).max(0.0) as u64
    }

    fn notify_playback_started(&self) {
        if let Some(observer) = &self.playback_observer {
            observer.on_playback_started();
        }
    }

    fn notify_playback_stopped(&self) {
        if let Some(observer) = &self.playback_observer {
            observer.on_playback_stopped();
        }
    }

    fn handle_shortcuts(&mut self, ctx: &Context) {
        let command = ctx.input(|i| i.modifiers.command);
        let shift = ctx.input(|i| i.modifiers.shift);
        if command && ctx.input(|i| i.key_pressed(Key::C)) {
            self.copy_selection();
        }
        if command && ctx.input(|i| i.key_pressed(Key::X)) {
            self.cut_selection();
        }
        if command && ctx.input(|i| i.key_pressed(Key::V)) {
            let tick = self.current_tick_position();
            self.paste_clipboard_at(tick);
        }
        if ctx.input(|i| i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace)) {
            self.delete_selected_notes();
        }
        if command && ctx.input(|i| i.key_pressed(Key::Z)) {
            if shift {
                self.redo();
            } else {
                self.undo();
            }
        } else if command && ctx.input(|i| i.key_pressed(Key::Y)) {
            self.redo();
        }
    }

    fn delete_note_by_id(&mut self, id: NoteId) {
        if let Some(idx) = self.note_index_by_id(id) {
            self.push_undo_snapshot();
            let removed = self.state.notes.remove(idx);
            self.emit_note_deleted(removed);
            self.selected_notes.remove(&removed.id);
        }
    }

    fn handle_note_click(&mut self, note_id: NoteId, modifiers: Modifiers) {
        if modifiers.command {
            self.toggle_selection(note_id);
        } else if modifiers.shift {
            self.extend_selection(note_id);
        } else {
            self.set_single_selection(note_id);
        }
    }

    fn prepare_selection_for_drag(&mut self, note_id: NoteId, modifiers: Modifiers) {
        if modifiers.command {
            if !self.selected_notes.remove(&note_id) {
                self.selected_notes.insert(note_id);
            }
        } else if modifiers.shift {
            self.extend_selection(note_id);
        } else if !self.selected_notes.contains(&note_id) {
            self.set_single_selection(note_id);
        }
    }

    fn resolve_drag_action(&self, pointer: Pos2, rect: Rect) -> DragAction {
        const HANDLE_WIDTH: f32 = 6.0;
        let left_handle =
            Rect::from_min_max(rect.min, Pos2::new(rect.min.x + HANDLE_WIDTH, rect.max.y));
        let right_handle =
            Rect::from_min_max(Pos2::new(rect.max.x - HANDLE_WIDTH, rect.min.y), rect.max);
        if left_handle.contains(pointer) {
            DragAction::ResizeStart
        } else if right_handle.contains(pointer) {
            DragAction::ResizeEnd
        } else {
            DragAction::Move
        }
    }

    fn begin_note_drag(
        &mut self,
        anchor: NoteId,
        pointer: Pos2,
        pointer_tick: i64,
        action: DragAction,
    ) {
        if self.selected_notes.is_empty() {
            self.set_single_selection(anchor);
        }
        self.push_undo_snapshot();
        self.is_dragging_note = true;
        self.is_resizing_note = matches!(action, DragAction::ResizeStart | DragAction::ResizeEnd);
        self.drag_action = action;
        self.drag_start_pos = Some(pointer);
        self.drag_primary_anchor = Some(anchor);
        self.drag_original_notes = self
            .selected_notes
            .iter()
            .filter_map(|id| {
                self.note_index_by_id(*id)
                    .map(|idx| (*id, self.state.notes[idx]))
            })
            .collect();
        if let Some(note) = self.state.notes.iter().find(|n| n.id == anchor) {
            self.drag_original_start = Some(note.start);
            self.drag_original_duration = Some(note.duration);
            self.drag_original_key = Some(note.key);
        }
        self.drag_pointer_offset_ticks =
            Some(pointer_tick - self.drag_original_start.unwrap_or(0) as i64);
        self.drag_changed_note = false;
        if matches!(action, DragAction::Move | DragAction::None) {
            if let Some(note) = self.state.notes.iter().find(|n| n.id == anchor) {
                self.preview_note_on(note.key, 100);
            }
        }
    }

    fn create_note_at_pointer<F, G>(&mut self, pointer: Pos2, to_tick: F, to_key: G)
    where
        F: Fn(Pos2) -> i64,
        G: Fn(Pos2) -> u8,
    {
        let start_tick = to_tick(pointer).max(0);
        let snapped_start = self.snap_tick(start_tick, None, false);
        let default_duration = if self.snap_interval > 0 {
            self.snap_interval
        } else {
            self.state.ticks_per_beat as u64
        }
        .max(1);
        let key = to_key(pointer);
        let note = Note::new(snapped_start, default_duration, key, 100);
        self.push_undo_snapshot();
        self.state.notes.push(note);
        self.sort_notes();
        self.emit_note_added(note);
        self.set_single_selection(note.id);
        self.drag_primary_anchor = Some(note.id);
        self.drag_original_notes = vec![(note.id, note)];
        self.drag_original_start = Some(note.start);
        self.drag_original_duration = Some(note.duration);
        self.drag_original_key = Some(note.key);
        self.drag_pointer_offset_ticks = Some(0);
        self.is_dragging_note = true;
        self.drag_action = DragAction::Create;
        self.drag_start_pos = Some(pointer);
        self.preview_note_on(note.key, 100);
    }

    fn update_note_drag<F, G>(&mut self, pointer: Pos2, to_tick: F, to_key: G, modifiers: Modifiers)
    where
        F: Fn(Pos2) -> i64,
        G: Fn(Pos2) -> u8,
    {
        let originals_snapshot = self.drag_original_notes.clone();
        let pointer_tick = to_tick(pointer);
        let disable_snap = modifiers.alt;
        match self.drag_action {
            DragAction::Move | DragAction::None => {
                let key = to_key(pointer);
                let anchor_id = match self.drag_primary_anchor {
                    Some(id) => id,
                    None => return,
                };
                let anchor_original = match originals_snapshot
                    .iter()
                    .find(|(id, _)| *id == anchor_id)
                    .map(|(_, note)| *note)
                {
                    Some(note) => note,
                    None => return,
                };
                let offset = self.drag_pointer_offset_ticks.unwrap_or(0);
                let snapped = self.snap_tick(
                    pointer_tick - offset,
                    Some(anchor_original.start),
                    disable_snap,
                );
                let delta = snapped as i64 - anchor_original.start as i64;
                let key_delta = key as i16 - anchor_original.key as i16;
                for (id, original) in &originals_snapshot {
                    let mut preview = None;
                    if let Some(note) = self.note_mut_by_id(*id) {
                        let new_start = (original.start as i64 + delta).max(0) as u64;
                        let new_key = (original.key as i16 + key_delta).clamp(0, 127) as u8;
                        let should_preview = note.key != new_key && *id == anchor_id;
                        if note.start != new_start || note.key != new_key {
                            note.start = new_start;
                            note.key = new_key;
                            self.drag_changed_note = true;
                        }
                        if should_preview {
                            preview = Some(new_key);
                        }
                    }
                    if let Some(key) = preview {
                        self.preview_note_on(key, 100);
                    }
                }
                self.sort_notes();
            }
            DragAction::ResizeStart => {
                if let Some(anchor_id) = self.drag_primary_anchor {
                    if let Some(original) = originals_snapshot
                        .iter()
                        .find(|(id, _)| *id == anchor_id)
                        .map(|(_, note)| *note)
                    {
                        let snapped =
                            self.snap_tick(pointer_tick, Some(original.start), disable_snap);
                        let end = original.start + original.duration;
                        let new_start = snapped.min(end.saturating_sub(1));
                        if let Some(note) = self.note_mut_by_id(anchor_id) {
                            if new_start != note.start
                                || note.duration != end.saturating_sub(new_start)
                            {
                                note.start = new_start;
                                note.duration = end.saturating_sub(new_start).max(1);
                                self.drag_changed_note = true;
                            }
                        }
                    }
                }
            }
            DragAction::ResizeEnd => {
                if let Some(anchor_id) = self.drag_primary_anchor {
                    if let Some(original) = originals_snapshot
                        .iter()
                        .find(|(id, _)| *id == anchor_id)
                        .map(|(_, note)| *note)
                    {
                        let snapped = self.snap_tick(
                            pointer_tick,
                            Some(original.start + original.duration),
                            disable_snap,
                        );
                        if let Some(note) = self.note_mut_by_id(anchor_id) {
                            let new_end = snapped.max(note.start + 1);
                            if new_end != note.start + note.duration {
                                note.duration = new_end.saturating_sub(note.start).max(1);
                                self.drag_changed_note = true;
                            }
                        }
                    }
                }
            }
            DragAction::Create => {
                if let Some(anchor_id) = self.drag_primary_anchor {
                    if let Some(original) = originals_snapshot
                        .iter()
                        .find(|(id, _)| *id == anchor_id)
                        .map(|(_, note)| *note)
                    {
                        let snapped = self.snap_tick(
                            pointer_tick,
                            Some(original.start + original.duration),
                            disable_snap,
                        );
                        let new_end = snapped.max(original.start + 1);
                        let new_key = to_key(pointer);
                        let mut preview = None;
                        if let Some(note) = self.note_mut_by_id(anchor_id) {
                            if new_end != note.start + note.duration {
                                note.duration = new_end - note.start;
                            }
                            if new_key != note.key {
                                note.key = new_key;
                                preview = Some(new_key);
                            }
                            self.drag_changed_note = true;
                        }
                        if let Some(key) = preview {
                            self.preview_note_on(key, 100);
                        }
                    }
                }
            }
            DragAction::LoopEdit | DragAction::PlayheadSeek => {
                // Loop editing and playhead seeking - handled in ui_piano_roll interaction code
                return;
            }
        }
    }

    fn snap_value(&self, value: i64) -> i64 {
        if self.snap_interval == 0 {
            return value;
        }
        let interval = self.snap_interval as i64;
        let remainder = value.rem_euclid(interval);
        if remainder >= interval / 2 {
            value + (interval - remainder)
        } else {
            value - remainder
        }
    }

    fn snap_tick(&self, raw_tick: i64, reference: Option<u64>, disable: bool) -> u64 {
        if self.snap_interval == 0 || disable {
            return raw_tick.max(0) as u64;
        }
        match (self.snap_mode, reference) {
            (SnapMode::Relative, Some(original)) => {
                let delta = raw_tick - original as i64;
                let snapped_delta = self.snap_value(delta);
                (original as i64 + snapped_delta).max(0) as u64
            }
            _ => self.snap_value(raw_tick).max(0) as u64,
        }
    }

    fn ui_curve_lanes(&mut self, ui: &mut Ui) {
        // Find velocity curve lane ID and clone data
        let velocity_lane_id = self.state.curves.iter()
            .find(|c| c.lane_type == CurveLaneType::Velocity)
            .map(|c| c.id);
        
            if let Some(lane_id) = velocity_lane_id {
                let key_width = 60.0; // Same as piano roll (for grid alignment calculation)
                let tpb = self.state.ticks_per_beat.max(1) as u64;
                let manual_scroll_x = self.manual_scroll_x;
                let zoom_x = self.zoom_x;
                let available_height = ui.available_height();
                
                // Clone points and lane info for rendering
                let points_clone: Vec<_> = self.state.curves.iter()
                    .find(|c| c.id == lane_id)
                    .map(|c| c.points.clone())
                    .unwrap_or_default();
                let (min_val, max_val) = CurveLaneType::Velocity.value_range();
                let value_range = max_val - min_val;
                let dragging = self.dragging_curve_point;
                
                let mut point_to_delete: Option<CurvePointId> = None;
                let mut point_to_start_drag: Option<CurvePointId> = None;
                let mut new_point: Option<(u64, f32)> = None;
                
                // Curve editing area - full width, extends to window edges
                ui.push_id("curve_editor_scroll", |ui| {
                    ScrollArea::horizontal()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                        let available_width = ui.available_width();
                        let (rect, response) = ui.allocate_exact_size(
                            Vec2::new(available_width.max(400.0), available_height),
                            Sense::click_and_drag()
                        );
                        
                        let painter = ui.painter_at(rect);
                        
                        // Draw background
                        painter.rect_filled(rect, 0.0, Color32::from_rgb(40, 40, 40));
                        
                        // Draw grid lines using EXACT same logic as piano roll
                        // To align grids, we need to calculate note_offset_x the same way as piano roll
                        // Piano roll: note_offset_x = rect.min.x + key_width + manual_scroll_x
                        // But piano roll's rect.min.x includes the key area, while our rect.min.x is at window edge
                        // So we need to offset by key_width to align: rect.min.x + key_width + manual_scroll_x
                        // This ensures grid lines align perfectly even though curve editor extends to edges
                        let note_offset_x = rect.min.x + key_width + manual_scroll_x;
                        
                        let denom = self.state.time_signature.1.max(1) as u64;
                        let numer = self.state.time_signature.0.max(1) as u64;
                        let ticks_per_measure = (tpb * numer * 4).saturating_div(denom).max(tpb);
                        
                        // Calculate visible range - allow showing tick 0 even if it's slightly to the left
                        let visible_beats_start = (-manual_scroll_x / zoom_x).floor();
                        let visible_beats_end = visible_beats_start + (rect.width() / zoom_x) + 2.0;
                        // Allow rendering from tick 0 even if it's slightly outside visible range
                        let render_start_beats = visible_beats_start.min(0.0);
                        let mut start_tick = (render_start_beats * tpb as f32).floor() as i64;
                        if start_tick < 0 {
                            start_tick = 0;
                        }
                        let end_tick = (visible_beats_end * tpb as f32).ceil() as i64;
                        
                        let subdivision = if zoom_x >= 220.0 {
                            8
                        } else if zoom_x >= 90.0 {
                            4
                        } else if zoom_x >= 45.0 {
                            2
                        } else {
                            1
                        };
                        let tick_step = (tpb / subdivision).max(1);
                        
                        let mut tick = (start_tick / tick_step as i64) * tick_step as i64;
                        if tick < 0 {
                            tick = 0;
                        }
                        
                        let grid_top = rect.min.y;
                        let grid_bottom = rect.max.y;
                        
                        let measure_line_color = Color32::from_rgb(210, 210, 210);
                        let beat_line_color = Color32::from_rgb(140, 140, 140);
                        let subdivision_color = Color32::from_rgb(90, 90, 90);
                        
                        // Allow rendering slightly to the left to show tick 0 grid line
                        let render_left_bound = rect.min.x - 10.0;
                        while tick <= end_tick {
                            let x = note_offset_x + (tick as f32 / tpb as f32) * zoom_x;
                            if x >= render_left_bound && x <= rect.max.x {
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
                        
                        // Draw horizontal grid (value lines)
                        for i in 0..=4 {
                            let y = rect.min.y + (rect.height() / 4.0) * i as f32;
                            painter.line_segment(
                                [Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)],
                                Stroke::new(1.0, Color32::from_rgb(50, 50, 50)),
                            );
                        }
                        
                        // Draw curve line
                        if points_clone.len() >= 2 {
                            let mut points_vec = Vec::new();
                            for point in &points_clone {
                                let x = note_offset_x + (point.tick as f32 / tpb as f32) * zoom_x;
                                let normalized_value = (point.value - min_val) / value_range;
                                let y = rect.max.y - normalized_value * rect.height();
                                if x >= rect.min.x - 10.0 && x <= rect.max.x + 10.0 {
                                    points_vec.push(Pos2::new(x, y));
                                }
                            }
                            if points_vec.len() >= 2 {
                                for i in 0..points_vec.len() - 1 {
                                    painter.line_segment(
                                        [points_vec[i], points_vec[i + 1]],
                                        Stroke::new(2.0, Color32::from_rgb(100, 200, 100)),
                                    );
                                }
                            }
                        }
                        
                        // Draw curve points and handle interactions
                        for point in &points_clone {
                            let x = note_offset_x + (point.tick as f32 / tpb as f32) * zoom_x;
                            let normalized_value = (point.value - min_val) / value_range;
                            let y = rect.max.y - normalized_value * rect.height();
                            
                            if x >= rect.min.x - 5.0 && x <= rect.max.x + 5.0 {
                                let point_pos = Pos2::new(x, y);
                                let point_rect = Rect::from_center_size(point_pos, Vec2::new(8.0, 8.0));
                                painter.circle_filled(point_pos, 4.0, Color32::from_rgb(150, 250, 150));
                                painter.circle_stroke(point_pos, 4.0, Stroke::new(1.0, Color32::WHITE));
                                
                                // Handle point interactions
                                if response.clicked_by(PointerButton::Primary) {
                                    if let Some(pointer) = response.interact_pointer_pos() {
                                        if point_rect.contains(pointer) {
                                            point_to_start_drag = Some(point.id);
                                        }
                                    }
                                }
                                
                                if response.clicked_by(PointerButton::Secondary) {
                                    if let Some(pointer) = response.interact_pointer_pos() {
                                        if point_rect.contains(pointer) && points_clone.len() > 1 {
                                            point_to_delete = Some(point.id);
                                        }
                                    }
                                }
                            }
                        }
                        
                        // Handle dragging
                        if let Some((drag_lane_id, drag_point_id)) = dragging {
                            if drag_lane_id == lane_id && ui.input(|i| i.pointer.primary_down()) {
                                if let Some(pointer) = response.interact_pointer_pos() {
                                    // Convert pointer position to tick
                                    // note_offset_x = rect.min.x + key_width + manual_scroll_x
                                    // So: pointer.x - note_offset_x = pointer.x - rect.min.x - key_width - manual_scroll_x
                                    // This gives us the position relative to tick 0
                                    let rel_x = pointer.x - note_offset_x;
                                    let beats = rel_x / zoom_x;
                                    let tick = (beats * tpb as f32).round() as i64;
                                    // Only allow dragging to tick >= 0
                                    if tick >= 0 {
                                        let disable_snap = ui.input(|i| i.modifiers.alt);
                                        let snapped_tick = if disable_snap {
                                            tick as u64
                                        } else {
                                            self.snap_value(tick).max(0) as u64
                                        };
                                        
                                        let rel_y = pointer.y - rect.min.y;
                                        let normalized_value = 1.0 - (rel_y / rect.height());
                                        let value = min_val + normalized_value * value_range;
                                        
                                        if let Some(lane) = self.state.curves.iter_mut().find(|c| c.id == drag_lane_id) {
                                            lane.update_point(drag_point_id, snapped_tick, value);
                                            self.emit_event(EditorEvent::CurvePointUpdated {
                                                lane_id: drag_lane_id,
                                                point_id: drag_point_id,
                                            });
                                        }
                                    }
                                }
                            } else {
                                self.dragging_curve_point = None;
                            }
                        }
                        
                        // Handle adding new point
                        if response.clicked_by(PointerButton::Primary) && dragging.is_none() && point_to_start_drag.is_none() {
                            if let Some(pointer) = response.interact_pointer_pos() {
                                if rect.contains(pointer) {
                                    // Convert pointer position to tick
                                    // note_offset_x = rect.min.x + key_width + manual_scroll_x
                                    let rel_x = pointer.x - note_offset_x;
                                    let beats = rel_x / zoom_x;
                                    let tick = (beats * tpb as f32).round() as i64;
                                    // Only allow adding points at tick >= 0
                                    if tick >= 0 {
                                        let disable_snap = ui.input(|i| i.modifiers.alt);
                                        let snapped_tick = if disable_snap {
                                            tick as u64
                                        } else {
                                            self.snap_value(tick).max(0) as u64
                                        };
                                        
                                        let rel_y = pointer.y - rect.min.y;
                                        let normalized_value = 1.0 - (rel_y / rect.height());
                                        let value = min_val + normalized_value * value_range;
                                        
                                        // Check if clicking near existing point
                                        let near_existing = points_clone.iter().any(|p| {
                                            let px = note_offset_x + (p.tick as f32 / tpb as f32) * zoom_x;
                                            let py = rect.max.y - ((p.value - min_val) / value_range) * rect.height();
                                            (pointer.x - px).abs() < 10.0 && (pointer.y - py).abs() < 10.0
                                        });
                                        
                                        if !near_existing {
                                            new_point = Some((snapped_tick, value));
                                        }
                                    }
                                }
                            }
                        }
                    });
                }); // Close push_id
                
                // Handle deletions and additions outside the closure
                if let Some(point_id) = point_to_delete {
                    self.push_undo_snapshot();
                    if let Some(lane) = self.state.curves.iter_mut().find(|c| c.id == lane_id) {
                        lane.remove_point(point_id);
                        self.emit_event(EditorEvent::CurvePointRemoved {
                            lane_id,
                            point_id,
                        });
                    }
                }
                
                if let Some((tick, value)) = new_point {
                    self.push_undo_snapshot();
                    if let Some(lane) = self.state.curves.iter_mut().find(|c| c.id == lane_id) {
                        let point = lane.insert_point(tick, value);
                        self.emit_event(EditorEvent::CurvePointAdded {
                            lane_id,
                            point_id: point.id,
                        });
                    }
                }
                
                if let Some(point_id) = point_to_start_drag {
                    self.push_undo_snapshot();
                    self.dragging_curve_point = Some((lane_id, point_id));
                }
            } else {
                ui.label("No velocity curve lane found");
            }
    }
}

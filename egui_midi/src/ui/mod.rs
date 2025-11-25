use crate::audio::PlaybackBackend;
use crate::structure::{MidiState, Note, NoteId};
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
}

#[derive(Clone, Debug)]
pub enum EditorEvent {
    NoteAdded(Note),
    NoteUpdated { before: Note, after: Note },
    NoteDeleted(Note),
    SelectionChanged(Vec<NoteId>),
    StateReplaced(MidiState),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TransportState {
    pub is_playing: bool,
    pub position_seconds: f32,
    pub bpm_override: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapMode {
    Absolute,
    Relative,
}

impl Default for SnapMode {
    fn default() -> Self {
        SnapMode::Absolute
    }
}

pub struct MidiEditor {
    pub state: MidiState,
    pub playback: Option<PlaybackHandle>,
    
    // View state
    pub scroll: Vec2,
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
    
    // Config
    pub snap_interval: u64, // Ticks (e.g., 480 for quarter note)
    pub snap_mode: SnapMode,
    pub swing_ratio: f32,
    pub volume: f32,
    pub loop_enabled: bool,
    pub loop_start_tick: u64,
    pub loop_end_tick: u64,

    // Integration
    pub transport_override: Option<TransportState>,
    pub pending_events: Vec<EditorEvent>,
    pub clipboard: Vec<Note>,
    pub undo_stack: Vec<MidiState>,
    pub redo_stack: Vec<MidiState>,
    pub drag_changed_note: bool,
}

impl MidiEditor {
    pub fn new(playback: Option<PlaybackHandle>) -> Self {
        Self::with_state(MidiState::default(), playback)
    }

    pub fn with_state(state: MidiState, playback: Option<PlaybackHandle>) -> Self {
        let loop_default = (state.ticks_per_beat as u64).saturating_mul(4).max(state.ticks_per_beat as u64);
        Self {
            state,
            playback,
            scroll: Vec2::ZERO,
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
            snap_interval: 120,
            snap_mode: SnapMode::Absolute,
            swing_ratio: 0.0,
            volume: 0.5,
            loop_enabled: false,
            loop_start_tick: 0,
            loop_end_tick: loop_default,
            transport_override: None,
            pending_events: Vec::new(),
            clipboard: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            drag_changed_note: false,
        }
    }

    pub fn replace_state(&mut self, state: MidiState) {
        self.state = state;
        self.selected_notes.clear();
        self.pending_events.push(EditorEvent::StateReplaced(self.state.clone()));
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
        self.pending_events
            .push(EditorEvent::StateReplaced(self.state.clone()));
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
        let existing = self.state.notes.clone();
        for note in existing {
            self.emit_note_deleted(note);
        }
        self.state.notes.clear();
        self.selected_notes.clear();
    }

    pub fn undo(&mut self) -> bool {
        if let Some(previous) = self.undo_stack.pop() {
            self.redo_stack.push(self.state.clone());
            self.state = previous;
            self.pending_events
                .push(EditorEvent::StateReplaced(self.state.clone()));
            self.selected_notes.clear();
            return true;
        }
        false
    }

    pub fn redo(&mut self) -> bool {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.state.clone());
            self.state = next;
            self.pending_events
                .push(EditorEvent::StateReplaced(self.state.clone()));
            self.selected_notes.clear();
            return true;
        }
        false
    }

    pub fn set_playback_backend(&mut self, backend: Option<PlaybackHandle>) {
        self.playback = backend;
    }

    pub fn take_events(&mut self) -> Vec<EditorEvent> {
        std::mem::take(&mut self.pending_events)
    }

    pub fn set_transport_state(&mut self, state: Option<TransportState>) {
        self.transport_override = state;
    }

    pub fn center_on_c4(&mut self) {
        // C4 is MIDI 60. 
        // Y is calculated as (127 - note) * zoom_y.
        // We want C4 to be in middle of viewport if possible.
        // We don't have viewport size here easily without UI context.
        // But we can set a default approximate scroll.
        // Default viewport height might be around 600px?
        // 127 - 60 = 67 keys from top. 
        // 67 * 20 = 1340px from top.
        // If we want 1340px to be in middle, we need to scroll down ~1000px?
        self.manual_scroll_y = -1000.0; 
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
        self.pending_events.push(EditorEvent::NoteAdded(note));
    }

    fn emit_note_deleted(&mut self, note: Note) {
        self.pending_events.push(EditorEvent::NoteDeleted(note));
    }

    fn emit_note_updated(&mut self, before: Note, after: Note) {
        if before != after {
            self.pending_events
                .push(EditorEvent::NoteUpdated { before, after });
        }
    }

    fn finalize_note_drag_if_needed(&mut self) {
        if self.drag_changed_note {
            let originals = self.drag_original_notes.clone();
            for (id, before) in originals {
                if let Some(idx) = self.find_note_index_by_id(id) {
                    let after = self.state.notes[idx];
                    self.emit_note_updated(before, after);
                }
            }
        }
        self.drag_original_notes.clear();
        self.drag_primary_anchor = None;
        self.drag_changed_note = false;
    }

    fn notify_selection_changed(&mut self, previous: BTreeSet<NoteId>) {
        if previous != self.selected_notes {
            self.pending_events.push(EditorEvent::SelectionChanged(
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
        ui.vertical(|ui| {
            self.ui_toolbar(ui);
            ui.separator();
            self.ui_piano_roll(ui);
        });
        
        // Handle playback logic
        if ui.input(|i| i.key_pressed(Key::Space)) {
            self.is_playing = !self.is_playing;
            if self.is_playing {
                self.last_update = ui.input(|i| i.time);
                let seconds_per_beat = 60.0 / self.state.bpm;
                let seconds_per_tick = seconds_per_beat / self.state.ticks_per_beat as f32;
                self.last_tick = (self.current_time / seconds_per_tick) as u64;
            } else if let Some(playback) = &self.playback {
                playback.all_notes_off();
            }
        }

        if self.is_playing {
            ui.ctx().request_repaint();
            let now = ui.input(|i| i.time);
            let dt = now - self.last_update;
            self.last_update = now;
            
            if dt > 0.0 && dt < 1.0 { // Avoid large jumps
                self.current_time += dt as f32;
                self.update_sequencer();
            }
        } else {
            self.last_update = ui.input(|i| i.time);
            // Update last_tick to match current_time so when we start playing we don't skip or retrigger weirdly
            // But if we scrub, we might want to silence notes.
        }
    }

    fn update_sequencer(&mut self) {
        if self.state.ticks_per_beat == 0 || self.state.bpm <= 0.0 { return; }

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
                    playback.note_on(note.key, note.velocity);
                }
                
                // Check for Note Off: end lies between last_tick and current_tick
                let end = note.start + note.duration;
                if end > self.last_tick && end <= current_tick {
                    playback.note_off(note.key);
                }
            }
        }
        
        self.last_tick = current_tick;
    }

    fn ui_toolbar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui.button(if self.is_playing { "⏸ Pause" } else { "▶ Play" }).clicked() {
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
                } else if let Some(playback) = &self.playback {
                    playback.all_notes_off();
                }
            }
            if ui.button("⏹ Stop").clicked() {
                self.is_playing = false;
                self.current_time = 0.0;
                self.last_tick = 0;
                if let Some(playback) = &self.playback {
                    playback.all_notes_off();
                }
            }
            
            ui.separator();
            
            ui.label("Sig:");
            ui.horizontal(|ui| {
                let mut numer = self.state.time_signature.0;
                let mut denom = self.state.time_signature.1;
                let numer_changed =
                    ui.add(DragValue::new(&mut numer).speed(0.1).range(1..=32)).changed();
                ui.label("/");
                let denom_changed =
                    ui.add(DragValue::new(&mut denom).speed(0.1).range(1..=32)).changed();
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
            
            ui.label("Snap:");
            let mut snap = self.snap_interval;
            let snap_label = if snap == 0 {
                "自由".to_owned()
            } else {
                format!("1/{}", (480 * 4 / snap).max(1))
            };
            ComboBox::from_id_salt("snap_combo")
                .selected_text(snap_label)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut snap, 480 * 4, "1/1");
                    ui.selectable_value(&mut snap, 480 * 2, "1/2");
                    ui.selectable_value(&mut snap, 480, "1/4");
                    ui.selectable_value(&mut snap, 240, "1/8");
                    ui.selectable_value(&mut snap, 120, "1/16");
                });
            if snap != self.snap_interval {
                self.set_snap_interval(snap);
            }
            ui.small("ALT 关闭吸附");

            ui.separator();
            ui.label("Snap模式:");
            ComboBox::from_id_salt("snap_mode")
                .selected_text(match self.snap_mode {
                    SnapMode::Absolute => "绝对",
                    SnapMode::Relative => "相对",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.snap_mode, SnapMode::Absolute, "绝对");
                    ui.selectable_value(&mut self.snap_mode, SnapMode::Relative, "相对");
                });
                
            ui.separator();
            ui.label("Vol:");
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
            if ui.button("⎌ Undo").clicked() {
                self.undo();
            }
            if ui.button("↻ Redo").clicked() {
                self.redo();
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
                let (rect, response) = ui.allocate_exact_size(available_size, Sense::click_and_drag());
                
                // Handle Zoom (Ctrl/Alt + Scroll)
                let scroll_delta = ui.input(|i| i.raw_scroll_delta);
                if scroll_delta != Vec2::ZERO {
                     if ui.input(|i| i.modifiers.ctrl) {
                         // Zoom X (Horizontal) around mouse pointer
                         if scroll_delta.y != 0.0 {
                             let old_zoom = self.zoom_x;
                             let new_zoom = (self.zoom_x * if scroll_delta.y > 0.0 { 1.1 } else { 0.9 }).clamp(10.0, 500.0);
                             
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
                            let new_zoom = (self.zoom_y * if scroll_delta.y > 0.0 { 1.1 } else { 0.9 }).clamp(5.0, 50.0);
                            
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
                
                // Handle playhead interaction (click or drag on timeline)
                if let Some(pointer) = response.interact_pointer_pos() {
                    let in_timeline = pointer.y < rect.min.y + timeline_height && pointer.x >= rect.min.x + key_width;
                    if in_timeline {
                        pointer_consumed = true;
                        let mut x = pointer.x - (rect.min.x + key_width);
                        x = (x - self.manual_scroll_x).max(0.0);
                        let beats = x / self.zoom_x;
                        if beats >= 0.0 && ui.input(|i| i.pointer.primary_down()) {
                            self.current_time = beats * 60.0 / self.state.bpm;
                            let seconds_per_beat = 60.0 / self.state.bpm;
                            let seconds_per_tick = seconds_per_beat / self.state.ticks_per_beat as f32;
                            self.last_tick = (self.current_time / seconds_per_tick) as u64;
                            self.is_dragging_note = false;
                            self.drag_action = DragAction::None;
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
                    let y = rect.min.y + timeline_height + note_to_y((127 - i) as u8, self.zoom_y) + self.manual_scroll_y;
                    
                    // Only draw if visible (and maybe clip)
                    if y > rect.min.y + timeline_height && y < rect.max.y {
                        painter.line_segment(
                            [Pos2::new(rect.min.x + key_width, y), Pos2::new(rect.max.x, y)],
                            Stroke::new(1.0, horizontal_line_color)
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

                // Draw Notes
                let mut note_to_delete: Option<NoteId> = None;
                let note_offset_y = rect.min.y + timeline_height + self.manual_scroll_y;
                let notes_snapshot = self.state.notes.clone();

                for note in &notes_snapshot {
                    let x = note_offset_x + tick_to_x(note.start, self.zoom_x, self.state.ticks_per_beat);
                    let y = note_offset_y + note_to_y(note.key, self.zoom_y);
                    let w = tick_to_x(note.duration, self.zoom_x, self.state.ticks_per_beat).max(5.0);
                    let h = self.zoom_y;
                    let note_rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h));

                    if !note_rect.intersects(rect) {
                        continue;
                    }

                    let is_selected = self.selected_notes.contains(&note.id);
                    let color = if is_selected {
                        Color32::from_rgb(150, 250, 150)
                    } else {
                        Color32::from_rgb(100, 200, 100)
                    };
                    painter.rect_filled(note_rect.shrink(1.0), 2.0, color);
                    painter.rect_stroke(note_rect.shrink(1.0), 2.0, Stroke::new(1.0, Color32::WHITE));

                    if response.clicked_by(PointerButton::Primary) {
                        if let Some(pointer) = response.interact_pointer_pos() {
                            if note_rect.contains(pointer) {
                                let modifiers = ui.input(|i| i.modifiers);
                                self.handle_note_click(note.id, modifiers);
                                pointer_consumed = true;
                            }
                        }
                    }

                    if !self.is_dragging_note && ui.input(|i| i.pointer.primary_pressed()) {
                        if let Some(pointer) = response.interact_pointer_pos() {
                            if note_rect.contains(pointer) {
                                let modifiers = ui.input(|i| i.modifiers);
                                self.prepare_selection_for_drag(note.id, modifiers);
                                let action = self.resolve_drag_action(pointer, note_rect);
                                let pointer_tick = pointer_to_tick(pointer);
                                self.begin_note_drag(note.id, pointer, pointer_tick, action);
                                pointer_consumed = true;
                            }
                        }
                    }

                    if let Some(pointer) = response.hover_pos() {
                        let action = self.resolve_drag_action(pointer, note_rect);
                        if matches!(action, DragAction::ResizeStart | DragAction::ResizeEnd)
                            && note_rect.contains(pointer)
                        {
                            ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
                        }
                    }

                    if ui.input(|i| i.pointer.secondary_pressed()) {
                        if let Some(pointer) = response.interact_pointer_pos() {
                            if note_rect.contains(pointer) {
                                note_to_delete = Some(note.id);
                                pointer_consumed = true;
                            }
                        }
                    }
                }

                if self.is_dragging_note && ui.input(|i| i.pointer.primary_down()) {
                    if let Some(pointer) = response.interact_pointer_pos() {
                        let modifiers = ui.input(|i| i.modifiers);
                        self.update_note_drag(pointer, &pointer_to_tick, &pointer_to_key, modifiers);
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
                        let in_roll = pointer.x > rect.min.x + key_width && pointer.y > rect.min.y + timeline_height;
                        if in_roll {
                            if ui.input(|i| i.modifiers.shift) {
                                self.selection_box_start = Some(pointer);
                                self.selection_box_end = Some(pointer);
                            } else if !self.is_dragging_note {
                                self.create_note_at_pointer(pointer, &pointer_to_tick, &pointer_to_key);
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
                            for note in &self.state.notes {
                                let x =
                                    note_offset_x + tick_to_x(note.start, self.zoom_x, self.state.ticks_per_beat);
                                let y = note_offset_y + note_to_y(note.key, self.zoom_y);
                                let w =
                                    tick_to_x(note.duration, self.zoom_x, self.state.ticks_per_beat).max(5.0);
                                let h = self.zoom_y;
                                let note_rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, h));
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

                if let Some(id) = note_to_delete {
                    self.delete_note_by_id(id);
                }

                // double-click creation deprecated
                
                // Draw Timeline (Top Bar) - Drawn AFTER Notes
                // Fill timeline background
                let timeline_rect = Rect::from_min_size(
                    rect.min,
                    Vec2::new(rect.width(), timeline_height)
                );
                painter.rect_filled(timeline_rect, 0.0, ui.visuals().window_fill());
                painter.line_segment(
                    [timeline_rect.left_bottom(), timeline_rect.right_bottom()],
                    Stroke::new(1.0, separator_color) // Separator line
                );

                // Draw Timeline Labels (per measure)
                let mut measure_tick = (start_tick as u64 / ticks_per_measure) * ticks_per_measure;
                while measure_tick as i64 <= end_tick {
                    let x = note_offset_x + (measure_tick as f32 / tpb as f32) * self.zoom_x;
                    if x >= rect.min.x + key_width - 5.0 && x <= rect.max.x {
                        painter.line_segment(
                            [Pos2::new(x, rect.min.y), Pos2::new(x, rect.min.y + timeline_height)],
                            Stroke::new(1.0, measure_line_color)
                        );
                        let measure_index = (measure_tick / ticks_per_measure) + 1;
                        painter.text(
                            Pos2::new(x + 4.0, rect.min.y + 15.0),
                            Align2::LEFT_CENTER,
                            format!("{}", measure_index),
                            FontId::proportional(11.0),
                            Color32::GRAY
                        );
                    }
                    measure_tick += ticks_per_measure;
                }

                // Draw Playhead (Timeline portion + Line)
                // Drawn AFTER notes (so it's on top of notes)
                let playhead_x = note_offset_x + time_to_x((self.current_time * self.state.bpm / 60.0) as f32, self.zoom_x);
                if playhead_x > rect.min.x + key_width {
                    painter.line_segment(
                        [Pos2::new(playhead_x, rect.min.y), Pos2::new(playhead_x, rect.max.y)],
                        Stroke::new(2.0, Color32::from_rgba_premultiplied(100, 200, 255, 128))
                    );
                }

                // Draw Piano Keys (Sidebar) - Drawn LAST so they cover playhead and notes
                // Fill background for sidebar to occlude content
                let sidebar_rect = Rect::from_min_size(
                    Pos2::new(rect.min.x, rect.min.y + timeline_height),
                    Vec2::new(key_width, rect.height() - timeline_height)
                );
                painter.rect_filled(sidebar_rect, 0.0, ui.visuals().window_fill());

                for i in 0..=127 {
                    let y = rect.min.y + timeline_height + note_to_y((127 - i) as u8, self.zoom_y) + self.manual_scroll_y;
                    
                    // Only draw if visible
                    if y > rect.min.y + timeline_height && y < rect.max.y {
                        let note_idx = 127 - i;
                        let is_black = [1, 3, 6, 8, 10].contains(&(note_idx % 12));
                        let key_color = if is_black { Color32::BLACK } else { Color32::WHITE };
                        let text_color = if is_black { Color32::WHITE } else { Color32::BLACK };
                        
                        let key_rect = Rect::from_min_size(
                            Pos2::new(rect.min.x, y),
                            Vec2::new(key_width, self.zoom_y)
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
                                text_color
                            );
                        } else if note_idx < 12 && note_idx % 12 == 0 {
                             painter.text(
                                key_rect.left_center() + Vec2::new(2.0, 0.0),
                                Align2::LEFT_CENTER,
                                format!("C{}", (note_idx as i32 / 12) - 1),
                                FontId::proportional(10.0),
                                text_color
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

    fn draw_dashed_vertical_line(
        painter: &Painter,
        x: f32,
        top: f32,
        bottom: f32,
        stroke: Stroke,
    ) {
        let dash_len = 2.0;
        let gap_len = 2.0;
        let mut y = top;
        while y < bottom {
            let next = (y + dash_len).min(bottom);
            painter.line_segment(
                [Pos2::new(x, y), Pos2::new(x, next)],
                stroke.clone(),
            );
            y += dash_len + gap_len;
        }
    }

    fn find_note_index_by_id(&self, id: NoteId) -> Option<usize> {
        self.state.notes.iter().position(|n| n.id == id)
    }

    fn sort_notes(&mut self) {
        self.state.notes
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

    fn note_index_by_id(&self, id: NoteId) -> Option<usize> {
        self.state.notes.iter().position(|note| note.id == id)
    }

    fn note_mut_by_id(&mut self, id: NoteId) -> Option<&mut Note> {
        let idx = self.note_index_by_id(id)?;
        self.state.notes.get_mut(idx)
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
        let left_handle = Rect::from_min_max(rect.min, Pos2::new(rect.min.x + HANDLE_WIDTH, rect.max.y));
        let right_handle = Rect::from_min_max(Pos2::new(rect.max.x - HANDLE_WIDTH, rect.min.y), rect.max);
        if left_handle.contains(pointer) {
            DragAction::ResizeStart
        } else if right_handle.contains(pointer) {
            DragAction::ResizeEnd
        } else {
            DragAction::Move
        }
    }

    fn begin_note_drag(&mut self, anchor: NoteId, pointer: Pos2, pointer_tick: i64, action: DragAction) {
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
            .filter_map(|id| self.note_index_by_id(*id).map(|idx| (*id, self.state.notes[idx])))
            .collect();
        if let Some(note) = self.state.notes.iter().find(|n| n.id == anchor) {
            self.drag_original_start = Some(note.start);
            self.drag_original_duration = Some(note.duration);
            self.drag_original_key = Some(note.key);
        }
        self.drag_pointer_offset_ticks = Some(pointer_tick - self.drag_original_start.unwrap_or(0) as i64);
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
                let snapped = self.snap_tick(pointer_tick - offset, Some(anchor_original.start), disable_snap);
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
                        let snapped = self.snap_tick(pointer_tick, Some(original.start), disable_snap);
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
}

use midly::{MetaMessage, Smf, TrackEventKind};
use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

static NOTE_ID_COUNTER: AtomicU64 = AtomicU64::new(1);
static CURVE_LANE_ID_COUNTER: AtomicU64 = AtomicU64::new(1);
static CURVE_POINT_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NoteId(pub u64);

impl NoteId {
    pub fn next() -> Self {
        NoteId(NOTE_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub struct Note {
    pub id: NoteId,
    pub start: u64,    // Absolute ticks
    pub duration: u64, // Ticks
    pub key: u8,       // MIDI note number (0-127)
    pub velocity: u8,  // 0-127
}

impl Note {
    pub fn new(start: u64, duration: u64, key: u8, velocity: u8) -> Self {
        Self::with_id(NoteId::next(), start, duration, key, velocity)
    }

    pub fn with_id(id: NoteId, start: u64, duration: u64, key: u8, velocity: u8) -> Self {
        Self {
            id,
            start,
            duration,
            key,
            velocity,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CurveLaneId(pub u64);

impl CurveLaneId {
    pub fn next() -> Self {
        CurveLaneId(CURVE_LANE_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CurvePointId(pub u64);

impl CurvePointId {
    pub fn next() -> Self {
        CurvePointId(CURVE_POINT_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurveLaneType {
    Velocity,
    Pitch,
}

impl CurveLaneType {
    pub fn value_range(&self) -> (f32, f32) {
        match self {
            CurveLaneType::Velocity => (0.0, 127.0),
            CurveLaneType::Pitch => (-12.0, 12.0),
        }
    }

    pub fn default_name(&self) -> &'static str {
        match self {
            CurveLaneType::Velocity => "Velocity",
            CurveLaneType::Pitch => "Pitch",
        }
    }
}

#[derive(Clone, Debug)]
pub struct CurvePoint {
    pub id: CurvePointId,
    pub tick: u64,
    pub value: f32,
}

impl CurvePoint {
    pub fn new(tick: u64, value: f32) -> Self {
        Self {
            id: CurvePointId::next(),
            tick,
            value,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CurveLane {
    pub id: CurveLaneId,
    pub lane_type: CurveLaneType,
    pub name: String,
    pub enabled: bool,
    pub points: Vec<CurvePoint>,
}

impl CurveLane {
    pub fn new(lane_type: CurveLaneType) -> Self {
        Self {
            id: CurveLaneId::next(),
            lane_type,
            name: lane_type.default_name().to_owned(),
            enabled: true,
            points: Vec::new(),
        }
    }

    pub fn insert_point(&mut self, tick: u64, value: f32) -> CurvePoint {
        let mut point = CurvePoint::new(tick, value);
        self.clamp_point(&mut point);
        self.points.push(point.clone());
        self.sort_points();
        point
    }

    pub fn update_point(&mut self, point_id: CurvePointId, tick: u64, value: f32) -> Option<()> {
        if let Some(point) = self.points.iter_mut().find(|p| p.id == point_id) {
            point.tick = tick;
            point.value = value;
            let (min, max) = self.lane_type.value_range();
            point.value = point.value.clamp(min, max);
            self.sort_points();
            return Some(());
        }
        None
    }

    pub fn remove_point(&mut self, point_id: CurvePointId) -> Option<CurvePoint> {
        if let Some(idx) = self.points.iter().position(|p| p.id == point_id) {
            return Some(self.points.remove(idx));
        }
        None
    }

    pub fn value_at(&self, tick: u64) -> Option<f32> {
        if self.points.is_empty() {
            return None;
        }
        if self.points.len() == 1 {
            return Some(self.points[0].value);
        }
        if tick <= self.points[0].tick {
            return Some(self.points[0].value);
        }
        if tick >= self.points.last()?.tick {
            return Some(self.points.last()?.value);
        }
        let mut iter = self.points.iter().peekable();
        while let Some(current) = iter.next() {
            if let Some(next) = iter.peek() {
                if tick >= current.tick && tick <= next.tick {
                    let span = (next.tick - current.tick).max(1) as f32;
                    let alpha = (tick - current.tick) as f32 / span;
                    return Some(current.value + (next.value - current.value) * alpha);
                }
            }
        }
        self.points.last().map(|p| p.value)
    }

    fn clamp_point(&self, point: &mut CurvePoint) {
        let (min, max) = self.lane_type.value_range();
        point.value = point.value.clamp(min, max);
    }

    fn sort_points(&mut self) {
        self.points.sort_by(|a, b| a.tick.cmp(&b.tick).then_with(|| a.id.0.cmp(&b.id.0)));
    }
}

#[derive(Clone, Debug)]
pub struct TrackMeta {
    pub channel: u8,
    pub program: Option<u8>,
    pub track_name: Option<String>,
    pub single_channel: bool,
    pub tracks_with_notes: usize,
}

impl Default for TrackMeta {
    fn default() -> Self {
        Self {
            channel: 0,
            program: None,
            track_name: None,
            single_channel: true,
            tracks_with_notes: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MidiState {
    pub notes: Vec<Note>,
    pub curves: Vec<CurveLane>,
    pub ticks_per_beat: u16,
    pub bpm: f32,
    pub time_signature: (u8, u8),
    pub track: TrackMeta,
}

impl Default for MidiState {
    fn default() -> Self {
        Self {
            notes: Vec::new(),
            curves: vec![Self::default_velocity_lane()],
            ticks_per_beat: 480,
            bpm: 120.0,
            time_signature: (4, 4),
            track: TrackMeta::default(),
        }
    }
}

impl MidiState {
    pub fn from_smf(smf: &Smf) -> Self {
        let mut notes = Vec::new();
        let ticks_per_beat = match smf.header.timing {
            midly::Timing::Metrical(t) => t.as_int(),
            _ => 480, // Default fallback
        };
        let mut bpm = 120.0;
        let mut time_sig = (4, 4);
        let mut track_meta = TrackMeta::default();
        let mut tracks_with_notes = 0;
        let mut reference_channel: Option<u8> = None;
        let mut single_channel = true;
        let mut program = None;

        // Simplified loading: merge all MIDI tracks into a single note lane.
        for track in &smf.tracks {
            let mut current_ticks = 0;
            let mut active_notes: HashMap<(u8, u8), (u64, u8)> = HashMap::new();
            let mut track_has_notes = false;

            for event in track {
                current_ticks += event.delta.as_int() as u64;

                match event.kind {
                    TrackEventKind::Midi { channel, message } => {
                        let channel_val = channel.as_int();
                        match message {
                            midly::MidiMessage::NoteOn { key, vel } => {
                                let key_val = key.as_int();
                                if vel.as_int() > 0 {
                                    active_notes.insert(
                                        (channel_val, key_val),
                                        (current_ticks, vel.as_int()),
                                    );
                                } else {
                                    // NoteOn with velocity 0 is NoteOff
                                    if let Some((start, velocity)) =
                                        active_notes.remove(&(channel_val, key_val))
                                    {
                                        track_has_notes = true;
                                        notes.push(Note::new(
                                            start,
                                            current_ticks - start,
                                            key_val,
                                            velocity,
                                        ));
                                    }
                                }
                            }
                            midly::MidiMessage::NoteOff { key, .. } => {
                                let key_val = key.as_int();
                                if let Some((start, velocity)) =
                                    active_notes.remove(&(channel_val, key_val))
                                {
                                    track_has_notes = true;
                                    notes.push(Note::new(
                                        start,
                                        current_ticks - start,
                                        key_val,
                                        velocity,
                                    ));
                                }
                            }
                            midly::MidiMessage::ProgramChange { program: prog } => {
                                program = Some(prog.as_int());
                                track_meta.channel = channel_val;
                            }
                            _ => {}
                        }

                        if let Some(reference) = reference_channel {
                            if reference != channel_val {
                                single_channel = false;
                            }
                        } else {
                            reference_channel = Some(channel_val);
                        }
                    }
                    TrackEventKind::Meta(meta) => match meta {
                        MetaMessage::Tempo(value) => {
                            let micros_per_quarter = value.as_int() as f32;
                            if micros_per_quarter > 0.0 {
                                bpm = 60_000_000.0 / micros_per_quarter;
                            }
                        }
                        MetaMessage::TimeSignature(numer, denom, ..) => {
                            time_sig = (numer, 2u8.pow(denom as u32));
                        }
                        MetaMessage::TrackName(name) => {
                            track_meta.track_name = Some(
                                String::from_utf8_lossy(name.as_ref())
                                    .trim_matches(char::from(0))
                                    .to_string(),
                            );
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }

            if track_has_notes {
                tracks_with_notes += 1;
            }
        }

        notes.sort_by(|a, b| a.start.cmp(&b.start));

        track_meta.channel = reference_channel.unwrap_or(track_meta.channel);
        track_meta.program = program;
        track_meta.single_channel = single_channel;
        track_meta.tracks_with_notes = tracks_with_notes;

        Self {
            notes,
            curves: vec![Self::default_velocity_lane()],
            ticks_per_beat,
            bpm,
            time_signature: time_sig,
            track: track_meta,
        }
    }

    fn default_velocity_lane() -> CurveLane {
        CurveLane::new(CurveLaneType::Velocity)
    }

    pub fn get_velocity_at(&self, tick: u64) -> Option<u8> {
        for curve in &self.curves {
            if curve.lane_type == CurveLaneType::Velocity && curve.enabled {
                if let Some(value) = curve.value_at(tick) {
                    return Some(value.clamp(0.0, 127.0) as u8);
                }
            }
        }
        None
    }

    pub fn apply_velocity_curve_to_note(&self, note: &Note) -> u8 {
        if let Some(curve_velocity) = self.get_velocity_at(note.start) {
            curve_velocity
        } else {
            note.velocity
        }
    }

    pub fn to_smf(&self) -> Smf<'static> {
        use midly::{
            Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind,
        };

        let mut track: Vec<TrackEvent<'static>> = Vec::new();
        // Meta events for tempo and time signature at start.
        track.push(TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::Tempo(midly::num::u24::from(
                (60_000_000.0 / self.bpm.max(1.0)) as u32,
            ))),
        });
        track.push(TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::TimeSignature(
                self.time_signature.0,
                match self.time_signature.1 {
                    1 => 0,
                    2 => 1,
                    4 => 2,
                    8 => 3,
                    16 => 4,
                    _ => 2,
                },
                24,
                8,
            )),
        });
        if let Some(program) = self.track.program {
            track.push(TrackEvent {
                delta: 0.into(),
                kind: TrackEventKind::Midi {
                    channel: self.track.channel.into(),
                    message: MidiMessage::ProgramChange {
                        program: program.into(),
                    },
                },
            });
        }
        let mut events: Vec<(u64, TrackEventKind<'static>)> = Vec::new();
        for note in &self.notes {
            let velocity = self.apply_velocity_curve_to_note(note);
            events.push((
                note.start,
                TrackEventKind::Midi {
                    channel: self.track.channel.into(),
                    message: MidiMessage::NoteOn {
                        key: note.key.into(),
                        vel: velocity.into(),
                    },
                },
            ));
            events.push((
                note.start + note.duration,
                TrackEventKind::Midi {
                    channel: self.track.channel.into(),
                    message: MidiMessage::NoteOff {
                        key: note.key.into(),
                        vel: 0.into(),
                    },
                },
            ));
        }
        events.sort_by_key(|(t, _)| *t);

        let mut last_tick = 0;
        for (tick, kind) in events {
            let delta = tick.saturating_sub(last_tick);
            last_tick = tick;
            let delta_ticks = u32::try_from(delta).unwrap_or(u32::MAX);
            track.push(TrackEvent {
                delta: delta_ticks.into(),
                kind,
            });
        }
        track.push(TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        });

        Smf {
            header: Header {
                format: Format::SingleTrack,
                timing: Timing::Metrical(self.ticks_per_beat.into()),
            },
            tracks: vec![track],
        }
    }

    pub fn validate_single_track(&self) -> Result<(), MidiValidationError> {
        if self.track.tracks_with_notes > 1 {
            return Err(MidiValidationError::MultipleTracks {
                tracks: self.track.tracks_with_notes,
            });
        }
        if !self.track.single_channel {
            return Err(MidiValidationError::MixedChannels);
        }
        Ok(())
    }

    pub fn from_smf_strict(smf: &Smf) -> Result<Self, MidiValidationError> {
        let state = Self::from_smf(smf);
        state.validate_single_track()?;
        Ok(state)
    }

    pub fn to_single_track_smf(&self) -> Result<Smf<'static>, MidiValidationError> {
        self.validate_single_track()?;
        Ok(self.to_smf())
    }

    /// Helper method to apply a closure to selected notes identified by their IDs
    fn apply_to_selected_notes<F>(&mut self, note_ids: &[NoteId], mut f: F)
    where
        F: FnMut(&mut Note),
    {
        use std::collections::HashSet;
        if note_ids.is_empty() {
            return;
        }
        let id_set: HashSet<NoteId> = note_ids.iter().copied().collect();
        
        for note in &mut self.notes {
            if id_set.contains(&note.id) {
                f(note);
            }
        }
    }

    /// Humanize selected notes by adding random variations to timing and velocity
    pub fn humanize_notes(&mut self, note_ids: &[NoteId], time_range: u64, velocity_range: u8) {
        self.apply_to_selected_notes(note_ids, |note| {
            // Add random time offset
            let time_offset = if time_range > 0 {
                let range = time_range as i64;
                fastrand::i64(-range..=range)
            } else {
                0
            };
            note.start = (note.start as i64 + time_offset).max(0) as u64;
            
            // Add random velocity offset
            let velocity_offset = if velocity_range > 0 {
                let range = velocity_range as i8;
                fastrand::i8(-range..=range)
            } else {
                0
            };
            note.velocity = ((note.velocity as i16 + velocity_offset as i16).max(0).min(127)) as u8;
        });
        
        // Re-sort notes after time changes
        self.notes.sort_by_key(|n| n.start);
    }

    /// Apply batch transform to selected notes
    pub fn batch_transform_notes(
        &mut self,
        note_ids: &[NoteId],
        transform_type: BatchTransformType,
        value: f64,
    ) {
        let needs_sort = matches!(transform_type, BatchTransformType::VelocityOffset | BatchTransformType::DurationScale);
        
        self.apply_to_selected_notes(note_ids, |note| {
            match transform_type {
                BatchTransformType::VelocityOffset => {
                    let new_velocity = (note.velocity as f64 + value).round() as i16;
                    note.velocity = new_velocity.max(0).min(127) as u8;
                }
                BatchTransformType::DurationScale => {
                    let new_duration = (note.duration as f64 * value).round() as i64;
                    note.duration = new_duration.max(1) as u64;
                }
                BatchTransformType::PitchOffset => {
                    let new_key = (note.key as f64 + value).round() as i16;
                    note.key = new_key.max(0).min(127) as u8;
                }
            }
        });
        
        // Only sort if start time or duration might have changed
        // Pitch changes don't affect sort order
        if needs_sort {
            self.notes.sort_by_key(|n| n.start);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatchTransformType {
    VelocityOffset,
    DurationScale,
    PitchOffset,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MidiValidationError {
    MultipleTracks { tracks: usize },
    MixedChannels,
}

impl fmt::Display for MidiValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MidiValidationError::MultipleTracks { tracks } => {
                write!(f, "MIDI 包含 {tracks} 条含音符的轨道，超出单轨要求")
            }
            MidiValidationError::MixedChannels => {
                write!(f, "MIDI 轨道包含多个通道，无法保证单轨一致性")
            }
        }
    }
}

impl std::error::Error for MidiValidationError {}

pub fn load_single_track(bytes: &[u8]) -> Result<MidiState, midly::Error> {
    let smf = Smf::parse(bytes)?;
    Ok(MidiState::from_smf(&smf))
}

pub fn export_single_track(state: &MidiState) -> Vec<u8> {
    let smf = state.to_smf();
    let mut out = Vec::new();
    smf.write_std(&mut out).expect("Writing SMF failed");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use midly::{
        Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind,
    };

    fn build_simple_note_track(channel: u8, key: u8) -> Vec<TrackEvent<'static>> {
        vec![
            TrackEvent {
                delta: 0.into(),
                kind: TrackEventKind::Midi {
                    channel: channel.into(),
                    message: MidiMessage::NoteOn {
                        key: key.into(),
                        vel: 100.into(),
                    },
                },
            },
            TrackEvent {
                delta: 120.into(),
                kind: TrackEventKind::Midi {
                    channel: channel.into(),
                    message: MidiMessage::NoteOff {
                        key: key.into(),
                        vel: 0.into(),
                    },
                },
            },
            TrackEvent {
                delta: 0.into(),
                kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
            },
        ]
    }

    fn smf_with_tracks(tracks: Vec<Vec<TrackEvent<'static>>>) -> Smf<'static> {
        Smf {
            header: Header {
                format: Format::Parallel,
                timing: Timing::Metrical(480.into()),
            },
            tracks,
        }
    }

    #[test]
    fn strict_import_rejects_multiple_tracks() {
        let smf = smf_with_tracks(vec![
            build_simple_note_track(0, 60),
            build_simple_note_track(0, 67),
        ]);
        let err = MidiState::from_smf_strict(&smf).unwrap_err();
        assert!(matches!(
            err,
            MidiValidationError::MultipleTracks { tracks: 2 }
        ));
    }

    #[test]
    fn strict_import_rejects_mixed_channels() {
        let mut track = build_simple_note_track(0, 60);
        track.insert(
            1,
            TrackEvent {
                delta: 0.into(),
                kind: TrackEventKind::Midi {
                    channel: 1.into(),
                    message: MidiMessage::NoteOn {
                        key: 62.into(),
                        vel: 100.into(),
                    },
                },
            },
        );
        let smf = smf_with_tracks(vec![track]);
        let err = MidiState::from_smf_strict(&smf).unwrap_err();
        assert_eq!(err, MidiValidationError::MixedChannels);
    }
}

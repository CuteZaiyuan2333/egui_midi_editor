use midly::{MetaMessage, Smf, TrackEventKind};
use std::sync::atomic::{AtomicU64, Ordering};

static NOTE_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

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
    pub start: u64, // Absolute ticks
    pub duration: u64, // Ticks
    pub key: u8, // MIDI note number (0-127)
    pub velocity: u8, // 0-127
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

#[derive(Clone, Debug)]
pub struct MidiState {
    pub notes: Vec<Note>,
    pub ticks_per_beat: u16,
    pub bpm: f32,
    pub time_signature: (u8, u8),
    pub track_name: Option<String>,
}

impl Default for MidiState {
    fn default() -> Self {
        Self {
            notes: Vec::new(),
            ticks_per_beat: 480,
            bpm: 120.0,
            time_signature: (4, 4),
            track_name: None,
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
        let mut track_name = None;

        // Simplified loading: merge all MIDI tracks into a single note lane.
        for track in &smf.tracks {
            let mut current_ticks = 0;
            let mut active_notes: std::collections::HashMap<u8, (u64, u8)> = std::collections::HashMap::new();

            for event in track {
                current_ticks += event.delta.as_int() as u64;
                
                match event.kind {
                    TrackEventKind::Midi { message, .. } => {
                        match message {
                            midly::MidiMessage::NoteOn { key, vel } => {
                                if vel.as_int() > 0 {
                                    active_notes.insert(key.as_int(), (current_ticks, vel.as_int()));
                                } else {
                                    // NoteOn with velocity 0 is NoteOff
                                    if let Some((start, velocity)) = active_notes.remove(&key.as_int()) {
                                        notes.push(Note::new(start, current_ticks - start, key.as_int(), velocity));
                                    }
                                }
                            }
                            midly::MidiMessage::NoteOff { key, .. } => {
                                if let Some((start, velocity)) = active_notes.remove(&key.as_int()) {
                                    notes.push(Note::new(start, current_ticks - start, key.as_int(), velocity));
                                }
                            }
                            _ => {}
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
                            track_name = Some(
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
        }
        
        notes.sort_by(|a, b| a.start.cmp(&b.start));

        Self {
            notes,
            ticks_per_beat,
            bpm,
            time_signature: time_sig,
            track_name,
        }
    }

    pub fn to_smf(&self) -> Smf<'static> {
        use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};

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
        let mut events: Vec<(u64, TrackEventKind<'static>)> = Vec::new();
        for note in &self.notes {
            events.push((
                note.start,
                TrackEventKind::Midi {
                    channel: 0.into(),
                    message: MidiMessage::NoteOn {
                        key: note.key.into(),
                        vel: note.velocity.into(),
                    },
                },
            ));
            events.push((
                note.start + note.duration,
                TrackEventKind::Midi {
                    channel: 0.into(),
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
}

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

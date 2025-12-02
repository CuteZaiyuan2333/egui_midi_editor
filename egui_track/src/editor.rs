//! 编辑命令和事件模块
//!
//! 定义了音轨编辑器的命令系统和事件系统，用于与宿主应用交互。

use crate::structure::{ClipId, TrackId, ClipType};

#[derive(Clone, Debug)]
pub enum TrackEditorCommand {
    CreateClip {
        track_id: TrackId,
        start: f64,
        duration: f64,
        clip_type: ClipType,
    },
    DeleteClip {
        clip_id: ClipId,
    },
    MoveClip {
        clip_id: ClipId,
        new_track_id: TrackId,
        new_start: f64,
    },
    ResizeClip {
        clip_id: ClipId,
        new_duration: f64,
        resize_from_start: bool,
    },
    SplitClip {
        clip_id: ClipId,
        split_time: f64,
    },
    CreateTrack {
        name: String,
    },
    DeleteTrack {
        track_id: TrackId,
    },
    RenameTrack {
        track_id: TrackId,
        new_name: String,
    },
    SetPlayhead {
        position: f64,
    },
    SetTimeSignature {
        numer: u8,
        denom: u8,
    },
    SetBPM {
        bpm: f32,
    },
    SetMetronome {
        enabled: bool,
    },
    SetPlayback {
        is_playing: bool,
    },
    StopPlayback,
}

#[derive(Clone, Debug)]
pub enum TrackEditorEvent {
    ClipSelected {
        clip_id: ClipId,
    },
    ClipDoubleClicked {
        clip_id: ClipId,
    },
    ClipMoved {
        clip_id: ClipId,
        old_track_id: TrackId,
        new_track_id: TrackId,
        new_start: f64,
    },
    ClipResized {
        clip_id: ClipId,
        new_duration: f64,
    },
    PlayheadChanged {
        position: f64,
    },
    TrackCreated {
        track_id: TrackId,
    },
    TrackDeleted {
        track_id: TrackId,
    },
    TimeSignatureChanged {
        numer: u8,
        denom: u8,
    },
    BPMChanged {
        bpm: f32,
    },
    MetronomeChanged {
        enabled: bool,
    },
    PlaybackStateChanged {
        is_playing: bool,
    },
}

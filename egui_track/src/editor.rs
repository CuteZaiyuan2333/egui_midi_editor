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
        disable_snap: bool,
    },
    ResizeClip {
        clip_id: ClipId,
        new_duration: f64,
        resize_from_start: bool,
        disable_snap: bool,
    },
    SplitClip {
        clip_id: ClipId,
        split_time: f64,
    },
    RenameClip {
        clip_id: ClipId,
        new_name: String,
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
    SetSnapEnabled {
        enabled: bool,
    },
    SetSnapInterval {
        interval: u64,
    },
    SetPlayback {
        is_playing: bool,
    },
    StopPlayback,
    SetTrackMute {
        track_id: TrackId,
        muted: bool,
    },
    SetTrackSolo {
        track_id: TrackId,
        solo: bool,
    },
    SetTrackVolume {
        track_id: TrackId,
        volume: f32,
    },
    SetTrackPan {
        track_id: TrackId,
        pan: f32,
    },
    SetTrackRecordArm {
        track_id: TrackId,
        armed: bool,
    },
    SetTrackMonitor {
        track_id: TrackId,
        monitor: bool,
    },
    CopyClips {
        clip_ids: Vec<ClipId>,
    },
    CutClips {
        clip_ids: Vec<ClipId>,
    },
    PasteClips {
        track_id: TrackId,
        start_time: f64,
    },
    DeleteClips {
        clip_ids: Vec<ClipId>,
    },
    UpdateClipPreview {
        clip_id: ClipId,
        preview_notes: Vec<crate::structure::PreviewNote>,
    },
    UpdateClipMidiFilePath {
        clip_id: ClipId,
        new_file_path: String,
    },
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
    ClipRenamed {
        clip_id: ClipId,
        new_name: String,
    },
    ClipDeleted {
        clip_id: ClipId,
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
    SnapEnabledChanged {
        enabled: bool,
    },
    SnapIntervalChanged {
        interval: u64,
    },
    PlaybackStateChanged {
        is_playing: bool,
    },
    TrackMuteChanged {
        track_id: TrackId,
        muted: bool,
    },
    TrackSoloChanged {
        track_id: TrackId,
        solo: bool,
    },
    TrackVolumeChanged {
        track_id: TrackId,
        volume: f32,
    },
    TrackPanChanged {
        track_id: TrackId,
        pan: f32,
    },
    TrackRecordArmChanged {
        track_id: TrackId,
        armed: bool,
    },
    TrackMonitorChanged {
        track_id: TrackId,
        monitor: bool,
    },
}

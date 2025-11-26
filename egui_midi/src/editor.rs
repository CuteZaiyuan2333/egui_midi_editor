use crate::structure::{CurveLaneId, CurvePointId, MidiState, Note, NoteId};

/// 宿主可描述的吸附模式
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

/// 外部控制的传输/播放状态
#[derive(Clone, Copy, Debug, Default)]
pub struct TransportState {
    pub is_playing: bool,
    pub position_seconds: f32,
    pub bpm_override: Option<f32>,
}

/// 外部宿主可监听的编辑事件
#[derive(Clone, Debug)]
pub enum EditorEvent {
    StateReplaced(MidiState),
    NoteAdded(Note),
    NoteDeleted(Note),
    NoteUpdated {
        before: Note,
        after: Note,
    },
    SelectionChanged(Vec<NoteId>),
    PlaybackStateChanged {
        is_playing: bool,
    },
    TransportChanged {
        current_time: f32,
        current_tick: u64,
    },
    CurveLaneAdded(CurveLaneId),
    CurveLaneRemoved(CurveLaneId),
    CurvePointAdded {
        lane_id: CurveLaneId,
        point_id: CurvePointId,
    },
    CurvePointRemoved {
        lane_id: CurveLaneId,
        point_id: CurvePointId,
    },
    CurvePointUpdated {
        lane_id: CurveLaneId,
        point_id: CurvePointId,
    },
}

/// 宿主可推送到编辑器的命令
#[derive(Clone, Debug)]
pub enum EditorCommand {
    ReplaceState(MidiState),
    SetNotes(Vec<Note>),
    AppendNotes(Vec<Note>),
    ClearNotes,
    SeekSeconds(f32),
    SetPlayback(bool),
    CenterOnKey(u8),
    SetBpm(f32),
    SetTimeSignature(u8, u8),
    SetVolume(f32),
    SetLoop {
        enabled: bool,
        start_tick: u64,
        end_tick: u64,
    },
    SetSnap {
        interval: u64,
        mode: SnapMode,
    },
    OverrideTransport(Option<TransportState>),
    AddCurvePoint {
        lane_id: CurveLaneId,
        tick: u64,
        value: f32,
    },
    UpdateCurvePoint {
        lane_id: CurveLaneId,
        point_id: CurvePointId,
        tick: u64,
        value: f32,
    },
    RemoveCurvePoint {
        lane_id: CurveLaneId,
        point_id: CurvePointId,
    },
    ToggleCurveLaneEnabled {
        lane_id: CurveLaneId,
    },
}

/// 初始化与运行时的视图配置
#[derive(Clone, Debug)]
pub struct MidiEditorOptions {
    pub zoom_x: f32,
    pub zoom_y: f32,
    pub snap_interval: u64,
    pub snap_mode: SnapMode,
    pub swing_ratio: f32,
    pub volume: f32,
    pub preview_pitch_shift: f32,
    pub loop_enabled: bool,
    pub loop_start_tick: u64,
    pub loop_end_tick: u64,
    pub manual_scroll_x: f32,
    pub manual_scroll_y: f32,
    /// 可选：启动时将视图滚动到某个音高
    pub center_on_key: Option<u8>,
}

impl Default for MidiEditorOptions {
    fn default() -> Self {
        Self {
            zoom_x: 100.0,
            zoom_y: 20.0,
            snap_interval: 120,
            snap_mode: SnapMode::Absolute,
            swing_ratio: 0.0,
            volume: 0.5,
            preview_pitch_shift: 0.0,
            loop_enabled: false,
            loop_start_tick: 0,
            loop_end_tick: 1920,
            manual_scroll_x: 0.0,
            manual_scroll_y: 0.0,
            center_on_key: Some(60),
        }
    }
}

impl MidiEditorOptions {
    pub fn centered_on(key: u8) -> Self {
        Self {
            center_on_key: Some(key),
            ..Self::default()
        }
    }
}

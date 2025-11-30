//! 数据结构模块
//!
//! 定义了音轨编辑器使用的核心数据结构，包括音轨、剪辑片段和时间轴状态。

use egui::Color32;
use std::sync::atomic::{AtomicU64, Ordering};

static TRACK_ID_COUNTER: AtomicU64 = AtomicU64::new(1);
static CLIP_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TrackId(pub u64);

impl TrackId {
    pub fn next() -> Self {
        TrackId(TRACK_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClipId(pub u64);

impl ClipId {
    pub fn next() -> Self {
        ClipId(CLIP_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Clone, Debug)]
pub struct PreviewNote {
    pub start: f64,      // 相对于剪辑开始的时间（秒）
    pub duration: f64,   // 持续时间（秒）
    pub key: u8,         // MIDI 音符编号 (0-127)
    pub velocity: u8,    // 力度 (0-127)
}

#[derive(Clone, Debug)]
pub struct MidiClipData {
    pub midi_file_path: Option<String>,
    pub preview_notes: Vec<PreviewNote>,
}

#[derive(Clone, Debug)]
pub struct AudioClipData {
    pub audio_file_path: Option<String>,
    pub waveform_data: Option<Vec<f32>>,  // 归一化的波形数据，用于预览
}

#[derive(Clone, Debug)]
pub enum ClipType {
    Midi { midi_data: Option<MidiClipData> },
    Audio { audio_data: Option<AudioClipData> },
}

#[derive(Clone, Debug)]
pub struct Clip {
    pub id: ClipId,
    pub track_id: TrackId,
    pub start_time: f64,       // 开始时间（秒）
    pub duration: f64,         // 持续时间（秒）
    pub clip_type: ClipType,
    pub name: String,
    pub color: Color32,
}

impl Clip {
    pub fn new_midi(track_id: TrackId, start_time: f64, duration: f64, name: String) -> Self {
        Self {
            id: ClipId::next(),
            track_id,
            start_time,
            duration,
            clip_type: ClipType::Midi { midi_data: None },
            name,
            color: Color32::from_rgb(100, 200, 100),
        }
    }

    pub fn new_audio(track_id: TrackId, start_time: f64, duration: f64, name: String) -> Self {
        Self {
            id: ClipId::next(),
            track_id,
            start_time,
            duration,
            clip_type: ClipType::Audio { audio_data: None },
            name,
            color: Color32::from_rgb(150, 150, 250),
        }
    }

    pub fn end_time(&self) -> f64 {
        self.start_time + self.duration
    }
}

#[derive(Clone, Debug)]
pub struct Track {
    pub id: TrackId,
    pub name: String,
    pub height: f32,           // 轨道高度
    pub muted: bool,
    pub solo: bool,
    pub volume: f32,
    pub clips: Vec<Clip>,
}

impl Track {
    pub fn new(name: String) -> Self {
        Self {
            id: TrackId::next(),
            name,
            height: 80.0,
            muted: false,
            solo: false,
            volume: 1.0,
            clips: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct TimelineState {
    pub zoom_x: f32,           // 水平缩放（像素/秒）
    pub scroll_x: f64,         // 水平滚动位置（秒）
    pub scroll_y: f32,         // 垂直滚动位置
    pub playhead_position: f64, // 播放头位置（秒）
    pub snap_enabled: bool,
    pub snap_interval: f64,     // 对齐间隔（秒）
    pub time_signature: (u8, u8),
    pub bpm: f32,
}

impl Default for TimelineState {
    fn default() -> Self {
        Self {
            zoom_x: 100.0,      // 100 像素/秒
            scroll_x: 0.0,
            scroll_y: 0.0,
            playhead_position: 0.0,
            snap_enabled: true,
            snap_interval: 0.25,  // 1/4 拍
            time_signature: (4, 4),
            bpm: 120.0,
        }
    }
}

impl TimelineState {
    pub fn time_to_x(&self, time: f64) -> f32 {
        ((time - self.scroll_x) * self.zoom_x as f64) as f32
    }

    pub fn x_to_time(&self, x: f32) -> f64 {
        (x as f64 / self.zoom_x as f64) + self.scroll_x
    }

    pub fn snap_time(&self, time: f64) -> f64 {
        if self.snap_enabled {
            (time / self.snap_interval).round() * self.snap_interval
        } else {
            time
        }
    }
}

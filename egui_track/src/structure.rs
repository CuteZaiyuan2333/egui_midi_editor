//! 数据结构模块
//!
//! 定义了音轨编辑器使用的核心数据结构，包括音轨、剪辑片段和时间轴状态。

use egui::Color32;
use serde::{Deserialize, Serialize, Serializer, Deserializer};
use std::sync::atomic::{AtomicU64, Ordering};

// Color32 序列化辅助类型
#[derive(Serialize, Deserialize)]
struct Color32Helper {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl From<Color32> for Color32Helper {
    fn from(color: Color32) -> Self {
        Self {
            r: color.r(),
            g: color.g(),
            b: color.b(),
            a: color.a(),
        }
    }
}

impl From<Color32Helper> for Color32 {
    fn from(helper: Color32Helper) -> Self {
        Color32::from_rgba_unmultiplied(helper.r, helper.g, helper.b, helper.a)
    }
}

fn serialize_color32<S>(color: &Color32, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    Color32Helper::from(*color).serialize(serializer)
}

fn deserialize_color32<'de, D>(deserializer: D) -> Result<Color32, D::Error>
where
    D: Deserializer<'de>,
{
    let helper = Color32Helper::deserialize(deserializer)?;
    Ok(Color32::from(helper))
}

static TRACK_ID_COUNTER: AtomicU64 = AtomicU64::new(1);
static CLIP_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TrackId(pub u64);

impl TrackId {
    pub fn next() -> Self {
        TrackId(TRACK_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ClipId(pub u64);

impl ClipId {
    pub fn next() -> Self {
        ClipId(CLIP_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PreviewNote {
    pub start: f64,      // 相对于剪辑开始的时间（秒）
    pub duration: f64,   // 持续时间（秒）
    pub key: u8,         // MIDI 音符编号 (0-127)
    pub velocity: u8,    // 力度 (0-127)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MidiClipData {
    pub midi_file_path: Option<String>,
    pub preview_notes: Vec<PreviewNote>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AudioClipData {
    pub audio_file_path: Option<String>,
    pub waveform_data: Option<Vec<f32>>,  // 归一化的波形数据，用于预览
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ClipType {
    Midi { midi_data: Option<MidiClipData> },
    Audio { audio_data: Option<AudioClipData> },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Clip {
    pub id: ClipId,
    pub track_id: TrackId,
    pub start_time: f64,       // 开始时间（秒）
    pub duration: f64,         // 持续时间（秒）
    pub clip_type: ClipType,
    pub name: String,
    #[serde(serialize_with = "serialize_color32", deserialize_with = "deserialize_color32")]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimelineState {
    pub zoom_x: f32,           // 水平缩放（像素/节拍，与 MIDI 编辑器一致）
    pub scroll_x: f64,         // 水平滚动位置（节拍）
    pub scroll_y: f32,         // 垂直滚动位置
    pub manual_scroll_x: f32,  // 手动水平滚动偏移（像素，与 MIDI 编辑器一致）
    pub manual_scroll_y: f32,  // 手动垂直滚动偏移（像素，与 MIDI 编辑器一致）
    pub playhead_position: f64, // 播放头位置（秒，用于播放控制）
    pub snap_enabled: bool,
    pub snap_interval: u64,     // 对齐间隔（tick 单位，与 MIDI 编辑器一致）
    pub snap_mode: SnapMode,   // 对齐模式（绝对/相对）
    pub time_signature: (u8, u8),
    pub bpm: f32,
    pub ticks_per_beat: u16,   // 每拍的 tick 数（与 MIDI 编辑器一致，默认 480）
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnapMode {
    Absolute,  // 绝对对齐：总是对齐到网格
    Relative,  // 相对对齐：保持相对偏移
}

impl Default for TimelineState {
    fn default() -> Self {
        Self {
            zoom_x: 100.0,      // 100 像素/节拍
            scroll_x: 0.0,       // 0 节拍
            scroll_y: 0.0,
            manual_scroll_x: 0.0,
            manual_scroll_y: 0.0,
            playhead_position: 0.0,  // 0 秒
            snap_enabled: true,
            snap_interval: 480,  // 1 拍 = 480 ticks（默认）
            snap_mode: SnapMode::Absolute,
            time_signature: (4, 4),
            bpm: 120.0,
            ticks_per_beat: 480,  // 默认 480 ticks/beat
        }
    }
}

impl TimelineState {
    /// 将时间（秒）转换为 tick
    pub fn time_to_tick(&self, time: f64) -> u64 {
        let seconds_per_beat = 60.0 / self.bpm.max(1.0) as f64;
        let seconds_per_tick = seconds_per_beat / self.ticks_per_beat.max(1) as f64;
        (time / seconds_per_tick).round() as u64
    }

    /// 将 tick 转换为时间（秒）
    pub fn tick_to_time(&self, tick: u64) -> f64 {
        let seconds_per_beat = 60.0 / self.bpm.max(1.0) as f64;
        let seconds_per_tick = seconds_per_beat / self.ticks_per_beat.max(1) as f64;
        tick as f64 * seconds_per_tick
    }

    /// 将节拍转换为 tick
    pub fn beat_to_tick(&self, beat: f64) -> u64 {
        (beat * self.ticks_per_beat as f64).round() as u64
    }

    /// 将 tick 转换为节拍
    pub fn tick_to_beat(&self, tick: u64) -> f64 {
        tick as f64 / self.ticks_per_beat as f64
    }

    /// 将 tick 转换为 x 坐标（像素）
    /// 这是与 MIDI 编辑器一致的坐标转换函数
    pub fn tick_to_x(&self, tick: u64, header_width: f32) -> f32 {
        let beat = self.tick_to_beat(tick);
        let scroll_beat = self.scroll_x;
        let rel_beat = beat - scroll_beat;
        header_width + (rel_beat as f32 * self.zoom_x) + self.manual_scroll_x
    }

    /// 将 x 坐标（像素）转换为 tick
    /// 这是与 MIDI 编辑器一致的坐标转换函数
    pub fn x_to_tick(&self, x: f32, header_width: f32) -> u64 {
        let rel_x = x - header_width - self.manual_scroll_x;
        let beat = (rel_x / self.zoom_x) as f64 + self.scroll_x;
        self.beat_to_tick(beat.max(0.0))
    }

    /// 对齐 tick 到网格
    pub fn snap_tick(&self, tick: u64, disable_snap: bool) -> u64 {
        if !self.snap_enabled || disable_snap || self.snap_interval == 0 {
            return tick;
        }
        (match self.snap_mode {
            SnapMode::Absolute => {
                (tick as f64 / self.snap_interval as f64).round() * self.snap_interval as f64
            }
            SnapMode::Relative => {
                // 相对对齐模式：保持相对偏移（这里简化处理，与绝对对齐相同）
                (tick as f64 / self.snap_interval as f64).round() * self.snap_interval as f64
            }
        }) as u64
    }

    /// 旧的时间到 x 坐标转换（保持向后兼容）
    pub fn time_to_x(&self, time: f64) -> f32 {
        let tick = self.time_to_tick(time);
        self.tick_to_x(tick, 0.0)
    }

    /// 旧的 x 坐标到时间转换（保持向后兼容）
    pub fn x_to_time(&self, x: f32) -> f64 {
        let tick = self.x_to_tick(x, 0.0);
        self.tick_to_time(tick)
    }

    /// 旧的时间对齐函数（保持向后兼容，但内部使用 tick 对齐）
    pub fn snap_time(&self, time: f64) -> f64 {
        let tick = self.time_to_tick(time);
        let snapped_tick = self.snap_tick(tick, false);
        self.tick_to_time(snapped_tick)
    }
}

//! 多轨播放引擎模块
//!
//! 实现多轨 MIDI 播放功能，包括事件调度和音频混合。

use egui_track::{Track, ClipType, TrackId};
use crate::clip_operations::ticks_to_seconds;
use crate::audio::mixer::{AudioMixer, MidiEvent};
use std::collections::{HashMap, HashSet, VecDeque};

/// 调度的 MIDI 事件
#[derive(Clone, Debug)]
struct ScheduledEvent {
    time: f64,  // 绝对时间（秒）
    event: MidiEvent,
}

/// 活跃音符信息
struct ActiveNoteInfo {
    #[allow(dead_code)]
    start_time: f64,
    #[allow(dead_code)]
    track_index: usize,
    #[allow(dead_code)]
    velocity: u8,
}

/// 多轨播放引擎
pub struct MultiTrackPlaybackEngine {
    mixer: AudioMixer,
    is_playing: bool,
    playback_position: f64,  // 当前播放位置（秒）
    last_update_time: f64,
    event_queue: VecDeque<ScheduledEvent>,  // 使用队列而不是每帧清空
    active_notes: HashMap<(TrackId, u8), ActiveNoteInfo>,  // 使用 (TrackId, key) 作为键
    processed_events: HashSet<(TrackId, u8, u64)>,  // 跟踪已处理的事件，使用 u64 表示时间（毫秒）避免重复触发
}

impl MultiTrackPlaybackEngine {
    pub fn new(track_count: usize) -> Self {
        let sample_rate = 44_100;
        let channels = 2;  // 立体声
        let mixer = AudioMixer::new(sample_rate, channels, track_count);
        
        Self {
            mixer,
            is_playing: false,
            playback_position: 0.0,
            last_update_time: 0.0,
            event_queue: VecDeque::new(),
            active_notes: HashMap::new(),
            processed_events: HashSet::new(),
        }
    }
    
    /// 从指定位置开始播放
    pub fn start_from_position(&mut self, current_time: f64, position: f64) {
        self.is_playing = true;
        self.playback_position = position;
        self.last_update_time = current_time;
        
        // 停止所有轨道的音符
        // 注意：AudioMixer 在创建时设置了最大轨道数（32），这里我们停止所有可能的轨道
        for i in 0..32 {
            self.mixer.all_notes_off(i);
        }
        
        self.active_notes.clear();
        self.processed_events.clear();
        self.event_queue.clear();
    }

    /// 停止播放
    pub fn stop(&mut self) {
        self.is_playing = false;
        
        // 停止所有轨道的音符
        for i in 0..32 {
            self.mixer.all_notes_off(i);
        }
        
        self.active_notes.clear();
        self.processed_events.clear();
        self.event_queue.clear();
    }

    /// 暂停播放
    pub fn pause(&mut self) {
        self.is_playing = false;
    }

    /// 恢复播放
    pub fn resume(&mut self, current_time: f64) {
        self.is_playing = true;
        self.last_update_time = current_time;
    }

    /// 设置播放位置
    pub fn seek(&mut self, position: f64) {
        self.playback_position = position;
        
        // 停止所有音符
        for i in 0..32 {
            self.mixer.all_notes_off(i);
        }
        
        self.active_notes.clear();
        self.processed_events.clear();
        
        // 清除已过期的事件
        self.event_queue.retain(|e| e.time >= position);
    }

    /// 更新播放（应在每帧调用）
    pub fn update(&mut self, current_time: f64, tracks: &[Track], timeline_bpm: f32) {
        if !self.is_playing {
            return;
        }

        let delta_time = current_time - self.last_update_time;
        self.playback_position += delta_time;
        self.last_update_time = current_time;

        // 调度 MIDI 事件（只调度新的事件，不清空队列）
        self.schedule_midi_events(tracks, timeline_bpm);

        // 处理已到时间的 MIDI 事件
        self.process_scheduled_events();
        
        // 清理已处理的事件标记（只保留最近 1 秒的事件）
        let cleanup_time_ms = ((self.playback_position - 1.0) * 1000.0) as u64;
        self.processed_events.retain(|(_, _, time_ms)| *time_ms >= cleanup_time_ms);
    }

    /// 调度 MIDI 事件
    fn schedule_midi_events(&mut self, tracks: &[Track], _timeline_bpm: f32) {
        // 不再清空事件队列，而是添加新事件
        // 只调度在当前播放位置附近的事件（例如前后 0.5 秒）
        let time_window = 0.5;  // 秒
        let min_time = self.playback_position - time_window;
        let max_time = self.playback_position + time_window;

        // 检查是否有任何轨道是 solo
        let has_solo = tracks.iter().any(|track| track.solo);
        
        for (track_index, track) in tracks.iter().enumerate() {
            // Solo 逻辑：
            // - 如果有任何轨道是 solo，只播放 solo 的轨道（忽略 muted）
            // - 如果没有轨道是 solo，播放所有未静音的轨道
            if has_solo {
                if !track.solo {
                    continue;  // 如果有 solo 轨道，跳过非 solo 轨道
                }
            } else {
                if track.muted {
                    continue;  // 如果没有 solo 轨道，跳过静音轨道
                }
            }

            // 更新轨道音量和声像
            self.mixer.set_track_volume(track_index, track.volume);
            self.mixer.set_track_pan(track_index, track.pan);

            // 遍历轨道的所有剪辑
            for clip in &track.clips {
                if let ClipType::Midi { ref midi_data } = clip.clip_type {
                    if let Some(ref midi_data) = midi_data {
                        // 只从文件路径加载 MIDI 数据
                        if let Some(ref file_path) = midi_data.midi_file_path {
                            let path = std::path::Path::new(file_path);
                            if path.exists() {
                                match crate::midiclip::load_midiclip_file(path) {
                                    Ok(midi_state) => {
                                        // 计算剪辑在当前播放位置的相对时间
                                        let clip_start = clip.start_time;
                                        let clip_end = clip.start_time + clip.duration;
                                        
                                        // 如果剪辑与时间窗口有重叠，就调度它
                                        // 这包括：剪辑在当前播放位置之前开始但仍在时间窗口内的情况
                                        let clip_overlaps_window = clip_start <= max_time && clip_end >= min_time;
                                        
                                        if clip_overlaps_window {
                                            let relative_time = if self.playback_position >= clip_start {
                                                self.playback_position - clip_start
                                            } else {
                                                0.0  // 如果播放位置在剪辑之前，从剪辑开始处计算
                                            };
                                            self.schedule_clip_events(
                                                &midi_state,
                                                relative_time,
                                                clip_start,
                                                track.id,
                                                track_index,
                                                min_time,
                                                max_time,
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        log::warn!("Failed to load MIDI file for playback: {:?}", e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 按时间排序事件队列
        // 注意：VecDeque 不支持直接排序，我们需要转换为 Vec 排序后再转回
        let mut events: Vec<_> = self.event_queue.drain(..).collect();
        events.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal));
        self.event_queue = events.into_iter().collect();
    }

    /// 调度单个剪辑的 MIDI 事件
    fn schedule_clip_events(
        &mut self,
        midi_state: &egui_midi::structure::MidiState,
        _relative_time: f64,
        clip_start: f64,
        track_id: TrackId,
        track_index: usize,
        min_time: f64,
        max_time: f64,
    ) {
        // 注意：我们不再使用 relative_ticks，而是直接使用时间来计算音符的开始和结束时间

        // 查找在当前时间范围内应该播放的音符
        for note in &midi_state.notes {
            let note_start_ticks = note.start;
            let note_end_ticks = note.start + note.duration;
            let note_start_time = clip_start + ticks_to_seconds(note_start_ticks, midi_state.bpm, midi_state.ticks_per_beat);
            let note_end_time = clip_start + ticks_to_seconds(note_end_ticks, midi_state.bpm, midi_state.ticks_per_beat);

            // 检查音符是否在当前时间窗口内
            if note_start_time >= min_time && note_start_time <= max_time {
                // 检查是否已经调度过这个事件（使用毫秒精度避免浮点误差）
                let time_ms = (note_start_time * 1000.0) as u64;
                let event_key = (track_id, note.key, time_ms);
                
                if !self.processed_events.contains(&event_key) {
                    // 如果音符在当前播放位置或之前，立即触发；否则加入队列
                    if note_start_time <= self.playback_position {
                        // 立即触发（处理从位置0开始播放时第一个音符的情况）
                        self.mixer.send_event(MidiEvent::NoteOn {
                            track_index,
                            key: note.key,
                            velocity: note.velocity,
                        });
                        log::debug!("Immediately triggered note at time {} (playback_position: {})", note_start_time, self.playback_position);
                    } else {
                        // 加入队列
                        self.event_queue.push_back(ScheduledEvent {
                            time: note_start_time,
                            event: MidiEvent::NoteOn {
                                track_index,
                                key: note.key,
                                velocity: note.velocity,
                            },
                        });
                        log::debug!("Queued note at time {} (playback_position: {})", note_start_time, self.playback_position);
                    }
                    self.processed_events.insert(event_key);
                }
            }
            
            // 检查音符是否应该结束
            if note_end_time >= min_time && note_end_time <= max_time {
                if note_end_time > self.playback_position && note_end_time <= max_time {
                    // 检查是否已经调度过这个事件（使用毫秒精度避免浮点误差）
                    let time_ms = (note_end_time * 1000.0) as u64;
                    let event_key = (track_id, note.key, time_ms);
                    if !self.processed_events.contains(&event_key) {
                        // 音符结束
                        self.event_queue.push_back(ScheduledEvent {
                            time: note_end_time,
                            event: MidiEvent::NoteOff {
                                track_index,
                                key: note.key,
                            },
                        });
                        self.processed_events.insert(event_key);
                    }
                }
            }
        }
    }

    /// 处理已调度的 MIDI 事件
    fn process_scheduled_events(&mut self) {
        // 处理所有已到时间的事件
        while let Some(event) = self.event_queue.front() {
            if event.time <= self.playback_position {
                let event = self.event_queue.pop_front().unwrap();
                
                // 发送事件到音频混合器
                self.mixer.send_event(event.event);
            } else {
                // 事件还未到时间，停止处理
                break;
            }
        }
    }

    /// 获取当前播放位置
    pub fn position(&self) -> f64 {
        self.playback_position
    }

    /// 检查是否正在播放
    pub fn is_playing(&self) -> bool {
        self.is_playing
    }
}

impl Default for MultiTrackPlaybackEngine {
    fn default() -> Self {
        Self::new(16)  // 默认 16 个轨道
    }
}

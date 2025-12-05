//! 多轨音频混合器
//!
//! 管理多个轨道音频引擎，混合它们的输出并发送到音频设备。

use crate::audio::track_audio_engine::{TrackAudioEngine, SineWaveTrackEngine};
use rodio::{OutputStream, OutputStreamHandle, Source};
use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::time::Duration;

/// MIDI 事件消息
#[derive(Clone, Copy, Debug)]
pub enum MidiEvent {
    NoteOn { track_index: usize, key: u8, velocity: u8 },
    NoteOff { track_index: usize, key: u8 },
    AllNotesOff { track_index: usize },
    SetTrackVolume { track_index: usize, volume: f32 },
    SetTrackPan { track_index: usize, pan: f32 },
}

/// 多轨音频混合器
pub struct AudioMixer {
    sender: mpsc::Sender<MidiEvent>,
    _stream: OutputStream,
    _handle: OutputStreamHandle,
    #[allow(dead_code)]
    sample_rate: u32,
    #[allow(dead_code)]
    channels: u16,
}

impl AudioMixer {
    /// 创建新的音频混合器
    pub fn new(sample_rate: u32, channels: u16, initial_track_count: usize) -> Self {
        let (_stream, handle) = OutputStream::try_default()
            .expect("无法初始化音频输出设备");
        
        let (sender, receiver) = mpsc::channel();
        
        // 创建轨道引擎（使用 Arc<Mutex<>> 以便在音频线程中安全访问）
        let mut track_engines: Vec<Arc<Mutex<dyn TrackAudioEngine + Send>>> = Vec::new();
        for _ in 0..initial_track_count {
            track_engines.push(Arc::new(Mutex::new(
                SineWaveTrackEngine::new(sample_rate)
            )));
        }
        
        // 创建音频源
        let source = MixerSource::new(
            receiver,
            track_engines,
            sample_rate,
            channels,
        );
        
        handle
            .play_raw(source.convert_samples())
            .expect("无法启动音频线程");
        
        Self {
            sender,
            _stream,
            _handle: handle,
            sample_rate,
            channels,
        }
    }
    
    /// 发送 MIDI 事件到音频线程
    pub fn send_event(&self, event: MidiEvent) {
        let _ = self.sender.send(event);
    }
    
    /// 触发音符开始
    #[allow(dead_code)]
    pub fn note_on(&self, track_index: usize, key: u8, velocity: u8) {
        self.send_event(MidiEvent::NoteOn {
            track_index,
            key,
            velocity,
        });
    }
    
    /// 触发音符结束
    #[allow(dead_code)]
    pub fn note_off(&self, track_index: usize, key: u8) {
        self.send_event(MidiEvent::NoteOff {
            track_index,
            key,
        });
    }
    
    /// 停止所有音符
    pub fn all_notes_off(&self, track_index: usize) {
        self.send_event(MidiEvent::AllNotesOff { track_index });
    }
    
    /// 设置轨道音量
    pub fn set_track_volume(&self, track_index: usize, volume: f32) {
        self.send_event(MidiEvent::SetTrackVolume {
            track_index,
            volume,
        });
    }
    
    /// 设置轨道声像
    pub fn set_track_pan(&self, track_index: usize, pan: f32) {
        self.send_event(MidiEvent::SetTrackPan {
            track_index,
            pan,
        });
    }
}

/// 音频源实现
struct MixerSource {
    receiver: mpsc::Receiver<MidiEvent>,
    track_engines: Vec<Arc<Mutex<dyn TrackAudioEngine + Send>>>,
    sample_rate: u32,
    channels: u16,
    master_volume: f32,
    sample_buffer: Vec<f32>,
    sample_index: usize,
}

impl MixerSource {
    fn new(
        receiver: mpsc::Receiver<MidiEvent>,
        track_engines: Vec<Arc<Mutex<dyn TrackAudioEngine + Send>>>,
        sample_rate: u32,
        channels: u16,
    ) -> Self {
        let _buffer_size = 1024 * channels as usize;
        Self {
            receiver,
            track_engines,
            sample_rate,
            channels,
            master_volume: 0.7,
            sample_buffer: Vec::new(),
            sample_index: 0,
        }
    }
    
    fn process_messages(&mut self) {
        while let Ok(event) = self.receiver.try_recv() {
            match event {
                MidiEvent::NoteOn { track_index, key, velocity } => {
                    // 动态扩展轨道引擎
                    while self.track_engines.len() <= track_index {
                        self.track_engines.push(Arc::new(Mutex::new(
                            SineWaveTrackEngine::new(self.sample_rate)
                        )));
                    }
                    if let Some(engine) = self.track_engines.get(track_index) {
                        if let Ok(mut engine) = engine.lock() {
                            engine.note_on(key, velocity);
                        }
                    }
                }
                MidiEvent::NoteOff { track_index, key } => {
                    // 动态扩展轨道引擎
                    while self.track_engines.len() <= track_index {
                        self.track_engines.push(Arc::new(Mutex::new(
                            SineWaveTrackEngine::new(self.sample_rate)
                        )));
                    }
                    if let Some(engine) = self.track_engines.get(track_index) {
                        if let Ok(mut engine) = engine.lock() {
                            engine.note_off(key);
                        }
                    }
                }
                MidiEvent::AllNotesOff { track_index } => {
                    // 动态扩展轨道引擎
                    while self.track_engines.len() <= track_index {
                        self.track_engines.push(Arc::new(Mutex::new(
                            SineWaveTrackEngine::new(self.sample_rate)
                        )));
                    }
                    if let Some(engine) = self.track_engines.get(track_index) {
                        if let Ok(mut engine) = engine.lock() {
                            engine.all_notes_off();
                        }
                    }
                }
                MidiEvent::SetTrackVolume { track_index, volume } => {
                    // 动态扩展轨道引擎
                    while self.track_engines.len() <= track_index {
                        self.track_engines.push(Arc::new(Mutex::new(
                            SineWaveTrackEngine::new(self.sample_rate)
                        )));
                    }
                    if let Some(engine) = self.track_engines.get(track_index) {
                        if let Ok(mut engine) = engine.lock() {
                            engine.set_volume(volume);
                        }
                    }
                }
                MidiEvent::SetTrackPan { track_index, pan } => {
                    // 动态扩展轨道引擎
                    while self.track_engines.len() <= track_index {
                        self.track_engines.push(Arc::new(Mutex::new(
                            SineWaveTrackEngine::new(self.sample_rate)
                        )));
                    }
                    if let Some(engine) = self.track_engines.get(track_index) {
                        if let Ok(mut engine) = engine.lock() {
                            engine.set_pan(pan);
                        }
                    }
                }
            }
        }
    }
}

impl Iterator for MixerSource {
    type Item = f32;
    
    fn next(&mut self) -> Option<Self::Item> {
        self.process_messages();
        
        // 如果缓冲区为空或已用完，生成新的音频帧
        if self.sample_index >= self.sample_buffer.len() {
            // 生成一帧音频（1024 个样本）
            let frame_size = 1024 * self.channels as usize;
            let mut frame_buffer = vec![0.0; frame_size];
            
            // 混合所有轨道的输出
            for engine in &self.track_engines {
                if let Ok(mut engine) = engine.lock() {
                    let mut track_output = vec![0.0; frame_size];
                    engine.process_audio(&mut track_output, self.sample_rate, self.channels);
                    
                    // 累加到帧缓冲区
                    for (frame, track) in frame_buffer.iter_mut().zip(track_output.iter()) {
                        *frame += *track;
                    }
                }
            }
            
            // 应用主音量并限制
            for sample in &mut frame_buffer {
                *sample = (*sample * self.master_volume).clamp(-1.0, 1.0);
            }
            
            // 更新缓冲区
            self.sample_buffer = frame_buffer;
            self.sample_index = 0;
        }
        
        // 返回下一个样本
        if self.sample_index < self.sample_buffer.len() {
            let sample = self.sample_buffer[self.sample_index];
            self.sample_index += 1;
            Some(sample)
        } else {
            Some(0.0)
        }
    }
}

impl Source for MixerSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    
    fn channels(&self) -> u16 {
        self.channels
    }
    
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

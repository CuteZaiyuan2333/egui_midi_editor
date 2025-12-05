//! 轨道音频引擎接口和实现

use crate::audio::sine_synth::{SineSynthConfig, Voice};
use std::collections::HashMap;

/// 轨道音频引擎接口
/// 
/// 每个轨道都有独立的音频引擎实例，可以独立控制音量、声像等参数。
/// 未来可以替换为 VST 插件引擎。
pub trait TrackAudioEngine: Send + Sync {
    /// 触发音符开始
    fn note_on(&mut self, key: u8, velocity: u8);
    
    /// 触发音符结束
    fn note_off(&mut self, key: u8);
    
    /// 停止所有音符
    fn all_notes_off(&mut self);
    
    /// 设置轨道音量 (0.0 - 1.0)
    fn set_volume(&mut self, volume: f32);
    
    /// 设置轨道声像 (-1.0 左, 0.0 中, 1.0 右)
    fn set_pan(&mut self, pan: f32);
    
    /// 处理音频输出
    /// 
    /// # 参数
    /// - `output`: 输出缓冲区（单声道或立体声交错）
    /// - `sample_rate`: 采样率
    /// - `channels`: 声道数（1=单声道, 2=立体声）
    fn process_audio(&mut self, output: &mut [f32], sample_rate: u32, channels: u16);
}

/// 正弦波合成器实现（当前实现）
pub struct SineWaveTrackEngine {
    voices: HashMap<u8, Voice>,
    volume: f32,
    pan: f32,
    sample_rate: u32,
    config: SineSynthConfig,
    max_voices: usize,
}

impl SineWaveTrackEngine {
    pub fn new(sample_rate: u32) -> Self {
        Self::with_config(sample_rate, SineSynthConfig::default())
    }
    
    pub fn with_config(sample_rate: u32, config: SineSynthConfig) -> Self {
        Self {
            voices: HashMap::new(),
            volume: 0.5,
            pan: 0.0,
            sample_rate,
            config,
            max_voices: config.max_voices,
        }
    }
}

impl TrackAudioEngine for SineWaveTrackEngine {
    fn note_on(&mut self, key: u8, velocity: u8) {
        // 如果该键已经有音符在播放，先移除
        self.voices.remove(&key);
        
        // 如果达到最大复音数，移除最旧的音符
        if self.voices.len() >= self.max_voices {
            if let Some(oldest_key) = self.voices.keys().next().copied() {
                self.voices.remove(&oldest_key);
            }
        }
        
        // 创建新的音符
        let voice = Voice::new(
            key,
            velocity,
            self.sample_rate,
            &self.config,
            0.0, // pitch_shift
        );
        self.voices.insert(key, voice);
    }
    
    fn note_off(&mut self, key: u8) {
        if let Some(voice) = self.voices.get_mut(&key) {
            voice.release();
        }
    }
    
    fn all_notes_off(&mut self) {
        self.voices.clear();
    }
    
    fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }
    
    fn set_pan(&mut self, pan: f32) {
        self.pan = pan.clamp(-1.0, 1.0);
    }
    
    fn process_audio(&mut self, output: &mut [f32], sample_rate: u32, channels: u16) {
        let frame_count = output.len() / channels as usize;
        
        for frame_idx in 0..frame_count {
            // 生成单声道样本
            let mut sample = 0.0;
            
            // 处理所有活跃的音符
            let mut keys_to_remove = Vec::new();
            for (key, voice) in &mut self.voices {
                sample += voice.next_sample(sample_rate);
                if voice.is_finished() {
                    keys_to_remove.push(*key);
                }
            }
            for key in keys_to_remove {
                self.voices.remove(&key);
            }
            
            // 应用音量
            sample *= self.volume;
            
            // 应用软限制，防止削波
            sample = sample.tanh() * 0.7;
            
            // 写入输出缓冲区
            if channels == 1 {
                // 单声道
                output[frame_idx] = sample;
            } else if channels == 2 {
                // 立体声，应用声像
                let left = sample * (1.0 - self.pan).max(0.0);
                let right = sample * (1.0 + self.pan).max(0.0);
                output[frame_idx * 2] = left;
                output[frame_idx * 2 + 1] = right;
            } else {
                // 多声道，复制到所有声道
                for ch in 0..channels {
                    output[frame_idx * channels as usize + ch as usize] = sample;
                }
            }
        }
    }
}


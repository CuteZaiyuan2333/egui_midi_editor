//! 多轨音频播放模块
//!
//! 提供每轨独立的音频引擎和音频混合功能。

mod track_audio_engine;
pub mod mixer;
mod sine_synth;

// 这些类型在内部模块中使用，不需要公开导出
// pub use track_audio_engine::{TrackAudioEngine, SineWaveTrackEngine};
// pub use mixer::{AudioMixer, MidiEvent};
// pub use sine_synth::{SineSynthConfig, Voice, Oscillator, AdsrEnvelope};


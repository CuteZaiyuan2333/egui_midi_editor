use crossbeam_channel::{unbounded, Receiver, Sender};
use rodio::{OutputStream, OutputStreamHandle, Source};
use std::time::Duration;

/// 宿主可替换的播放后端抽象。
pub trait PlaybackBackend {
    fn note_on(&self, key: u8, velocity: u8);
    fn note_off(&self, key: u8);
    fn all_notes_off(&self);
    fn set_volume(&self, volume: f32);
}

/// 默认的正弦波播放实现，提供多复音与 ADSR。
pub struct AudioEngine {
    _stream: OutputStream,
    _handle: OutputStreamHandle,
    sender: Sender<AudioMessage>,
}

impl AudioEngine {
    pub fn new() -> Self {
        Self::with_config(SineSynthConfig::default())
    }

    pub fn with_config(config: SineSynthConfig) -> Self {
        let (_stream, handle) = OutputStream::try_default().expect("无法初始化输出设备");
        let (sender, receiver) = unbounded();
        let synth = PolyphonicSynth::new(receiver, config);
        handle
            .play_raw(synth.convert_samples())
            .expect("无法启动音频线程");

        Self {
            _stream,
            _handle: handle,
            sender,
        }
    }

    fn dispatch(&self, msg: AudioMessage) {
        let _ = self.sender.send(msg);
    }
}

impl PlaybackBackend for AudioEngine {
    fn note_on(&self, key: u8, velocity: u8) {
        self.dispatch(AudioMessage::NoteOn { key, velocity });
    }

    fn note_off(&self, key: u8) {
        self.dispatch(AudioMessage::NoteOff { key });
    }

    fn all_notes_off(&self) {
        self.dispatch(AudioMessage::AllNotesOff);
    }

    fn set_volume(&self, volume: f32) {
        self.dispatch(AudioMessage::SetVolume(volume));
    }
}

/// 空实现，允许宿主禁用音频输出。
#[derive(Default)]
pub struct NullPlayback;

impl PlaybackBackend for NullPlayback {
    fn note_on(&self, _key: u8, _velocity: u8) {}
    fn note_off(&self, _key: u8) {}
    fn all_notes_off(&self) {}
    fn set_volume(&self, _volume: f32) {}
}

#[derive(Clone, Copy, Debug)]
pub struct SineSynthConfig {
    pub sample_rate: u32,
    pub max_voices: usize,
    pub attack_ms: f32,
    pub decay_ms: f32,
    pub sustain_level: f32,
    pub release_ms: f32,
}

impl Default for SineSynthConfig {
    fn default() -> Self {
        Self {
            sample_rate: 44_100,
            max_voices: 32,
            attack_ms: 8.0,
            decay_ms: 60.0,
            sustain_level: 0.75,
            release_ms: 150.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum AudioMessage {
    NoteOn { key: u8, velocity: u8 },
    NoteOff { key: u8 },
    AllNotesOff,
    SetVolume(f32),
}

struct PolyphonicSynth {
    receiver: Receiver<AudioMessage>,
    voices: Vec<Voice>,
    sample_rate: u32,
    volume: f32,
    config: SineSynthConfig,
}

impl PolyphonicSynth {
    fn new(receiver: Receiver<AudioMessage>, config: SineSynthConfig) -> Self {
        Self {
            receiver,
            voices: Vec::new(),
            sample_rate: config.sample_rate,
            volume: 0.5,
            config,
        }
    }

    fn process_messages(&mut self) {
        while let Ok(msg) = self.receiver.try_recv() {
            match msg {
                AudioMessage::NoteOn { key, velocity } => {
                    self.voices.retain(|v| v.key != key);
                    if self.voices.len() >= self.config.max_voices {
                        self.voices.remove(0);
                    }
                    self.voices.push(Voice::new(key, velocity, self.sample_rate, &self.config));
                }
                AudioMessage::NoteOff { key } => {
                    for voice in &mut self.voices {
                        if voice.key == key {
                            voice.release();
                        }
                    }
                }
                AudioMessage::AllNotesOff => {
                    self.voices.clear();
                }
                AudioMessage::SetVolume(vol) => {
                    self.volume = vol.clamp(0.0, 2.0);
                }
            }
        }
    }
}

impl Iterator for PolyphonicSynth {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        self.process_messages();

        if self.voices.is_empty() {
            return Some(0.0);
        }

        let mut mix = 0.0;
        self.voices.retain_mut(|voice| {
            mix += voice.next_sample(self.sample_rate);
            !voice.is_finished()
        });

        Some((mix * self.volume * 0.7).tanh())
    }
}

impl Source for PolyphonicSynth {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

struct Voice {
    osc: Oscillator,
    env: AdsrEnvelope,
    key: u8,
}

impl Voice {
    fn new(key: u8, velocity: u8, sample_rate: u32, config: &SineSynthConfig) -> Self {
        Self {
            osc: Oscillator::new(key, velocity),
            env: AdsrEnvelope::new(sample_rate, config),
            key,
        }
    }

    fn next_sample(&mut self, sample_rate: u32) -> f32 {
        let env = self.env.next();
        let osc = self.osc.next_sample(sample_rate);
        env * osc
    }

    fn release(&mut self) {
        self.env.trigger_release();
    }

    fn is_finished(&self) -> bool {
        self.env.is_idle()
    }
}

struct Oscillator {
    phase: f32,
    frequency: f32,
    velocity: f32,
}

impl Oscillator {
    fn new(key: u8, velocity: u8) -> Self {
        let frequency = 440.0 * 2.0f32.powf((key as f32 - 69.0) / 12.0);
        Self {
            phase: 0.0,
            frequency,
            velocity: velocity as f32 / 127.0,
        }
    }

    fn next_sample(&mut self, sample_rate: u32) -> f32 {
        let sample = (self.phase * 2.0 * std::f32::consts::PI).sin() * self.velocity;
        self.phase += self.frequency / sample_rate as f32;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        sample
    }
}

struct AdsrEnvelope {
    stage: EnvelopeStage,
    level: f32,
    attack_step: f32,
    decay_step: f32,
    sustain_level: f32,
    release_step: f32,
}

impl AdsrEnvelope {
    fn new(sample_rate: u32, config: &SineSynthConfig) -> Self {
        Self {
            stage: EnvelopeStage::Attack,
            level: 0.0,
            attack_step: step_size(config.attack_ms, sample_rate),
            decay_step: step_size(config.decay_ms, sample_rate),
            sustain_level: config.sustain_level.clamp(0.0, 1.0),
            release_step: step_size(config.release_ms, sample_rate),
        }
    }

    fn next(&mut self) -> f32 {
        match self.stage {
            EnvelopeStage::Attack => {
                self.level += self.attack_step;
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = EnvelopeStage::Decay;
                }
            }
            EnvelopeStage::Decay => {
                self.level -= self.decay_step;
                if self.level <= self.sustain_level {
                    self.level = self.sustain_level;
                    self.stage = EnvelopeStage::Sustain;
                }
            }
            EnvelopeStage::Sustain => {}
            EnvelopeStage::Release => {
                self.level -= self.release_step;
                if self.level <= 0.0 {
                    self.level = 0.0;
                    self.stage = EnvelopeStage::Idle;
                }
            }
            EnvelopeStage::Idle => {
                self.level = 0.0;
            }
        }
        self.level
    }

    fn trigger_release(&mut self) {
        if !matches!(self.stage, EnvelopeStage::Release | EnvelopeStage::Idle) {
            self.stage = EnvelopeStage::Release;
        }
    }

    fn is_idle(&self) -> bool {
        matches!(self.stage, EnvelopeStage::Idle)
    }
}

#[derive(Clone, Copy, Debug)]
enum EnvelopeStage {
    Attack,
    Decay,
    Sustain,
    Release,
    Idle,
}

fn step_size(ms: f32, sample_rate: u32) -> f32 {
    if ms <= 0.0 {
        return 1.0;
    }
    let samples = (ms / 1000.0) * sample_rate as f32;
    if samples <= 1.0 {
        1.0
    } else {
        1.0 / samples
    }
}

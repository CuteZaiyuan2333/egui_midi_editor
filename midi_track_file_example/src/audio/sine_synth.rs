//! 正弦波合成器实现
//!
//! 从 egui_midi 提取的合成器组件，用于每轨独立的音频生成。

#[derive(Clone, Copy, Debug)]
pub struct SineSynthConfig {
    #[allow(dead_code)]
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

pub struct Voice {
    osc: Oscillator,
    env: AdsrEnvelope,
    #[allow(dead_code)]
    key: u8,
}

impl Voice {
    pub fn new(
        key: u8,
        velocity: u8,
        sample_rate: u32,
        config: &SineSynthConfig,
        pitch_shift: f32,
    ) -> Self {
        let mut osc = Oscillator::new(key, velocity);
        osc.set_pitch_shift(pitch_shift);
        Self {
            osc,
            env: AdsrEnvelope::new(sample_rate, config),
            key,
        }
    }

    pub fn next_sample(&mut self, sample_rate: u32) -> f32 {
        let env = self.env.next();
        let osc = self.osc.next_sample(sample_rate);
        env * osc
    }

    pub fn release(&mut self) {
        self.env.trigger_release();
    }

    pub fn is_finished(&self) -> bool {
        self.env.is_idle()
    }

    #[allow(dead_code)]
    pub fn set_pitch_shift(&mut self, semitones: f32) {
        self.osc.set_pitch_shift(semitones);
    }
}

pub struct Oscillator {
    phase: f32,
    base_frequency: f32,
    frequency: f32,
    velocity: f32,
}

impl Oscillator {
    pub fn new(key: u8, velocity: u8) -> Self {
        let frequency = 440.0 * 2.0f32.powf((key as f32 - 69.0) / 12.0);
        Self {
            phase: 0.0,
            base_frequency: frequency,
            frequency,
            velocity: velocity as f32 / 127.0,
        }
    }

    pub fn next_sample(&mut self, sample_rate: u32) -> f32 {
        let sample = (self.phase * 2.0 * std::f32::consts::PI).sin() * self.velocity;
        self.phase += self.frequency / sample_rate as f32;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        sample
    }

    pub fn set_pitch_shift(&mut self, semitones: f32) {
        let ratio = 2.0f32.powf(semitones / 12.0);
        self.frequency = self.base_frequency * ratio;
    }
}

pub struct AdsrEnvelope {
    stage: EnvelopeStage,
    level: f32,
    attack_step: f32,
    decay_step: f32,
    sustain_level: f32,
    release_step: f32,
}

impl AdsrEnvelope {
    pub fn new(sample_rate: u32, config: &SineSynthConfig) -> Self {
        Self {
            stage: EnvelopeStage::Attack,
            level: 0.0,
            attack_step: step_size(config.attack_ms, sample_rate),
            decay_step: step_size(config.decay_ms, sample_rate),
            sustain_level: config.sustain_level.clamp(0.0, 1.0),
            release_step: step_size(config.release_ms, sample_rate),
        }
    }

    pub fn next(&mut self) -> f32 {
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

    pub fn trigger_release(&mut self) {
        if !matches!(self.stage, EnvelopeStage::Release | EnvelopeStage::Idle) {
            self.stage = EnvelopeStage::Release;
        }
    }

    pub fn is_idle(&self) -> bool {
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


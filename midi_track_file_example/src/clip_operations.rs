//! 剪辑操作模块
//!
//! 处理 MIDI 编辑器与轨道剪辑之间的双向关联和数据交换。

use egui_track::{Clip, MidiClipData};
use egui_midi::{ui::MidiEditor, structure::MidiState};
use std::result::Result;
use std::io;

/// 从 MIDI 编辑器导出到剪辑
#[allow(dead_code)]
pub fn export_midi_to_clip(
    editor: &MidiEditor,
    clip: &mut Clip,
) -> Result<(), io::Error> {
    // 获取编辑器的 MIDI 状态
    let midi_state = editor.snapshot_state();
    
    // 计算剪辑持续时间（基于 MIDI 数据）
    let duration = calculate_midi_duration(&midi_state);
    
    // 更新剪辑的持续时间
    clip.duration = duration;
    
    // 更新剪辑的 MIDI 数据
    if let egui_track::ClipType::Midi { ref mut midi_data } = clip.clip_type {
        let midi_data = midi_data.get_or_insert_with(|| MidiClipData {
            midi_file_path: None,
            preview_notes: Vec::new(),
            midi_state: None,
        });
        
        // 保存完整的 MIDI 状态
        midi_data.midi_state = Some(midi_state.clone());
        
        // 更新预览音符（用于快速预览）
        midi_data.preview_notes = generate_preview_notes(&midi_state);
    } else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Clip is not a MIDI clip"
        ));
    }
    
    Ok(())
}

/// 从剪辑加载到 MIDI 编辑器
#[allow(dead_code)]
pub fn load_clip_to_midi_editor(
    clip: &Clip,
    editor: &mut MidiEditor,
) -> Result<(), io::Error> {
    if let egui_track::ClipType::Midi { ref midi_data } = clip.clip_type {
        if let Some(midi_data) = midi_data {
            // 只从文件路径加载
            if let Some(ref file_path) = midi_data.midi_file_path {
                let path = std::path::Path::new(file_path);
                if path.exists() {
                    match crate::midiclip::load_midiclip_file(path) {
                        Ok(state) => {
                            editor.replace_state(state);
                            return Ok(());
                        }
                        Err(e) => {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                format!("Failed to load MIDI file: {:?}", e)
                            ));
                        }
                    }
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::NotFound,
                        format!("MIDI file not found: {:?}", file_path)
                    ));
                }
            }
        }
    }
    
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "Clip does not contain MIDI file path"
    ))
}

/// 计算 MIDI 数据的持续时间（秒）
#[allow(dead_code)]
fn calculate_midi_duration(midi_state: &MidiState) -> f64 {
    if midi_state.notes.is_empty() {
        return 4.0; // 默认 4 秒
    }
    
    // 找到最后一个音符的结束时间
    let max_end_tick = midi_state.notes.iter()
        .map(|note| note.start + note.duration)
        .max()
        .unwrap_or(0);
    
    // 转换为秒
    ticks_to_seconds(max_end_tick, midi_state.bpm, midi_state.ticks_per_beat)
}

/// 生成预览音符（用于快速预览）
fn generate_preview_notes(midi_state: &MidiState) -> Vec<egui_track::PreviewNote> {
    midi_state.notes.iter()
        .map(|note| {
            let start_seconds = ticks_to_seconds(note.start, midi_state.bpm, midi_state.ticks_per_beat);
            let duration_seconds = ticks_to_seconds(note.duration, midi_state.bpm, midi_state.ticks_per_beat);
            
            egui_track::PreviewNote {
                start: start_seconds,
                duration: duration_seconds,
                key: note.key,
                velocity: note.velocity,
            }
        })
        .collect()
}

/// MIDI ticks 转换为秒
pub fn ticks_to_seconds(ticks: u64, bpm: f32, ticks_per_beat: u16) -> f64 {
    let beats = ticks as f64 / ticks_per_beat as f64;
    beats * 60.0 / bpm as f64
}

/// 秒转换为 MIDI ticks
#[allow(dead_code)]
pub fn seconds_to_ticks(seconds: f64, bpm: f32, ticks_per_beat: u16) -> u64 {
    let beats = seconds * bpm as f64 / 60.0;
    (beats * ticks_per_beat as f64) as u64
}

/// 从 .midiclip 文件生成预览音符数据
/// 
/// 这个函数从文件中加载 MIDI 数据并生成预览音符。
/// 为了性能考虑，可能会限制预览音符的数量。
pub fn generate_preview_notes_from_file(file_path: &std::path::Path) -> Result<Vec<egui_track::PreviewNote>, io::Error> {
    // 加载 MIDI 文件
    let midi_state = crate::midiclip::load_midiclip_file(file_path)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Failed to load MIDI file: {:?}", e)))?;
    
    // 生成预览音符
    let preview_notes = generate_preview_notes(&midi_state);
    
    // 性能优化：如果音符太多，进行采样
    const MAX_PREVIEW_NOTES: usize = 1000;
    let preview_notes = if preview_notes.len() > MAX_PREVIEW_NOTES {
        log::warn!("Too many notes ({}), sampling to {} for preview", preview_notes.len(), MAX_PREVIEW_NOTES);
        // 均匀采样
        let step = preview_notes.len() / MAX_PREVIEW_NOTES;
        preview_notes.into_iter()
            .enumerate()
            .filter(|(i, _)| i % step == 0)
            .map(|(_, note)| note)
            .take(MAX_PREVIEW_NOTES)
            .collect()
    } else {
        preview_notes
    };
    
    Ok(preview_notes)
}


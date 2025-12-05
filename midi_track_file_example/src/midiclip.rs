//! MIDI Clip 文件管理模块
//!
//! 处理 .midiclip 文件的创建、转换、加载、保存等操作。
//! .midiclip 文件使用标准 MIDI 格式（.mid）存储。

use egui_midi::structure::MidiState;
use std::path::{Path, PathBuf};
use std::fs;
use std::io;

/// 创建新的 .midiclip 文件（包含默认的 MIDI 数据）
pub fn create_midiclip_file(path: &Path) -> Result<PathBuf, io::Error> {
    // 确保路径有 .midiclip 扩展名
    let mut file_path = path.to_path_buf();
    if file_path.extension().and_then(|s| s.to_str()) != Some("midiclip") {
        file_path.set_extension("midiclip");
    }
    
    // 创建默认的 MIDI 状态
    let default_state = MidiState::default();
    
    // 保存到文件
    save_midiclip_file(&file_path, &default_state)?;
    
    Ok(file_path)
}

/// 将 .mid 文件转换为 .midiclip（复制并重命名）
pub fn convert_mid_to_midiclip(mid_path: &Path) -> Result<PathBuf, io::Error> {
    // 验证文件是单轨 MIDI
    let data = fs::read(mid_path)?;
    let smf = midly::Smf::parse(&data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Invalid MIDI file: {:?}", e)))?;
    
    // 验证是单轨
    MidiState::from_smf_strict(&smf)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Not a single-track MIDI file: {}", e)))?;
    
    // 创建 .midiclip 文件路径
    let mut midiclip_path = mid_path.to_path_buf();
    midiclip_path.set_extension("midiclip");
    
    // 复制文件
    fs::copy(mid_path, &midiclip_path)?;
    
    Ok(midiclip_path)
}

/// 从 .midiclip 文件加载 MIDI 数据
pub fn load_midiclip_file(path: &Path) -> Result<MidiState, io::Error> {
    let data = fs::read(path)?;
    let smf = midly::Smf::parse(&data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Invalid MIDI file: {:?}", e)))?;
    
    MidiState::from_smf_strict(&smf)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Not a single-track MIDI file: {}", e)))
}

/// 保存 MIDI 数据到 .midiclip 文件（标准 MIDI 格式）
pub fn save_midiclip_file(path: &Path, state: &MidiState) -> Result<(), io::Error> {
    // 确保目录存在
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    
    // 导出为标准 MIDI 格式
    let smf = state.to_single_track_smf()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Failed to export MIDI: {}", e)))?;
    
    // 写入缓冲区
    let mut buffer = Vec::new();
    smf.write_std(&mut buffer)
        .map_err(|e| io::Error::new(io::ErrorKind::WriteZero, format!("Failed to encode MIDI file: {:?}", e)))?;
    
    // 写入文件
    fs::write(path, buffer)?;
    
    Ok(())
}

/// 检查文件是否是 .midiclip 文件
pub fn is_midiclip_file(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|s| s.eq_ignore_ascii_case("midiclip"))
        .unwrap_or(false)
}

/// 检查文件是否是 .mid 或 .midi 文件
pub fn is_midi_file(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|s| s.eq_ignore_ascii_case("mid") || s.eq_ignore_ascii_case("midi"))
        .unwrap_or(false)
}


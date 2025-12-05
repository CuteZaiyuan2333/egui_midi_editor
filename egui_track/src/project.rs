//! 项目文件系统模块
//!
//! 处理项目的保存和加载，管理项目目录结构。

use crate::structure::{Track, TimelineState};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use std::io;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectFile {
    pub version: String,
    pub timeline: TimelineState,
    pub tracks: Vec<Track>,
}

impl ProjectFile {
    pub fn new(timeline: TimelineState, tracks: Vec<Track>) -> Self {
        Self {
            version: "1.0".to_string(),
            timeline,
            tracks,
        }
    }

    /// 保存项目到指定路径
    /// 项目结构：
    /// - <项目名称>/<项目名称>.json - 项目配置文件
    /// - <项目名称>/midi/ - MIDI剪辑文件夹
    /// - <项目名称>/audio/ - 音频剪辑文件夹
    /// - <项目名称>/export/ - 导出文件夹
    pub fn save_to_path(&self, project_path: &Path) -> Result<(), io::Error> {
        // 判断 project_path 是文件还是目录
        // 如果有扩展名（如 .json），则认为是文件路径
        let project_dir = if project_path.extension().is_some() {
            // 是文件路径，获取父目录作为项目目录
            project_path.parent().unwrap_or(Path::new("."))
        } else {
            // 是目录路径，直接使用
            project_path
        };
        
        // 确保项目目录存在
        fs::create_dir_all(project_dir)?;

        // 创建子文件夹
        let midi_dir = project_dir.join("midi");
        let audio_dir = project_dir.join("audio");
        let export_dir = project_dir.join("export");

        fs::create_dir_all(&midi_dir)?;
        fs::create_dir_all(&audio_dir)?;
        fs::create_dir_all(&export_dir)?;

        // 确定项目文件路径（支持 .tracks 和 .json 扩展名）
        let json_path = if project_path.extension().is_some() {
            project_path.to_path_buf()
        } else {
            // 如果没有扩展名，使用项目目录名作为文件名，默认使用 .tracks
            let project_name = project_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("project");
            project_dir.join(format!("{}.tracks", project_name))
        };

        // 序列化并保存JSON文件
        let json_content = serde_json::to_string_pretty(self)?;
        fs::write(&json_path, json_content)?;

        Ok(())
    }

    /// 从指定路径加载项目
    pub fn load_from_path(project_path: &Path) -> Result<Self, io::Error> {
        // 读取JSON文件
        let json_content = fs::read_to_string(project_path)?;
        
        // 反序列化
        let project: ProjectFile = serde_json::from_str(&json_content)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("JSON解析错误: {}", e)))?;

        // 验证目录结构（可选，不强制要求）
        let project_dir = project_path.parent().unwrap_or(Path::new("."));
        let midi_dir = project_dir.join("midi");
        let audio_dir = project_dir.join("audio");
        let export_dir = project_dir.join("export");

        // 如果目录不存在，创建它们（向后兼容）
        if !midi_dir.exists() {
            fs::create_dir_all(&midi_dir)?;
        }
        if !audio_dir.exists() {
            fs::create_dir_all(&audio_dir)?;
        }
        if !export_dir.exists() {
            fs::create_dir_all(&export_dir)?;
        }

        Ok(project)
    }

    /// 获取项目目录路径
    pub fn get_project_dir(project_path: &Path) -> PathBuf {
        if project_path.is_file() {
            project_path.parent().unwrap_or(Path::new(".")).to_path_buf()
        } else {
            project_path.to_path_buf()
        }
    }

    /// 获取MIDI文件夹路径
    pub fn get_midi_dir(project_path: &Path) -> PathBuf {
        Self::get_project_dir(project_path).join("midi")
    }

    /// 获取音频文件夹路径
    pub fn get_audio_dir(project_path: &Path) -> PathBuf {
        Self::get_project_dir(project_path).join("audio")
    }

    /// 获取导出文件夹路径
    pub fn get_export_dir(project_path: &Path) -> PathBuf {
        Self::get_project_dir(project_path).join("export")
    }
}

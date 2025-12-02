//! # egui_track
//!
//! 一个用于 DAW（数字音频工作站）风格的音轨编辑器组件库。
//!
//! ## 功能特性
//!
//! - **多轨管理**：支持多个音轨的创建、删除、重排序
//! - **剪辑片段编辑**：支持 MIDI 和音频剪辑的创建、移动、调整大小、分割
//! - **时间轴操作**：时间轴缩放、滚动、播放头控制
//! - **交互操作**：拖拽、选择、多选、网格对齐
//!
//! ## 基本使用
//!
//! ```rust
//! use egui_track::{TrackEditor, TrackEditorOptions};
//!
//! let mut editor = TrackEditor::new(TrackEditorOptions::default());
//!
//! // 在 egui UI 中使用
//! editor.ui(ui);
//! ```
//!
//! ## 集成到宿主应用
//!
//! ```rust
//! use egui_track::{TrackEditor, TrackEditorEvent, TrackEditorCommand};
//!
//! // 创建编辑器
//! let mut track_editor = TrackEditor::new(TrackEditorOptions::default());
//!
//! // 设置事件监听器
//! track_editor.set_event_listener(Box::new(|event| {
//!     match event {
//!         TrackEditorEvent::ClipDoubleClicked { clip_id } => {
//!             // 打开 MIDI 编辑器
//!             open_midi_editor(clip_id);
//!         }
//!         _ => {}
//!     }
//! }));
//!
//! // 在 UI 中渲染
//! track_editor.ui(ui);
//!
//! // 处理命令
//! track_editor.execute_command(TrackEditorCommand::CreateClip { ... });
//! ```

pub mod structure;
pub mod editor;
pub mod ui;
pub mod project;

pub use structure::{Track, Clip, TrackId, ClipId, TimelineState, ClipType, MidiClipData, AudioClipData, PreviewNote};
pub use editor::{TrackEditorCommand, TrackEditorEvent};
pub use ui::{TrackEditor, TrackEditorOptions};
pub use project::ProjectFile;

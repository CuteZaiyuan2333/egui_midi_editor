//! # egui_file_tree
//!
//! 一个用于显示文件系统目录树的egui组件库。
//!
//! ## 功能特性
//!
//! - **树状显示**：以树状图方式显示文件目录结构
//! - **展开/折叠**：支持展开和折叠文件夹
//! - **文件类型区分**：区分显示文件和文件夹
//! - **选择支持**：支持选择文件或文件夹
//! - **双击事件**：支持双击事件，由使用方处理文件打开
//!
//! ## 基本使用
//!
//! ```rust
//! use egui_file_tree::FileTree;
//! use std::path::PathBuf;
//!
//! let mut file_tree = FileTree::new(PathBuf::from("/path/to/directory"));
//!
//! // 在 egui UI 中使用
//! let events = file_tree.ui(ui);
//! for event in events {
//!     match event {
//!         FileTreeEvent::PathSelected { path } => {
//!             println!("Selected: {:?}", path);
//!         }
//!         FileTreeEvent::PathDoubleClicked { path } => {
//!             println!("Double clicked: {:?}", path);
//!             // 处理文件打开
//!         }
//!     }
//! }
//! ```

mod tree;

pub use tree::{FileTree, FileTreeEvent};


# egui MIDI Editor

A modern, lightweight single-track MIDI editor library built with Rust and egui framework. Designed to be seamlessly integrated into DAW (Digital Audio Workstation) software, providing a focused and performant MIDI editing experience.

## 🎯 Project Goals

This library focuses on **single-track MIDI editing** and is designed to serve as a component within larger DAW applications. The primary objectives are:

- **Developer Experience**: Provide an elegant and intuitive API for easy integration
- **Practical Workflow**: Ship the common tools (selection, quantize-to-grid, clipboard, undo/redo) needed to embed a usable piano roll
- **Performance**: Optimized for handling large MIDI files efficiently
- **Simplicity**: Focused on single-track editing without unnecessary complexity

## 🎵 Features

### 核心编辑功能
- **Visual MIDI Editor**: Intuitive piano roll interface for editing *one track at a time*
  - 音符创建、选择、拖拽移动
  - 音符长度调整（拖拽右边缘）
  - 多选支持（Ctrl/Cmd + 点击，Shift + 点击扩展选择）
  - 框选（拖拽选择区域）
  - 吸附到网格（可配置吸附间隔和模式）
  
- **Inspector & Clipboard**: 
  - 多选音符的属性编辑（音高、力度、开始时间、持续时间）
  - 复制/剪切/粘贴（Ctrl/Cmd + C/X/V）
  - 删除选中音符（Delete/Backspace）
  - 量化到网格（Quantize to snap grid）
  
- **Undo / Redo Stack**: 
  - 完整的撤销/重做系统，记录所有编辑操作
  - 键盘快捷键：Ctrl/Cmd + Z（撤销），Ctrl/Cmd + Shift + Z 或 Ctrl/Cmd + Y（重做）

### 曲线编辑功能
- **Velocity Curve（力度曲线）**:
  - 可视化力度曲线编辑
  - 添加、编辑、删除曲线点
  - 线性插值计算力度值
  - 曲线值范围：0-127
  - 导出MIDI时自动应用曲线到音符力度
  
- **Pitch Curve（音高曲线）**:
  - 音高偏移曲线编辑（支持半音偏移）
  - 曲线值范围：-12 到 +12 半音
  - 与力度曲线相同的编辑功能
  
- **曲线编辑器界面**:
  - 可调整的分割器（Splitter）调整钢琴卷帘和曲线编辑器的高度比例
  - 曲线通道的启用/禁用切换
  - 实时预览曲线效果

### 音频播放功能
- **Real-time Audio Playback**: 
  - 内置音频引擎，支持实时预览
  - ADSR包络合成（Attack, Decay, Sustain, Release）
  - 音高偏移预览（Pitch Shift Preview）
  - 音量控制
  - 可插拔的音频后端接口（`PlaybackBackend`），支持集成到DAW的音频系统

### 传输控制
- **Transport Controls**: 
  - 播放/暂停（Space键或程序控制）
  - BPM控制（可设置和实时调整）
  - 时间轴定位（Seek）
  - 循环播放支持（Loop regions，可配置开始和结束位置）
  - 时间签名设置（Time Signature）

### 文件I/O
- **Strict Single-Track I/O**: 
  - MIDI文件导入/导出（使用midly库）
  - 单轨验证（`from_smf_strict` 确保单轨单通道）
  - `.aquamidi` 项目格式支持（示例应用）
  - 标准`.mid`文件导出

### 开发者API
- **Developer-Friendly API**: 
  - 事件/命令总线系统（`EditorEvent` / `EditorCommand`）
  - 严格验证辅助函数
  - 播放观察者接口（`PlaybackObserver`）
  - 可自定义选项（`MidiEditorOptions`）
  - 事件监听器（`set_event_listener`）

### 其他特性
- **Cross-platform**: Works on Windows, macOS, and Linux
- **Modular Architecture**: Clean separation between UI, audio, and MIDI processing
- **Keyboard Shortcuts**: 
  - `Space`: 播放/暂停
  - `Ctrl/Cmd + C`: 复制
  - `Ctrl/Cmd + X`: 剪切
  - `Ctrl/Cmd + V`: 粘贴
  - `Ctrl/Cmd + Z`: 撤销
  - `Ctrl/Cmd + Shift + Z` 或 `Ctrl/Cmd + Y`: 重做
  - `Delete` / `Backspace`: 删除选中音符

## 🏗️ Architecture

The project is organized as a Rust workspace with two main components:

### `egui_midi` (Library)
Core MIDI editor library containing:
- **structure.rs**: MIDI data structures and file I/O operations
- **audio.rs**: Audio engine with polyphonic synthesis and ADSR envelopes
- **ui/mod.rs**: Complete egui-based MIDI editor interface

### `example_app` (Demo Application)
A demonstration application showcasing the library's capabilities with a functional MIDI editor interface.

## 🚀 Quick Start

### Prerequisites
- Rust 1.70+ (install from [rustup.rs](https://rustup.rs/))
- Audio output device for playback functionality

### Building and Running
```bash
# Clone the repository
git clone https://github.com/CuteZaiyuan2333/egui_midi_editor.git
cd egui_midi_editor

# Build the project
cargo build --release

# Run the example application
cargo run --release -p example_app

# Note: the demo opens/saves `.aquamidi` single-track projects and can export standard `.mid` files.
```

## 🎹 Usage

### Basic Integration

The library is designed to be easily integrated into your DAW application:

```rust
use egui_midi::MidiEditor;
use egui_midi::audio::{AudioEngine, PlaybackBackend, PlaybackObserver};
use egui_midi::editor::{EditorCommand, EditorEvent};
use egui_midi::structure::{MidiState, Note};
use std::sync::Arc;

// Create audio engine (optional - can be None if you handle audio externally)
let audio: Arc<dyn PlaybackBackend> = Arc::new(AudioEngine::new());

// Initialize MIDI editor
let mut editor = MidiEditor::new(Some(audio));

// Add notes to the single track
editor.insert_note(Note::new(0, 480, 60, 100)); // C4 quarter note

// Observe editor events (state diffs, playback, selection, etc.)
editor.set_event_listener(|event| match event {
    EditorEvent::StateReplaced(state) => {
        // Persist or display the new MidiState
        log::info!("state now contains {} notes", state.notes.len());
    }
    EditorEvent::PlaybackStateChanged { is_playing } => {
        log::info!("transport {}", if is_playing { "started" } else { "stopped" });
    }
    _ => {}
});

// Optional: hook start/stop notifications independent of MIDI events
struct TransportHook;
impl PlaybackObserver for TransportHook {
    fn on_playback_started(&self) {
        log::info!("audio preview engaged");
    }
    fn on_playback_stopped(&self) {
        log::info!("audio preview halted");
    }
}
editor.set_playback_observer(Some(Arc::new(TransportHook)));

// Drive editor actions from your host logic
editor.apply_command(EditorCommand::SeekSeconds(4.0));
editor.apply_command(EditorCommand::SetPlayback(true));

// Render the editor in your egui UI
editor.ui(ui);
```

### Example App File Menu

The bundled `example_app` includes a File menu (New/Open/Save/Save As/Export MIDI) that operates on a custom single-track project format with the `.aquamidi` extension. `.aquamidi` files wrap a validated single-track SMF payload plus a lightweight header, ensuring demos stay aligned with the library’s “one track per editor” constraint. Use Export MIDI to write a standard `.mid` file that any DAW can open; importing `.mid` directly in the demo is not supported yet, so convert via your host application if needed.

### Strict Single-Track MIDI I/O

```rust
use egui_midi::structure::{MidiState, MidiValidationError};

// Import with validation (enforces single track + single channel)
let smf = midly::Smf::parse(bytes)?;
let state = MidiState::from_smf_strict(&smf)?;

// Mutate editor state...

// Export with the same guarantees
let smf = editor.state.to_single_track_smf()?;
```

### Custom Audio Backend

For DAW integration, you can implement your own audio backend to use your existing audio system:

```rust
use egui_midi::audio::PlaybackBackend;

pub struct DawAudioBackend {
    // Your DAW's audio engine
}

impl PlaybackBackend for DawAudioBackend {
    fn note_on(&self, key: u8, velocity: u8) {
        // Forward to your DAW's audio engine
    }
    
    fn note_off(&self, key: u8) {
        // Forward to your DAW's audio engine
    }
    
    fn all_notes_off(&self) {
        // Stop all notes in your DAW
    }
    
    fn set_volume(&self, volume: f32) {
        // Set volume in your DAW
    }

    fn set_pitch_shift(&self, semitones: f32) {
        // Optional: adapt preview detune / resample rate
    }
}
```

### Integration Best Practices

- **Single Track Focus**: This library handles one MIDI track at a time. For multi-track DAWs, create multiple `MidiEditor` instances
- **Audio Backend**: Use `None` if your DAW already handles audio, or implement `PlaybackBackend` + (optionally) `PlaybackObserver` to integrate with your audio system
- **State Management**: The editor maintains its own state, making it easy to embed in larger applications
- **Events & Commands**: Subscribe via `set_event_listener` to react to user edits, and use `apply_command` to drive transport/selection from your host
- **Embedding Checklist**: See [docs/embedding.md](docs/embedding.md) for a step-by-step guide

## 📝 已实现功能详细列表

### 音符编辑
- ✅ 点击空白区域创建新音符
- ✅ 点击音符进行选择
- ✅ 拖拽音符移动位置
- ✅ 拖拽音符右边缘调整长度
- ✅ Ctrl/Cmd + 点击：切换选择
- ✅ Shift + 点击：扩展选择
- ✅ 拖拽框选多个音符
- ✅ 吸附到网格（Snap to grid）
- ✅ 吸附模式：绝对模式（Absolute）和相对模式（Relative）

### 剪贴板操作
- ✅ 复制选中音符（Ctrl/Cmd + C）
- ✅ 剪切选中音符（Ctrl/Cmd + X）
- ✅ 粘贴音符（Ctrl/Cmd + V）
- ✅ 删除选中音符（Delete/Backspace）

### 撤销/重做
- ✅ 完整的操作历史记录
- ✅ 撤销（Ctrl/Cmd + Z）
- ✅ 重做（Ctrl/Cmd + Shift + Z 或 Ctrl/Cmd + Y）

### 检查器面板
- ✅ 显示选中音符的属性
- ✅ 编辑音高（Key）
- ✅ 编辑力度（Velocity）
- ✅ 编辑开始时间（Start）
- ✅ 编辑持续时间（Duration）
- ✅ 多选时批量编辑

### 曲线编辑
- ✅ 力度曲线（Velocity Curve）
  - 添加曲线点（点击曲线区域）
  - 拖拽曲线点调整位置和值
  - 删除曲线点（右键点击或Delete键）
  - 线性插值计算
  - 导出时自动应用到音符
- ✅ 音高曲线（Pitch Curve）
  - 与力度曲线相同的编辑功能
  - 支持-12到+12半音偏移
- ✅ 曲线通道管理
  - 启用/禁用曲线通道
  - 可调整的分割器调整界面布局

### 音频播放
- ✅ 实时音频预览
- ✅ ADSR包络合成
- ✅ 音量控制
- ✅ 音高偏移预览
- ✅ 可插拔音频后端接口

### 传输控制
- ✅ 播放/暂停（Space键）
- ✅ BPM设置和调整
- ✅ 时间签名设置
- ✅ 时间轴定位（Seek）
- ✅ 循环播放配置

### 文件操作
- ✅ 导入MIDI文件（单轨验证）
- ✅ 导出MIDI文件
- ✅ `.aquamidi` 项目格式（示例应用）
- ✅ 标准`.mid`文件导出

### 视图控制
- ✅ 水平/垂直缩放
- ✅ 滚动视图
- ✅ 定位到指定音高
- ✅ 可调整的曲线编辑器高度

## ⚠️ Current Limitations

- **严格单轨限制**: 验证拒绝多轨或混合通道的SMF文件
- **示例应用限制**: 示例应用只能打开/保存`.aquamidi`项目文件（使用"导出MIDI"功能导出`.mid`文件）
- **高级编辑功能**: 人性化（Humanize）、套索选择（Lasso selection）等高级功能仍在计划中
- **多曲线通道**: 目前主要支持力度曲线，音高曲线功能已实现但UI集成可能需要进一步完善

## 🛠️ Development

### Project Structure
```
egui_midi_editor/
├── Cargo.toml              # Workspace configuration
├── egui_midi/              # Core library (for integration)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs          # Public API
│       ├── structure.rs    # MIDI data structures and file I/O
│       ├── audio.rs        # Audio engine (optional preview)
│       └── ui/
│           └── mod.rs      # UI components
└── example_app/            # Demo application
    ├── Cargo.toml
    └── src/
        └── main.rs         # Example integration
```

### Key Dependencies
- **egui**: Immediate mode GUI framework for rendering
- **midly**: MIDI file parsing and generation
- **rodio**: Audio playback and synthesis (for preview only)
- **crossbeam-channel**: Thread-safe message passing

### For Developers Integrating This Library

The library is designed with developer experience in mind:

1. **Minimal Dependencies**: Only essential dependencies to keep your project lean
2. **Flexible Audio**: Optional audio backend - use your DAW's audio system
3. **Clear API**: Well-structured API for common MIDI operations
4. **Single Responsibility**: Focused on single-track editing only
5. **Performance First**: Optimized data structures and rendering

### Contributing

We welcome contributions that improve:
- **Developer Experience**: Better APIs, clearer documentation, more examples
- **Editing Tools**: Advanced editing capabilities
- **Performance**: Optimizations for large MIDI files
- **Code Quality**: Cleaner code, better error handling

## 📋 Roadmap

### 已实现功能 ✅
- [x] **单轨钢琴卷帘编辑器**
  - 音符创建、选择、拖拽、调整大小
  - 多选和框选
  - 吸附到网格（Snap to grid）
  
- [x] **检查器和剪贴板**
  - 音符属性编辑（音高、力度、时间、持续时间）
  - 复制/剪切/粘贴
  - 删除操作
  
- [x] **撤销/重做系统**
  - 完整的操作历史记录
  - 键盘快捷键支持
  
- [x] **曲线编辑功能**
  - 力度曲线（Velocity Curve）编辑
  - 音高曲线（Pitch Curve）编辑
  - 曲线点添加、编辑、删除
  - 线性插值计算
  - 可调整的分割器界面
  
- [x] **音频播放引擎**
  - 实时音频预览
  - ADSR合成
  - 音量和音高偏移控制
  - 可插拔音频后端接口
  
- [x] **传输控制**
  - 播放/暂停/停止
  - BPM控制
  - 时间轴定位
  - 循环播放支持
  
- [x] **文件I/O**
  - 严格单轨验证（`from_smf_strict`）
  - `.aquamidi` 项目格式
  - 标准`.mid`文件导出
  
- [x] **开发者API**
  - 事件/命令总线系统
  - 播放观察者接口
  - 可自定义选项

### 计划中功能 🚧
- [ ] 循环播放UI改进和传输反馈优化
- [ ] 高级编辑工具：人性化（Humanize）、批量变换
- [ ] 密集编排的性能优化
- [ ] 更好的API设计 + 全面的文档和示例
- [ ] 示例应用支持直接导入`.mid`文件

### 未来考虑 💡
- [ ] 和弦/音阶感知编辑辅助
- [ ] 可自定义UI主题
- [ ] 插件式扩展点
- [ ] 超出SMF单轨的导出选项

**Note**: This project focuses on single-track editing. Multi-track editing, VST support, MIDI device I/O, and sample-based synthesis are **not** planned features, as they are better handled by the host DAW application.

## 🤝 Contributing

Contributions are welcome! Please feel free to submit issues, feature requests, or pull requests.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Built with the amazing [egui](https://github.com/emilk/egui) framework
- Audio synthesis powered by [rodio](https://github.com/RustAudio/rodio)
- MIDI processing via [midly](https://github.com/kuviman/midly)

## 📊 Project Status

**Version**: v0.1.0 (Beta)  
**Status**: Active development  
**License**: MIT  
**Language**: Rust  

---

Made with ❤️ by CuteZaiyuan2333
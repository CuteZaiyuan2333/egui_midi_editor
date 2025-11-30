# egui_track

一个用于 DAW（数字音频工作站）风格的音轨编辑器组件库，基于 Rust 和 egui 框架。

## 功能特性

- **多轨管理**：支持多个音轨的创建、删除、重排序
- **剪辑片段编辑**：支持 MIDI 和音频剪辑的创建、移动、调整大小、分割
- **时间轴操作**：时间轴缩放、滚动、播放头控制
- **交互操作**：拖拽、选择、多选、网格对齐

## 设计原则

- **独立性**：不直接依赖 `egui_midi`，但可复用其绘制和交互模式
- **可扩展性**：通过 trait 和回调机制支持宿主自定义行为
- **模块化**：清晰的模块分离，便于维护和测试
- **性能优先**：仅渲染可见区域，优化大数据量场景

## 基本使用

```rust
use egui_track::{TrackEditor, TrackEditorOptions};

let mut editor = TrackEditor::new(TrackEditorOptions::default());

// 在 egui UI 中使用
editor.ui(ui);
```

## 集成到宿主应用

```rust
use egui_track::{TrackEditor, TrackEditorEvent, TrackEditorCommand, ClipType};

// 创建编辑器
let mut track_editor = TrackEditor::new(TrackEditorOptions::default());

// 设置事件监听器
track_editor.set_event_listener(Box::new(|event| {
    match event {
        TrackEditorEvent::ClipDoubleClicked { clip_id } => {
            // 打开 MIDI 编辑器
            open_midi_editor(clip_id);
        }
        TrackEditorEvent::ClipSelected { clip_id } => {
            // 处理剪辑选择
        }
        _ => {}
    }
}));

// 在 UI 中渲染
track_editor.ui(ui);

// 处理命令
track_editor.execute_command(TrackEditorCommand::CreateClip {
    track_id: some_track_id,
    start: 0.0,
    duration: 2.0,
    clip_type: ClipType::Midi { midi_data: None },
});
```

## 核心 API

### TrackEditor

主编辑器组件，管理所有音轨和剪辑。

```rust
pub struct TrackEditor {
    // ...
}

impl TrackEditor {
    pub fn new(options: TrackEditorOptions) -> Self;
    pub fn ui(&mut self, ui: &mut Ui);
    pub fn execute_command(&mut self, command: TrackEditorCommand);
    pub fn set_event_listener(&mut self, listener: Box<dyn FnMut(&TrackEditorEvent)>);
    pub fn take_events(&mut self) -> Vec<TrackEditorEvent>;
    pub fn tracks(&self) -> &[Track];
    pub fn timeline(&self) -> &TimelineState;
    pub fn selected_clips(&self) -> &BTreeSet<ClipId>;
}
```

### TrackEditorCommand

编辑命令，用于程序化地操作编辑器。

```rust
pub enum TrackEditorCommand {
    CreateClip { track_id: TrackId, start: f64, duration: f64, clip_type: ClipType },
    DeleteClip { clip_id: ClipId },
    MoveClip { clip_id: ClipId, new_track_id: TrackId, new_start: f64 },
    ResizeClip { clip_id: ClipId, new_duration: f64, resize_from_start: bool },
    SplitClip { clip_id: ClipId, split_time: f64 },
    CreateTrack { name: String },
    DeleteTrack { track_id: TrackId },
    RenameTrack { track_id: TrackId, new_name: String },
    SetPlayhead { position: f64 },
}
```

### TrackEditorEvent

编辑器事件，用于通知宿主应用用户操作。

```rust
pub enum TrackEditorEvent {
    ClipSelected { clip_id: ClipId },
    ClipDoubleClicked { clip_id: ClipId },
    ClipMoved { clip_id: ClipId, old_track_id: TrackId, new_track_id: TrackId, new_start: f64 },
    ClipResized { clip_id: ClipId, new_duration: f64 },
    PlayheadChanged { position: f64 },
    TrackCreated { track_id: TrackId },
    TrackDeleted { track_id: TrackId },
}
```

## 交互操作

### 鼠标操作

- **单击剪辑**：选择剪辑
- **Ctrl/Cmd + 单击**：切换选择
- **Shift + 单击**：添加到选择
- **拖拽剪辑**：移动剪辑位置
- **拖拽剪辑边缘**：调整剪辑大小
- **双击剪辑**：触发 `ClipDoubleClicked` 事件（用于打开编辑器）
- **框选**：在空白区域拖拽创建选择框
- **中键拖拽**：平移时间轴

### 键盘快捷键

- **Ctrl/Cmd + A**：全选所有剪辑
- **Delete / Backspace**：删除选中的剪辑
- **Ctrl + 鼠标滚轮**：缩放时间轴
- **鼠标滚轮（水平）**：水平滚动时间轴

## 与 egui_midi 的集成

虽然 `egui_track` 不直接依赖 `egui_midi`，但可以通过事件系统集成：

1. **事件驱动**：`egui_track` 通过 `ClipDoubleClicked` 事件通知宿主需要编辑 MIDI
2. **数据传递**：宿主负责在 `egui_track` 和 `egui_midi` 之间传递数据
3. **共享绘制代码**：可以将 `egui_midi` 的绘制函数提取为独立工具库（未来可考虑）

示例集成代码：

```rust
track_editor.set_event_listener(Box::new(|event| {
    match event {
        TrackEditorEvent::ClipDoubleClicked { clip_id } => {
            // 找到对应的剪辑
            if let Some(clip) = find_clip(clip_id) {
                match &clip.clip_type {
                    ClipType::Midi { midi_data } => {
                        // 打开 MIDI 编辑器
                        if let Some(path) = &midi_data.midi_file_path {
                            open_midi_editor_with_file(path);
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}));
```

## 示例程序

运行示例程序：

```bash
cargo run -p egui_track_example
```

示例程序展示了基本的音轨编辑器界面，包含几个示例轨道和剪辑片段。

## 依赖项

- `egui = "0.30"` - UI 框架

## 许可证

（根据项目许可证）

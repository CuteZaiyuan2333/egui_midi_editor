# 开发文档 - egui MIDI Editor

## 项目概述

这是一个基于Rust和egui框架的现代MIDI编辑器项目，提供跨平台的MIDI编辑和音频播放功能。

## 架构设计

### 核心模块

1. **structure.rs** - MIDI数据结构
   - `Note`: 音符数据结构，包含ID、开始时间、持续时间、音高和力度
   - `MidiState`: MIDI状态管理，包含所有音符、BPM、拍号等信息
   - 支持MIDI文件导入导出（使用midly库）

2. **audio.rs** - 音频引擎
   - 多音合成器，支持同时播放多个音符
   - ADSR包络控制（Attack, Decay, Sustain, Release）
   - 基于rodio库的音频输出

3. **ui/mod.rs** - 用户界面
   - 钢琴卷帘编辑器
   - 音符选择、拖拽、调整大小
   - 撤销/重做、复制/粘贴功能
   - 播放控制和时间轴

### 设计原则

- **模块化**: 清晰的模块分离，便于维护和扩展
- **零警告**: 代码以零警告为目标
- **跨平台**: 支持Windows、macOS、Linux
- **实时性**: 低延迟的音频播放和UI响应

### 架构职责边界

项目采用清晰的职责分离设计，确保示例项目和编辑器库各司其职：

#### 示例项目（example_app）职责

示例项目负责**应用层功能**，包括：

- **顶部菜单栏**：提供文件操作菜单（New, Open, Save, Save As, Import MIDI, Export MIDI）
- **底部状态栏**：显示应用状态信息（文件路径、操作结果、错误信息）
- **文件操作**：
  - 项目文件格式（`.aquamidi`）的读写
  - 标准MIDI文件（`.mid`）的导入导出
  - 文件对话框交互
- **状态管理**：维护当前文件路径、状态信息等应用级状态

**不应包含**：
- 直接操作编辑器内部状态（如 `insert_note()`）
- 编辑器UI组件的实现
- MIDI编辑逻辑

#### 编辑器库（egui_midi）职责

编辑器库负责**所有MIDI编辑功能**，包括：

- **完整UI渲染**：
  - 工具栏（播放控制、撤销/重做、BPM、节拍设置）
  - 钢琴卷帘（音符编辑、选择、拖拽）
  - 曲线编辑器（速度曲线、音高曲线）
  - 检查器面板（属性编辑、高级工具）
- **编辑功能**：
  - 音符创建、删除、修改
  - 选择、复制、粘贴、删除
  - 量化、人性化、批量变换
  - 撤销/重做
- **播放控制**：播放/暂停、停止、循环播放
- **快捷键处理**：Space键播放、Ctrl+C/X/V等编辑快捷键
- **事件系统**：`EditorEvent` 和 `EditorCommand` 接口

### 初始化最佳实践

#### ✅ 推荐方式

1. **使用默认空状态**：
```rust
let editor = MidiEditor::new(Some(audio));
// 编辑器以空状态启动，用户可以通过文件菜单加载数据
```

2. **通过文件加载初始化数据**：
```rust
// 在应用启动时，可以加载示例文件或用户上次打开的文件
if let Some(path) = get_last_opened_file() {
    let state = read_aquamidi_file(&path)?;
    editor.replace_state(state);
}
```

3. **使用 EditorCommand 接口**（适用于程序化初始化）：
```rust
// 如果需要程序化添加初始数据，使用命令接口
editor.apply_command(EditorCommand::AppendNotes(vec![
    Note::new(0, 480, 60, 100),
    Note::new(480, 480, 64, 100),
]));
editor.apply_command(EditorCommand::CenterOnKey(60));
```

#### ❌ 不推荐方式

避免在应用初始化时直接调用编辑器方法：
```rust
// ❌ 不推荐：直接操作编辑器状态
let mut editor = MidiEditor::new(Some(audio));
editor.insert_note(Note::new(0, 480, 60, 100)); // 这属于编辑器功能
editor.center_on_c4(); // 这属于编辑器功能
```

这种方式违反了职责边界，应该通过文件加载或命令接口来实现。

### 配置选项

编辑器库提供了丰富的配置选项，通过 `MidiEditorOptions` 进行设置：

#### 基本配置

```rust
use egui_midi::editor::MidiEditorOptions;

let options = MidiEditorOptions {
    zoom_x: 100.0,
    zoom_y: 20.0,
    snap_interval: 120,
    snap_mode: SnapMode::Absolute,
    volume: 0.5,
    center_on_key: Some(60), // 启动时居中到C4
    enable_space_playback: true, // 启用Space键播放控制
    ..Default::default()
};

let editor = MidiEditor::with_state_and_options(
    MidiState::default(),
    Some(audio),
    options
);
```

#### 快捷键配置

如果宿主应用需要处理 Space 键（例如用于全局播放控制），可以禁用编辑器内部的 Space 键处理：

```rust
let options = MidiEditorOptions {
    enable_space_playback: false, // 禁用Space键播放控制
    ..Default::default()
};

// 然后通过 EditorCommand 控制播放
editor.apply_command(EditorCommand::SetPlayback(true));
```

#### 视图配置

```rust
let options = MidiEditorOptions {
    zoom_x: 150.0, // 水平缩放
    zoom_y: 25.0,  // 垂直缩放
    manual_scroll_x: 0.0, // 初始水平滚动位置
    manual_scroll_y: 0.0, // 初始垂直滚动位置
    center_on_key: Some(60), // 启动时居中到指定音高
    ..Default::default()
};
```

### 接口使用指南

#### 状态管理接口

- `replace_state(state)`: 完全替换编辑器状态（用于加载文件）
- `snapshot_state()`: 获取当前状态的快照（用于保存文件）
- `midi_state()`: 获取状态的只读引用

#### 事件监听

```rust
editor.set_event_listener(|event| {
    match event {
        EditorEvent::StateReplaced(_) => {
            // 状态已替换，可能需要保存
        }
        EditorEvent::PlaybackStateChanged { is_playing } => {
            // 播放状态改变
        }
        _ => {}
    }
});

// 在UI循环中处理事件
for event in editor.take_events() {
    // 处理事件
}
```

#### 命令接口

```rust
// 控制播放
editor.apply_command(EditorCommand::SetPlayback(true));
editor.apply_command(EditorCommand::SeekSeconds(10.0));

// 修改设置
editor.apply_command(EditorCommand::SetBpm(120.0));
editor.apply_command(EditorCommand::SetTimeSignature(4, 4));

// 编辑操作
editor.apply_command(EditorCommand::CenterOnKey(60));
```

## 开发指南

### 环境搭建

```bash
# 安装Rust
# 访问 https://rustup.rs/ 获取安装指令

# 克隆项目
git clone https://github.com/CuteZaiyuan2333/egui_midi_editor.git
cd egui_midi_editor

# 构建项目
cargo build --release

# 运行示例
cargo run --release -p example_app
```

### 代码规范

1. **命名规范**
   - 函数名使用snake_case
   - 类型名使用CamelCase
   - 常量使用SCREAMING_SNAKE_CASE

2. **错误处理**
   - 使用Result<T, E>进行错误传播
   - 避免使用unwrap()，使用expect()并提供有意义的错误信息

3. **文档注释**
   - 公共API必须有文档注释
   - 复杂算法需要详细注释

### 性能优化

1. **UI渲染**
   - 使用egui的缓存机制避免重复计算
   - 合理设置重绘频率

2. **音频处理**
   - 使用固定大小的音频缓冲区
   - 避免在音频线程中分配内存

3. **MIDI处理**
   - 使用高效的数据结构（BTreeSet用于音符选择）
   - 批量处理音符操作

## 扩展功能

### 计划中的功能

1. **多轨编辑**
   - 支持多个MIDI轨道
   - 轨道静音/独奏功能

2. **高级编辑**
   - 量化功能
   - 人性化处理
   - 和弦识别和编辑

3. **音频增强**
   - 采样器支持
   - VST插件支持
   - 音频效果器

4. **MIDI设备**
   - MIDI输入设备支持
   - MIDI输出设备支持

### 贡献指南

1. Fork项目
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建Pull Request

## 调试技巧

### 音频问题

```rust
// 启用音频调试日志
env_logger::init();
// 设置日志级别
std::env::set_var("RUST_LOG", "debug");
```

### 性能分析

```bash
# 使用cargo profiler
cargo install cargo-profiler
cargo profiler callgrind --release

# 使用perf (Linux)
perf record cargo run --release
perf report
```

### UI调试

```rust
// 在egui中显示调试信息
ui.label(format!("FPS: {:.1}", ctx.input(|i| i.fps)));
ui.label(format!("Memory: {} MB", allocated_memory / 1024 / 1024));
```

## 发布流程

1. 更新版本号
   - 修改所有Cargo.toml中的版本号
   - 更新README.md中的版本信息

2. 测试构建
   ```bash
   cargo test --release
   cargo build --release
   ```

3. 创建标签
   ```bash
   git tag -a v0.1.0 -m "Release version 0.1.0"
   git push origin v0.1.0
   ```

4. 发布到crates.io（如需要）
   ```bash
   cargo publish -p egui_midi
   ```

## 常见问题

### Q: 音频播放有延迟？
A: 检查音频缓冲区大小设置，尝试减缓冲区大小但保持稳定性。

### Q: MIDI文件导入失败？
A: 检查MIDI文件格式，目前支持标准MIDI文件格式0和1。

### Q: UI响应慢？
A: 检查是否有大量的音符需要渲染，考虑实现音符分页或LOD（细节层次）系统。

## 联系信息

- GitHub: https://github.com/CuteZaiyuan2333
- 项目仓库: https://github.com/CuteZaiyuan2333/egui_midi_editor

---

*本文档最后更新: 2025年*
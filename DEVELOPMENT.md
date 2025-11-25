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
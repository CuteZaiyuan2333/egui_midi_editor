# 嵌入指南

本指南面向希望在宿主应用（去轨道化蓝图 DAW、节点编辑器等）中集成 `egui_midi` 的开发者，概述组件化接入的主要步骤与 API。

## 1. 初始化编辑器

```rust
use egui_midi::{
    audio::{AudioEngine, PlaybackBackend, PlaybackObserver},
    editor::{EditorCommand, EditorEvent, MidiEditorOptions},
    ui::MidiEditor,
};
use std::sync::Arc;

let audio: Arc<dyn PlaybackBackend> = Arc::new(AudioEngine::new());
let mut editor = MidiEditor::with_state_and_options(
    MidiState::default(),
    Some(audio),
    MidiEditorOptions::default(),
);
```

可通过 `MidiEditorOptions` 设置初始缩放、吸附、循环范围以及预览音高偏移。

## 2. 装载 / 导出 MIDI 数据

```rust
let smf = midly::Smf::parse(bytes)?;
let state = MidiState::from_smf_strict(&smf)?; // 保证单轨、单通道
editor.replace_state(state);

let exported = editor.snapshot_state().to_single_track_smf()?;
```

`from_smf_strict` / `to_single_track_smf` 会在发现多轨或混合通道时返回 `MidiValidationError`，便于宿主在入口处实施约束。

## 3. 订阅事件

```rust
editor.set_event_listener(|event| match event {
    EditorEvent::StateReplaced(state) => save_state(state),
    EditorEvent::SelectionChanged(ids) => highlight(ids),
    EditorEvent::PlaybackStateChanged { is_playing } => sync_transport(is_playing),
    EditorEvent::TransportChanged { current_time, .. } => update_timeline(current_time),
    _ => {}
});
```

事件涵盖：

- `StateReplaced(MidiState)`
- `NoteAdded / NoteDeleted / NoteUpdated`
- `SelectionChanged(Vec<NoteId>)`
- `PlaybackStateChanged`
- `TransportChanged`

## 4. 发送指令

使用 `EditorCommand` 驱动组件行为，例如：

```rust
editor.apply_command(EditorCommand::SeekSeconds(3.5));
editor.apply_command(EditorCommand::SetPlayback(true));
editor.apply_command(EditorCommand::ReplaceState(new_state));
editor.apply_command(EditorCommand::SetSnap { interval: 120, mode: SnapMode::Relative });
```

常用指令：

- `ReplaceState / SetNotes / AppendNotes / ClearNotes`
- `SeekSeconds`
- `SetPlayback`
- `CenterOnKey`
- `SetBpm / SetTimeSignature`
- `SetVolume / SetLoop / SetSnap`
- `OverrideTransport`

## 5. 音频集成

### 自定义 Backend

实现 `PlaybackBackend` 即可将音频事件接入宿主音频系统；默认 `AudioEngine` 提供多复音正弦波合成，可直接复用。

```rust
impl PlaybackBackend for MyBackend {
    fn note_on(&self, key: u8, velocity: u8) { ... }
    fn note_off(&self, key: u8) { ... }
    fn all_notes_off(&self) { ... }
    fn set_volume(&self, volume: f32) { mixer.set_gain(volume); }
    fn set_pitch_shift(&self, semitones: f32) { sampler.set_detune(semitones); }
}
```

### 播放回调

可选地实现 `PlaybackObserver` 以接收“开始/停止预听”通知：

```rust
struct Hook;
impl PlaybackObserver for Hook {
    fn on_playback_started(&self) { log::info!("preview start"); }
    fn on_playback_stopped(&self) { log::info!("preview stop"); }
}

editor.set_playback_observer(Some(Arc::new(Hook)));
```

## 6. UI 要点

- Inspector 面板支持单/多选属性查看与数值编辑
- 复制 / 剪切 / 粘贴 / 删除 支持键盘快捷键（Ctrl/Cmd + C/X/V/Delete）
- “量化到吸附”按钮与 `snap_interval` 配合
- 预览音高偏移滑条可实时调节合成器音调

## 7. 建议的宿主流程

1. 初始化 `MidiEditor`，配置 `PlaybackBackend` 与 `PlaybackObserver`
2. 通过 `from_smf_strict` 装载宿主的单轨 MIDI
3. 在 egui 布局中调用 `editor.ui(ui)`（可与其他节点同列展示）
4. 在 UI 刷新后调用 `editor.take_events()` 或 `set_event_listener` 收集变更
5. 根据事件更新宿主数据结构，并使用 `apply_command` 推送来自 DAW 的指令
6. 关闭组件时，调用 `editor.snapshot_state()` 或 `export_smf` 持久化

> 需要更完整的示例，可运行 `example_app` 参考默认实现。




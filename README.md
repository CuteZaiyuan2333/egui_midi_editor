# egui MIDI Editor

A modern, lightweight single-track MIDI editor library built with Rust and egui framework. Designed to be seamlessly integrated into DAW (Digital Audio Workstation) software, providing a focused and performant MIDI editing experience.

## 🎯 Project Goals

This library focuses on **single-track MIDI editing** and is designed to serve as a component within larger DAW applications. The primary objectives are:

- **Developer Experience**: Provide an elegant and intuitive API for easy integration
- **Practical Workflow**: Ship the common tools (selection, quantize-to-grid, clipboard, undo/redo) needed to embed a usable piano roll
- **Performance**: Optimized for handling large MIDI files efficiently
- **Simplicity**: Focused on single-track editing without unnecessary complexity

## 🎵 Features

- **Visual MIDI Editor**: Intuitive piano roll interface for editing *one track at a time*
- **Inspector & Clipboard**: Multi-select, copy/cut/paste, per-note editing, and quantize-to-snap grid actions
- **Undo / Redo Stack**: History snapshots for every editing action, plus keyboard shortcuts
- **Real-time Audio Playback**: Built-in audio engine with ADSR synthesis, pitch-shift preview, and pluggable backends
- **Cross-platform**: Works on Windows, macOS, and Linux
- **Modular Architecture**: Clean separation between UI, audio, and MIDI processing
- **Transport Controls**: Play/Pause/Stop with BPM control (loop regions planned)
- **Strict Single-Track I/O**: Helpers for validating/serializing single-track SMF payloads using midly
- **Developer-Friendly API**: Event/command bus, strict validation helpers, playback observers, and customizable options

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

## ⚠️ Current Limitations

- Strictly single-track & single-channel: validation rejects multi-track or mixed-channel SMFs
- Loop configuration fields exist but playback looping UI/logic is still experimental
- Example app can only open/save `.aquamidi` projects (use Export MIDI for `.mid`)
- Advanced editing features such as humanize, velocity curves, lasso selection, automation, etc. are planned but not implemented

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

### Delivered
- [x] Single-track piano roll with inspector, clipboard, quantize, undo/redo
- [x] Strict SMF validation helpers + `.aquamidi` project format + `.mid` export
- [x] Real-time preview synth with volume/pitch controls
- [x] Event/command bridge for embedding in host applications

### Next Up
- [ ] Loop playback UX plus improved transport feedback
- [ ] Advanced editing tools: humanize, velocity editing, batch transforms
- [ ] Performance optimizations for dense arrangements
- [ ] Better API ergonomics + comprehensive documentation/examples
- [ ] Demo support for importing `.mid` files directly

### Future Considerations
- [ ] Chord/scale aware editing helpers
- [ ] Customizable UI themes
- [ ] Plugin-style extension points for bespoke tooling
- [ ] Broader export options beyond SMF single-track

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
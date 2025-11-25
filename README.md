# egui MIDI Editor

A modern, lightweight single-track MIDI editor library built with Rust and egui framework. Designed to be seamlessly integrated into DAW (Digital Audio Workstation) software, providing a focused and performant MIDI editing experience.

## 🎯 Project Goals

This library focuses on **single-track MIDI editing** and is designed to serve as a component within larger DAW applications. The primary objectives are:

- **Developer Experience**: Provide an elegant and intuitive API for easy integration
- **Advanced Editing Tools**: Rich set of editing capabilities (quantize, humanize, etc.)
- **Performance**: Optimized for handling large MIDI files efficiently
- **Simplicity**: Focused on single-track editing without unnecessary complexity

## 🎵 Features

- **Visual MIDI Editor**: Intuitive piano roll interface for editing single-track MIDI notes
- **Real-time Audio Playback**: Built-in audio engine with ADSR envelope synthesis for preview
- **Cross-platform**: Works on Windows, macOS, and Linux
- **Modular Architecture**: Clean separation between UI, audio, and MIDI processing
- **Professional Editing**: Note editing, selection, undo/redo, copy/paste
- **Transport Controls**: Play, pause, loop functionality with BPM control
- **MIDI Import/Export**: Load and save MIDI files using the midly library
- **Developer-Friendly API**: Designed for easy integration into existing DAW projects

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
```

## 🎹 Usage

### Basic Integration

The library is designed to be easily integrated into your DAW application:

```rust
use egui_midi::MidiEditor;
use egui_midi::audio::{AudioEngine, PlaybackBackend};
use egui_midi::structure::Note;
use std::sync::Arc;

// Create audio engine (optional - can be None if you handle audio externally)
let audio: Arc<dyn PlaybackBackend> = Arc::new(AudioEngine::new());

// Initialize MIDI editor
let mut editor = MidiEditor::new(Some(audio));

// Add notes to the single track
editor.insert_note(Note::new(0, 480, 60, 100)); // C4 quarter note

// Render the editor in your egui UI
editor.ui(ui);
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
}
```

### Integration Best Practices

- **Single Track Focus**: This library handles one MIDI track at a time. For multi-track DAWs, create multiple `MidiEditor` instances
- **Audio Backend**: Use `None` if your DAW already handles audio, or implement `PlaybackBackend` to integrate with your audio system
- **State Management**: The editor maintains its own state, making it easy to embed in larger applications

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

### High Priority (Developer Experience & Core Features)
- [x] Basic single-track MIDI editing
- [x] MIDI file import/export
- [ ] **Advanced editing tools**: Quantize, humanize, velocity editing
- [ ] **Performance optimizations**: Efficient rendering for large MIDI files
- [ ] **Better API ergonomics**: More intuitive methods for common operations
- [ ] **Comprehensive documentation**: API docs with integration examples

### Medium Priority (Enhanced Editing)
- [ ] Chord detection and editing
- [ ] Scale constraints and snap-to-scale
- [ ] Note velocity curves and automation
- [ ] Advanced selection tools (lasso, time range, etc.)
- [ ] Batch operations on selected notes

### Future Considerations
- [ ] Customizable UI themes
- [ ] Plugin architecture for custom editing tools
- [ ] Export to various MIDI formats

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
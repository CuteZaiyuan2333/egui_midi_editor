# egui MIDI Editor

A modern, cross-platform MIDI editor built with Rust and egui framework. This project provides a comprehensive MIDI editing experience with real-time audio playback capabilities.

## 🎵 Features

- **Visual MIDI Editor**: Intuitive piano roll interface for editing MIDI notes
- **Real-time Audio Playback**: Built-in audio engine with ADSR envelope synthesis
- **Cross-platform**: Works on Windows, macOS, and Linux
- **Modular Architecture**: Clean separation between UI, audio, and MIDI processing
- **Professional Features**: Note editing, selection, undo/redo, copy/paste
- **Transport Controls**: Play, pause, loop functionality with BPM control
- **MIDI Import/Export**: Load and save MIDI files using the midly library

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
```rust
use egui_midi::MidiEditor;
use egui_midi::audio::{AudioEngine, PlaybackBackend};
use std::sync::Arc;

// Create audio engine
let audio: Arc<dyn PlaybackBackend> = Arc::new(AudioEngine::new());

// Initialize MIDI editor
let mut editor = MidiEditor::new(Some(audio));

// Add notes
editor.insert_note(Note::new(0, 480, 60, 100)); // C4 quarter note
```

### Custom Audio Backend
Implement the `PlaybackBackend` trait for custom audio engines:
```rust
pub trait PlaybackBackend {
    fn note_on(&self, key: u8, velocity: u8);
    fn note_off(&self, key: u8);
    fn all_notes_off(&self);
    fn set_volume(&self, volume: f32);
}
```

## 🛠️ Development

### Project Structure
```
egui_midi_editor/
├── Cargo.toml              # Workspace configuration
├── egui_midi/              # Core library
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── structure.rs     # MIDI data structures
│       ├── audio.rs         # Audio engine
│       └── ui/
│           └── mod.rs       # UI components
└── example_app/            # Demo application
    ├── Cargo.toml
    └── src/
        └── main.rs
```

### Key Dependencies
- **egui**: Immediate mode GUI framework
- **midly**: MIDI file parsing and generation
- **rodio**: Audio playback and synthesis
- **crossbeam-channel**: Thread-safe message passing

## 📋 Roadmap

- [ ] Multi-track MIDI editing
- [ ] Advanced audio synthesis (samples, VST support)
- [ ] MIDI device input/output
- [ ] Advanced editing tools (quantize, humanize)
- [ ] Plugin architecture for extensions
- [ ] Performance optimizations for large MIDI files

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
# egui MIDI Editor

A project centered around MIDI editor with multiple utility libraries and examples. Built with Rust and egui framework, providing a focused and performant MIDI editing experience along with useful tools for DAW (Digital Audio Workstation) software development.

## 🎯 Project Goals

This library focuses on **single-track MIDI editing** and is designed to serve as a component within larger DAW applications. The primary objectives are:

- **Developer Experience**: Provide an elegant and intuitive API for easy integration
- **Practical Workflow**: Ship the common tools (selection, quantize-to-grid, clipboard, undo/redo) needed to embed a usable piano roll
- **Performance**: Optimized for handling large MIDI files efficiently
- **Simplicity**: Focused on single-track editing without unnecessary complexity

## 🎵 Features

### Core Editing Features
- **Visual MIDI Editor**: Intuitive piano roll interface for editing *one track at a time*
  - Note creation, selection, drag and move
  - Note length adjustment (drag right edge)
  - Multi-select support (Ctrl/Cmd + click, Shift + click to extend selection)
  - Box selection (drag selection area)
  - Snap to grid (configurable snap interval and mode)
  - Enhanced visual feedback for selected notes (4x thicker white stroke)
  
- **Inspector & Clipboard**: 
  - Property editing for multi-selected notes (pitch, velocity, start time, duration)
  - Copy/Cut/Paste (Ctrl/Cmd + C/X/V)
  - Delete selected notes (Delete/Backspace)
  - Quantize to snap grid
  
- **Advanced Editing Tools**:
  - **Humanize**: Add random timing and velocity variations to selected notes for a more natural feel
    - Accessible via Inspector panel or right-click context menu
    - Configurable time and velocity randomization ranges
  - **Batch Transform**: Apply transformations to multiple selected notes simultaneously
    - Velocity offset: Adjust velocity by a fixed amount
    - Duration scale: Scale note durations by a factor
    - Pitch offset: Transpose notes by semitones
    - Interactive dialog for precise control
  - **Swing Rhythm**: Apply swing timing to selected notes by directly modifying their positions
    - Accessible via right-click context menu
    - Real-time adjustment with slider (0-100%) and custom input (0-200%)
    - Applies timing offset to even-numbered beats (2, 4, 6...)
    - Immediately modifies note positions (not just during playback)
  
- **Undo / Redo Stack**: 
  - Complete undo/redo system that records all editing operations
  - Keyboard shortcuts: Ctrl/Cmd + Z (undo), Ctrl/Cmd + Shift + Z or Ctrl/Cmd + Y (redo)

### Curve Editing Features
- **Velocity Curve**:
  - Visual velocity curve editing
  - Add, edit, and delete curve points
  - Linear interpolation for velocity values
  - Value range: 0-127
  - Automatically applies curve to note velocities when exporting MIDI
  
- **Pitch Curve**:
  - Pitch offset curve editing (supports semitone offsets)
  - Value range: -12 to +12 semitones
  - Same editing capabilities as velocity curve
  
- **Curve Editor Interface**:
  - Adjustable splitter to control height ratio between piano roll and curve editor
  - Enable/disable toggle for curve lanes
  - Real-time curve effect preview

### Audio Playback Features
- **Real-time Audio Playback**: 
  - Built-in audio engine with real-time preview
  - ADSR envelope synthesis (Attack, Decay, Sustain, Release)
  - Pitch shift preview
  - Volume control
  - Pluggable audio backend interface (`PlaybackBackend`) for integration with DAW audio systems

### Transport Controls
- **Transport Controls**: 
  - Play/Pause (Space key or programmatic control)
  - BPM control (configurable and real-time adjustment)
  - Timeline positioning (Seek)
  - Loop playback support (Loop regions with configurable start and end positions)
    - Interactive loop region editing: Shift + Left-drag on timeline to adjust loop boundaries
    - Visual loop markers on timeline (L/R indicators)
    - Loop status and position display in toolbar
  - Timeline interactions:
    - Left-drag on timeline: Adjust playhead position with grid snapping (Alt to disable snap)
    - Shift + Left-drag on timeline: Edit loop region (adjust start/end or move entire region) with grid snapping
  - Time signature settings
  - **Playback Settings Dialog**: Centralized settings panel for playback configuration
    - Volume control (0-200%)
    - Pitch shift adjustment (±12 semitones)
    - Loop region configuration
    - Snap interval and mode settings
    - Accessible via UI button
  - **Enhanced Transport Feedback**: Transport events include loop state and position information

### File I/O
- **Strict Single-Track I/O**: 
  - MIDI file import/export (using midly library)
  - Single-track validation (`from_smf_strict` ensures single track and single channel)
  - `.aquamidi` project format support (example app)
  - Standard `.mid` file export
  - **MIDI Import Support**: The example app supports direct import of standard `.mid` files
    - Files are validated to ensure single-track and single-channel compliance
    - Import via "Import MIDI..." menu option

### Developer API
- **Developer-Friendly API**: 
  - Event/command bus system (`EditorEvent` / `EditorCommand`)
  - Strict validation helper functions
  - Playback observer interface (`PlaybackObserver`)
  - Customizable options (`MidiEditorOptions`)
  - Event listener (`set_event_listener`)

### Other Features
- **Cross-platform**: Works on Windows, macOS, and Linux
- **Modular Architecture**: Clean separation between UI, audio, and MIDI processing
- **Keyboard Shortcuts**: 
  - `Space`: Play/Pause
  - `Ctrl/Cmd + C`: Copy
  - `Ctrl/Cmd + X`: Cut
  - `Ctrl/Cmd + V`: Paste
  - `Ctrl/Cmd + Z`: Undo
  - `Ctrl/Cmd + Shift + Z` or `Ctrl/Cmd + Y`: Redo
  - `Delete` / `Backspace`: Delete selected notes

## 🏗️ Architecture

The project is organized as a Rust workspace with multiple components:

### `egui_midi` (Library)
Core MIDI editor library containing:
- **structure.rs**: MIDI data structures and file I/O operations
- **audio.rs**: Audio engine with polyphonic synthesis and ADSR envelopes
- **ui/mod.rs**: Complete egui-based MIDI editor interface

### `egui_track` (Library)
Multi-track timeline editor library for DAW-style clip arrangement:
- **structure.rs**: Track, clip, and timeline data structures
- **editor.rs**: Command/event system for editor operations
- **ui/mod.rs**: Complete egui-based track editor interface with:
  - Multi-track management (create, delete, rename tracks)
  - Clip editing (create, move, resize, rename clips)
  - Track panel with mute, solo, record arm, monitor controls
  - Volume and pan sliders per track
  - Snap-to-grid with configurable intervals
  - Playhead positioning and playback control
  - Project file I/O support
- **project.rs**: Project file format for saving/loading track arrangements

### `example_app` (Demo Application)
A demonstration application showcasing the MIDI editor library's capabilities with a functional MIDI editor interface.

### `egui_track_example` (Demo Application)
A demonstration application showcasing the track editor library with a functional multi-track timeline interface. This example application serves as the foundation for future development, where we plan to build a simple DAW software that integrates both the MIDI editor (`egui_midi`) and track editor (`egui_track`) plugins into a unified digital audio workstation.

### `midi_track_file_example` (Integrated Demo Application)
A comprehensive demonstration application that integrates all three libraries (`egui_midi`, `egui_track`, and `egui_file_tree`) into a unified DAW-style interface:

**Layout Structure**:
- **Top Section**: Tabbed interface with:
  - Track Editor tab: Full multi-track timeline editor
  - Other Tools tab: Placeholder for additional tools
- **Bottom Section**: Split into two panels:
  - **Left Panel**: File tree browser for navigating directory structures
  - **Right Panel**: Multi-tab MIDI editor interface supporting multiple open MIDI files

**Key Features**:
- ✅ Resizable splitter controls for adjusting panel proportions:
  - Vertical splitter between top and bottom sections
  - Horizontal splitter between file tree and MIDI editors
- ✅ File tree integration with directory browsing
- ✅ MIDI file opening: Double-click `.mid` or `.midiclip` files in the file tree to open them in new MIDI editor tabs
- ✅ Multiple MIDI editor instances: Each tab maintains its own editor state
- ✅ Tab management: Add, switch, and close MIDI editor tabs
- ✅ Project file management: Save/load track editor projects (`.tracks` format)
- ✅ Menu integration: File operations and directory selection
- ✅ **MIDI Clip Workflow**:
  - Create and manage `.midiclip` files (single-track MIDI format)
  - Convert standard `.mid` files to `.midiclip` format via right-click context menu
  - Drag and drop `.midiclip` files from file tree to track editor to create clips
  - Double-click clips in track editor to open them in MIDI editor for editing
  - Automatic preview update: When a MIDI clip is edited and saved, all clips using the same file are automatically updated
  - Visual MIDI note preview within clips on the timeline
- ✅ **Multi-Track Playback**:
  - Real-time multi-track audio playback with sine wave synthesizer
  - Dynamic track engine allocation (supports unlimited tracks)
  - Per-track volume and pan control
  - Mute and Solo functionality
  - Independent zoom and scroll for MIDI editor and track editor
- ✅ **Clip Management**:
  - Right-click context menu for clips (Copy, Cut, Paste, Delete)
  - Clip renaming with automatic file system synchronization
  - Clip preview rendering showing individual MIDI notes

**Usage**:
```bash
cargo run --release -p midi_track_file_example
```

This application demonstrates how to integrate all three libraries into a cohesive DAW interface, providing a foundation for building complete digital audio workstation software.

### `egui_file_tree` (Library)
A file system tree component library for displaying directory structures in a tree view:
- **tree.rs**: File tree component implementation with:
  - Tree view display of file and directory structures
  - Expand/collapse folders (with ▶/▼ icons)
  - File and folder type distinction (📁/📄 icons)
  - Selection support for files and folders
  - Double-click events (handled by the application)
  - Right-click context menu events (handled by the application)
  - Drag and drop support for files (e.g., `.midiclip` files)
  - Independent drag state tracking per file item
  - Parent directory navigation ("../" option)
  - Automatic sorting (folders first, then by name)
  - Error handling for inaccessible directories
  - Indented display for hierarchical relationships

### `file_tree_example` (Demo Application)
A demonstration application showcasing the file tree component with a simple file browser interface:
- Top menu bar with "File" menu
- "Open Directory" option to select a directory
- Central panel displaying the file tree
- Bottom status bar showing current status and event information
- Event handling for selection and double-click operations

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

# Run the MIDI editor example application
cargo run --release -p example_app

# Run the track editor example application
cargo run --release -p egui_track_example

# Run the file tree example application
cargo run --release -p file_tree_example

# Run the integrated MIDI/Track/File example application
cargo run --release -p midi_track_file_example

# Note: The MIDI editor demo opens/saves `.aquamidi` single-track projects and can export standard `.mid` files.
# The track editor demo supports project file I/O for multi-track arrangements.
# The file tree demo allows browsing directory structures with a tree view interface.
# The integrated demo combines all three libraries into a unified DAW-style interface.
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

The bundled `example_app` includes a File menu (New/Open/Save/Save As/Import MIDI/Export MIDI) that operates on a custom single-track project format with the `.aquamidi` extension. `.aquamidi` files wrap a validated single-track SMF payload plus a lightweight header, ensuring demos stay aligned with the library's "one track per editor" constraint. The example app supports both importing standard `.mid` files (with single-track validation) and exporting to standard `.mid` format that any DAW can open.

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

## 📝 Implemented Features Detailed List

### Note Editing
- ✅ Click empty area to create new note
- ✅ Click note to select
- ✅ Drag note to move position
- ✅ Drag note right edge to adjust length
- ✅ Ctrl/Cmd + Click: Toggle selection
- ✅ Shift + Click: Extend selection
- ✅ Drag to box-select multiple notes
- ✅ Snap to grid
- ✅ Snap modes: Absolute and Relative
- ✅ Enhanced visual feedback: Selected notes have 4x thicker white stroke

### Clipboard Operations
- ✅ Copy selected notes (Ctrl/Cmd + C)
- ✅ Cut selected notes (Ctrl/Cmd + X)
- ✅ Paste notes (Ctrl/Cmd + V)
- ✅ Delete selected notes (Delete/Backspace)

### Undo/Redo
- ✅ Complete operation history
- ✅ Undo (Ctrl/Cmd + Z)
- ✅ Redo (Ctrl/Cmd + Shift + Z or Ctrl/Cmd + Y)

### Inspector Panel
- ✅ Display selected note properties
- ✅ Edit pitch (Key)
- ✅ Edit velocity
- ✅ Edit start time
- ✅ Edit duration
- ✅ Batch edit for multi-selection

### Curve Editing
- ✅ Velocity Curve
  - Add curve points (click on curve area)
  - Drag curve points to adjust position and value
  - Delete curve points (right-click or Delete key)
  - Linear interpolation calculation
  - Automatically applied to notes on export
- ✅ Pitch Curve
  - Same editing capabilities as velocity curve
  - Supports -12 to +12 semitone offsets
- ✅ Curve Lane Management
  - Enable/disable curve lanes
  - Adjustable splitter for interface layout

### Audio Playback
- ✅ Real-time audio preview
- ✅ ADSR envelope synthesis
- ✅ Volume control
- ✅ Pitch shift preview
- ✅ Pluggable audio backend interface

### Transport Controls
- ✅ Play/Pause (Space key)
- ✅ BPM setting and adjustment
- ✅ Time signature settings
- ✅ Timeline positioning (Seek)
- ✅ Loop playback configuration
  - Interactive loop region editing: Shift + Left-drag on timeline
  - Visual loop markers on timeline (L/R indicators)
  - Loop status and position display
- ✅ Timeline interactions
  - Left-drag: Adjust playhead with grid snapping (Alt to disable)
  - Shift + Left-drag: Edit loop region with grid snapping
- ✅ Enhanced transport feedback with loop state information

### File Operations
- ✅ Import MIDI files (single-track validation)
- ✅ Export MIDI files
- ✅ `.aquamidi` project format (example app)
- ✅ Standard `.mid` file export

### View Controls
- ✅ Horizontal/vertical zoom
- ✅ Scroll view
- ✅ Center on specified pitch
- ✅ Adjustable curve editor height

### Advanced Editing Tools
- ✅ Humanize
  - Add random timing variations to selected notes
  - Add random velocity variations to selected notes
  - Accessible via Inspector panel and right-click context menu
  - Configurable time and velocity ranges
- ✅ Batch Transform
  - Velocity offset: Adjust velocity by fixed amount (-127 to +127)
  - Duration scale: Scale note durations by factor (0.1x to 10.0x)
  - Pitch offset: Transpose notes by semitones (-127 to +127)
  - Interactive dialog with real-time preview
  - Accessible via Inspector panel and right-click context menu
- ✅ Swing Rhythm
  - Apply swing timing to selected notes by directly modifying positions
  - Accessible via right-click context menu
  - Real-time adjustment: Slider (0-100%) and custom input (0-200%)
  - Applies timing offset to even-numbered beats (2, 4, 6...)
  - Immediately modifies note positions (not just during playback)

### Playback Settings
- ✅ Playback Settings Dialog
  - Volume control (0-200%)
  - Pitch shift adjustment (±12 semitones)
  - Loop region configuration (start/end ticks)
  - Snap interval selection (1/1, 1/2, 1/4, 1/8, 1/16, Free)
  - Snap mode selection (Absolute/Relative)
  - Accessible via UI button

## ⚠️ Current Limitations

- **Strict Single-Track Constraint**: Validation rejects multi-track or mixed-channel SMF files
- **Multiple Curve Lanes**: Currently primarily supports velocity curves; pitch curve functionality is implemented but UI integration may need further refinement

## 🛠️ Development

### Project Structure
```
egui_midi_editor/
├── Cargo.toml                  # Workspace configuration
├── egui_midi/                  # MIDI editor library
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs             # Public API
│       ├── structure.rs       # MIDI data structures and file I/O
│       ├── audio.rs           # Audio engine (optional preview)
│       └── ui/
│           └── mod.rs          # UI components
├── egui_track/                 # Track editor library
│   ├── Cargo.toml
│   ├── README.md               # Track editor documentation
│   └── src/
│       ├── lib.rs             # Public API
│       ├── structure.rs       # Track, clip, timeline structures
│       ├── editor.rs          # Command/event system
│       ├── project.rs         # Project file I/O
│       └── ui/
│           ├── mod.rs          # Main UI components
│           ├── toolbar.rs      # Toolbar UI
│           ├── statusbar.rs   # Status bar UI
│           └── clip.rs         # Clip rendering
├── example_app/                # MIDI editor demo application
│   ├── Cargo.toml
│   └── src/
│       └── main.rs            # MIDI editor example
└── egui_track_example/        # Track editor demo application
    ├── Cargo.toml
    └── src/
        └── main.rs             # Track editor example
├── egui_file_tree/            # File tree component library
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs             # Public API
│       └── tree.rs            # File tree component implementation
└── file_tree_example/         # File tree demo application
    ├── Cargo.toml
    └── src/
        └── main.rs            # File tree example
└── midi_track_file_example/   # Integrated demo application
    ├── Cargo.toml
    └── src/
        └── main.rs            # Integrated MIDI/Track/File example
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

## 🌳 File Tree Component (`egui_file_tree`)

The `egui_file_tree` library provides a file system tree component for displaying directory structures in a tree view format, commonly used in file browsers and project explorers.

### File Tree Features

#### Tree View Display
- ✅ Tree structure visualization with hierarchical indentation
- ✅ Expand/collapse folders with visual indicators (▶/▼)
- ✅ File and folder type distinction with icons (📁/📄)
- ✅ Automatic sorting (folders first, then alphabetical by name)
- ✅ Parent directory navigation ("../" option at root level)

#### Interaction
- ✅ Click to select files or folders
- ✅ Double-click events (handled by the application)
- ✅ Right-click context menu support (application handles menu display)
- ✅ Drag and drop support for files (e.g., `.midiclip` files to track editor)
- ✅ Visual selection feedback
- ✅ Hover cursor indication
- ✅ Independent drag state tracking per file item

#### Error Handling
- ✅ Graceful handling of inaccessible directories
- ✅ Error messages displayed in the UI
- ✅ Continues operation even when some directories cannot be read

### File Tree Usage

```rust
use egui_file_tree::{FileTree, FileTreeEvent};
use std::path::PathBuf;

// Create file tree
let mut file_tree = FileTree::new(PathBuf::from("/path/to/directory"));

// Render in UI
let events = file_tree.ui(ui);

// Handle events
for event in events {
    match event {
        FileTreeEvent::PathSelected { path } => {
            println!("Selected: {:?}", path);
        }
        FileTreeEvent::PathDoubleClicked { path } => {
            println!("Double clicked: {:?}", path);
            // Handle file opening here
        }
        FileTreeEvent::PathRightClicked { path, pos } => {
            println!("Right clicked: {:?} at {:?}", path, pos);
            // Show context menu at position
        }
        FileTreeEvent::PathDragStarted { path } => {
            println!("Drag started: {:?}", path);
            // Handle drag operation (e.g., drag to track editor)
        }
        FileTreeEvent::NavigateToParent => {
            // Navigate to parent directory
            if let Some(parent) = file_tree.root_path().parent() {
                file_tree.set_root_path(parent.to_path_buf());
            }
        }
    }
}
```

### File Tree API

```rust
impl FileTree {
    /// Create a new file tree with the specified root directory
    pub fn new(root_path: PathBuf) -> Self;
    
    /// Set the root directory path
    pub fn set_root_path(&mut self, path: PathBuf);
    
    /// Expand a directory path
    pub fn expand_path(&mut self, path: &PathBuf);
    
    /// Collapse a directory path
    pub fn collapse_path(&mut self, path: &PathBuf);
    
    /// Get the current root directory path
    pub fn root_path(&self) -> &PathBuf;
    
    /// Render the UI and return events
    pub fn ui(&mut self, ui: &mut Ui) -> Vec<FileTreeEvent>;
}
```

### File Tree Events

```rust
pub enum FileTreeEvent {
    /// A path was selected (single click)
    PathSelected { path: PathBuf },
    
    /// A path was double-clicked
    PathDoubleClicked { path: PathBuf },
    
    /// A path was right-clicked
    PathRightClicked { path: PathBuf, pos: Pos2 },
    
    /// A path drag operation started
    PathDragStarted { path: PathBuf },
    
    /// Navigate to parent directory (clicked "../")
    NavigateToParent,
}
```

### File Tree Example Application

The `file_tree_example` application demonstrates the file tree component with a complete file browser interface. It includes:

- Top menu bar with "File" menu containing "Open Directory" option
- Central panel displaying the file tree with all features
- Bottom status bar showing current status and event information
- Full event handling for selection, double-click, and parent navigation

**Design Philosophy**: The library only handles display and navigation. File opening/closing operations are handled by the application through event callbacks, keeping the component focused and reusable.

## 🎼 Track Editor (`egui_track`)

The `egui_track` library provides a multi-track timeline editor for arranging MIDI and audio clips in a DAW-style interface.

### Track Editor Features

#### Multi-Track Management
- ✅ Create, delete, and rename tracks
- ✅ Track panel with interactive controls:
  - Mute, Solo, Record Arm, Monitor buttons
  - Volume slider (with dB display)
  - Pan slider (with L/C/R indicators)
  - Collapsible Inserts and Sends sections
- ✅ Right-click context menu for track operations

#### Clip Editing
- ✅ Create, move, resize, and delete clips
- ✅ Clip renaming via double-click on title bar
- ✅ Multi-select support (Ctrl/Cmd + click, Shift + click)
- ✅ Box selection for multiple clips
- ✅ Snap-to-grid with configurable intervals (1/16, 1/8, 1/4, 1 Beat)
- ✅ Alt key to temporarily disable snapping
- ✅ Clip types: MIDI clips and Audio clips
- ✅ Visual clip preview with title bars

#### Timeline & Transport
- ✅ Playhead positioning and playback control
- ✅ BPM and time signature settings
- ✅ Horizontal and vertical zoom (Ctrl/Alt + mouse wheel)
- ✅ Middle mouse button drag for panning
- ✅ Scroll limits with proper boundaries
- ✅ Visual grid system aligned with MIDI editor

#### Project Management
- ✅ Project file format for saving/loading arrangements
- ✅ Track and clip state persistence
- ✅ Timeline state (BPM, time signature, zoom, scroll) persistence

#### User Interface
- ✅ Toolbar with transport controls and snap settings
- ✅ Status bar with project information
- ✅ File menu (New, Open, Save, Save As, Export)
- ✅ All UI text in English

### Track Editor Usage

```rust
use egui_track::{TrackEditor, TrackEditorOptions, TrackEditorCommand, ClipType};

// Create track editor
let mut editor = TrackEditor::new(TrackEditorOptions::default());

// Create a track
editor.execute_command(TrackEditorCommand::CreateTrack {
    name: "Track 1".to_string(),
});

// Create a MIDI clip
editor.execute_command(TrackEditorCommand::CreateClip {
    track_id: some_track_id,
    start: 0.0,
    duration: 4.0,
    clip_type: ClipType::Midi { midi_data: None },
});

// Render in UI
editor.ui(ui);
```

### Track Editor Example Application

The `egui_track_example` application demonstrates the track editor library with a complete multi-track timeline interface. It includes:

- Full track editor UI with toolbar and status bar
- Project file management (New, Open, Save, Save As)
- Track and clip creation/editing
- All track panel controls and interactions

**Future Plans**: The track editor example application will serve as the foundation for building a simple DAW software that integrates both the MIDI editor (`egui_midi`) and track editor (`egui_track`) plugins. This unified DAW will allow users to:

- Arrange multiple tracks with MIDI and audio clips
- Double-click MIDI clips to open them in the MIDI editor for detailed note editing
- Seamlessly switch between timeline arrangement and MIDI note editing
- Export complete multi-track projects

**Current Implementation**: The `midi_track_file_example` application demonstrates an early implementation of this unified DAW concept, integrating:
- Track editor for multi-track arrangement
- File tree browser for project navigation
- Multiple MIDI editor instances for detailed note editing
- Resizable interface panels for flexible workflow

## 📋 Roadmap

### Implemented Features ✅
- [x] **Single-Track Piano Roll Editor**
  - Note creation, selection, drag, and resize
  - Multi-select and box selection
  - Snap to grid
  
- [x] **Inspector & Clipboard**
  - Note property editing (pitch, velocity, time, duration)
  - Copy/Cut/Paste
  - Delete operations
  
- [x] **Undo/Redo System**
  - Complete operation history
  - Keyboard shortcut support
  
- [x] **Curve Editing Features**
  - Velocity curve editing
  - Pitch curve editing
  - Curve point add, edit, delete
  - Linear interpolation calculation
  - Adjustable splitter interface
  
- [x] **Audio Playback Engine**
  - Real-time audio preview
  - ADSR synthesis
  - Volume and pitch shift control
  - Pluggable audio backend interface
  
- [x] **Transport Controls**
  - Play/Pause/Stop
  - BPM control
  - Timeline positioning with grid snapping
  - Loop playback support with interactive editing (Shift + Left-drag)
  - Enhanced transport feedback
  
- [x] **File I/O**
  - Strict single-track validation (`from_smf_strict`)
  - `.aquamidi` project format
  - Standard `.mid` file export
  - MIDI file import support (example app)
  
- [x] **Advanced Editing Tools**
  - Humanize: Random timing and velocity variations
  - Batch Transform: Velocity offset, duration scale, pitch offset
  - Swing Rhythm: Direct note position modification (0-200% range, accessible via right-click menu)
  
- [x] **Playback Settings**
  - Centralized playback settings dialog
  - Volume, pitch, loop, and snap configuration
  
- [x] **Developer API**
  - Event/command bus system
  - Playback observer interface
  - Customizable options

## 🎉 Recent Updates

### Latest Improvements
- **Integrated DAW Example**: Enhanced `midi_track_file_example` application with complete MIDI clip workflow
  - `.midiclip` file format support for single-track MIDI clips
  - Drag and drop from file tree to track editor
  - Automatic preview synchronization when clips are edited
  - Visual MIDI note preview within clips
  - Multi-track playback with dynamic track engine allocation
  - Independent zoom/scroll for MIDI editor and track editor
  - Clip context menu (Copy, Cut, Paste, Delete)
  - Project file format changed to `.tracks` extension
- **File Tree Enhancements**:
  - Fixed drag-and-drop detection logic to correctly identify dragged files
  - Right-click context menu support:
    - Convert `.mid` files to `.midiclip` format
    - Create new MIDI clips in folders
    - Edit and delete `.midiclip` files
- **Playback Engine Fixes**:
  - Fixed first note playback issue (notes at position 0 now play correctly)
  - Improved clip scheduling logic to handle overlapping time windows
  - Enhanced event processing for accurate multi-track playback
- **Swing Rhythm Enhancement**: Moved to right-click context menu with real-time adjustment (0-200% range, supports custom input)
- **Interactive Loop Editing**: Shift + Left-drag on timeline to edit loop boundaries with grid snapping
- **Timeline Interactions**: Left-drag for playhead positioning, Shift + Left-drag for loop editing (both support grid snapping, Alt to disable)
- **Visual Feedback**: Selected notes now have 4x thicker white stroke for better visibility
- **Enhanced Transport Events**: Transport events now include detailed loop state and position information

### Planned Features 🚧
- [ ] Performance optimizations for dense arrangements
- [ ] Better API design + comprehensive documentation and examples

### Future Considerations 💡
- [ ] Chord/scale-aware editing helpers
- [ ] Customizable UI themes
- [ ] Plugin-style extension points
- [ ] Export options beyond SMF single-track

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
//! Clip interaction module
//!
//! Defines hit regions for clip interaction (click, resize, etc.)

/// Hit region of a clip for interaction detection
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipHitRegion {
    /// Body of the clip (for moving)
    Body,
    /// Left edge of the clip (for resizing from start)
    LeftEdge,
    /// Right edge of the clip (for resizing from end)
    RightEdge,
}

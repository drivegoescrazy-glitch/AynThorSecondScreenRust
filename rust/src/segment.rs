//! One coloured run of chat, the unit the app draws.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment {
    pub text: String,
    /// Packed RGB, or `None` for the app's default colour.
    pub color: Option<u32>,
}

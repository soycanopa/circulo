//! Composer input events.

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ComposerInputEvent {
    Submit(String),
    Edited,
    Focus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ComposerEvent {
    Submit(String),
    ProjectPicked(circulo_core::Uuid),
    ProjectCleared,
}

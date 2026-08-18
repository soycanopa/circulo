//! Composer input events.

use circulo_core::{ComposerInteractionMode, ComposerPermissionMode, Uuid};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ComposerInputEvent {
    Submit(String),
    Edited,
    Focus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ComposerEvent {
    Submit(String),
    ProjectPicked(Uuid),
    ProjectCleared,
    OpenProject,
    WorkModeChanged(WorkMode),
    ModelChanged(String),
    ModelVariantChanged(String),
    PermissionModeChanged(ComposerPermissionMode),
    InteractionModeChanged(ComposerInteractionMode),
}

pub type PermissionMode = ComposerPermissionMode;
pub type InteractionMode = ComposerInteractionMode;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub enum WorkMode {
    #[default]
    Local,
    Remote,
}

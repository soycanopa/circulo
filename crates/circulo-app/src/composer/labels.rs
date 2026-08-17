use circulo_core::{ComposerInteractionMode, ComposerPermissionMode};

pub fn permission_label_key(mode: ComposerPermissionMode) -> &'static str {
    match mode {
        ComposerPermissionMode::FullAccess => "composer.permission.full_access",
        ComposerPermissionMode::Auto => "composer.permission.auto",
        ComposerPermissionMode::Supervised => "composer.permission.supervised",
        ComposerPermissionMode::AutoAcceptEdits => "composer.permission.auto_accept_edits",
    }
}

pub fn permission_description_key(mode: ComposerPermissionMode) -> &'static str {
    match mode {
        ComposerPermissionMode::FullAccess => "composer.permission.full_access_desc",
        ComposerPermissionMode::Auto => "composer.permission.auto_desc",
        ComposerPermissionMode::Supervised => "composer.permission.supervised_desc",
        ComposerPermissionMode::AutoAcceptEdits => "composer.permission.auto_accept_edits_desc",
    }
}

pub fn permission_icon(mode: ComposerPermissionMode) -> &'static str {
    match mode {
        ComposerPermissionMode::FullAccess
        | ComposerPermissionMode::Auto
        | ComposerPermissionMode::Supervised
        | ComposerPermissionMode::AutoAcceptEdits => "icons/shield.svg",
    }
}

pub fn interaction_label_key(mode: ComposerInteractionMode) -> &'static str {
    match mode {
        ComposerInteractionMode::Plan => "composer.mode.plan",
        ComposerInteractionMode::Build => "composer.mode.build",
        ComposerInteractionMode::Ask => "composer.mode.ask",
    }
}

pub fn interaction_icon(mode: ComposerInteractionMode) -> &'static str {
    match mode {
        ComposerInteractionMode::Plan => "icons/list.svg",
        ComposerInteractionMode::Build => "icons/wrench.svg",
        ComposerInteractionMode::Ask => "icons/message-circle.svg",
    }
}

pub fn interaction_accent(mode: ComposerInteractionMode) -> bool {
    matches!(mode, ComposerInteractionMode::Plan)
}

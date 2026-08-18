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

pub fn reasoning_label_key(variant: &str) -> String {
    format!("composer.reasoning.{}", variant.to_ascii_lowercase())
}

pub fn reasoning_display_label(catalog: &circulo_i18n::Catalog, variant: &str) -> String {
    let key = reasoning_label_key(variant);
    let localized = catalog.get(&key);
    if localized == key {
        capitalize_variant(variant)
    } else {
        localized.to_string()
    }
}

fn capitalize_variant(variant: &str) -> String {
    let mut chars = variant.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

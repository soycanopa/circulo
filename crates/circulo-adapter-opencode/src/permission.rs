use circulo_core::ComposerPermissionMode;

pub fn ruleset_for(mode: ComposerPermissionMode) -> Vec<serde_json::Value> {
    match mode {
        ComposerPermissionMode::FullAccess | ComposerPermissionMode::Auto => vec![
            permission_rule("read", "*", "allow"),
            permission_rule("edit", "*", "allow"),
            permission_rule("bash", "*", "allow"),
        ],
        ComposerPermissionMode::Supervised => vec![
            permission_rule("read", "*", "allow"),
            permission_rule("edit", "*", "ask"),
            permission_rule("bash", "*", "ask"),
        ],
        ComposerPermissionMode::AutoAcceptEdits => vec![
            permission_rule("read", "*", "allow"),
            permission_rule("edit", "*", "allow"),
            permission_rule("bash", "*", "ask"),
        ],
    }
}

fn permission_rule(permission: &str, pattern: &str, action: &str) -> serde_json::Value {
    serde_json::json!({
        "permission": permission,
        "pattern": pattern,
        "action": action,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supervised_asks_before_edits_and_commands() {
        let rules = ruleset_for(ComposerPermissionMode::Supervised);
        assert_eq!(rules.len(), 3);
        assert_eq!(rules[1]["action"], "ask");
        assert_eq!(rules[2]["action"], "ask");
    }
}

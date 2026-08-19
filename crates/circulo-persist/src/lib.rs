//! Local SQLite persistence for Circulo domain entities.

mod error;
mod schema;
mod store;

pub use error::PersistError;
pub use store::{default_db_path, Store};

#[cfg(test)]
mod tests {
    use circulo_core::{
        AgentType, DomainError, Message, MessagePart, MessageRole, MessageStatus, Project,
        ProjectStatus, Session, SessionStatus, Uuid,
    };
    use time::OffsetDateTime;

    use super::{default_db_path, PersistError, Store};

    fn now() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_700_000_000).expect("valid ts")
    }

    fn project(name: &str, id: u128) -> Project {
        Project {
            id: Uuid::from_u128(id),
            name: name.into(),
            description: None,
            color: None,
            folder_path: None,
            status: ProjectStatus::Active,
            created_at: now(),
            updated_at: now(),
        }
    }

    fn session(id: u128, project_id: Option<u128>, title: &str) -> Session {
        Session {
            id: Uuid::from_u128(id),
            project_id: project_id.map(Uuid::from_u128),
            title: title.into(),
            agent: AgentType::OpenCode,
            status: SessionStatus::Active,
            created_at: now(),
            updated_at: now(),
            last_message_at: None,
            first_send_at: None,
            composer_model_id: None,
            composer_model_variant: None,
            composer_permission_mode: None,
            composer_interaction_mode: None,
        }
    }

    fn user_message(id: u128, session_id: u128, text: &str) -> Message {
        Message {
            id: Uuid::from_u128(id),
            session_id: Uuid::from_u128(session_id),
            role: MessageRole::User,
            parts: vec![MessagePart::Text {
                content: text.into(),
            }],
            status: MessageStatus::Complete,
            created_at: now(),
            is_streaming: false,
        }
    }

    #[test]
    fn default_path_is_application_support_circulo() {
        let path = default_db_path().expect("HOME");
        assert!(path.ends_with("Library/Application Support/Circulo/circulo.sqlite"));
    }

    #[test]
    fn unassigned_session_persists() {
        let store = Store::open_in_memory().unwrap();
        let s = session(1, None, "New session");
        store.create_session(&s).unwrap();
        let loaded = store.get_session(s.id).unwrap().unwrap();
        assert!(loaded.project_id.is_none());
        assert_eq!(store.list_unassigned_sessions().unwrap().len(), 1);
    }

    #[test]
    fn delete_project_cascades_sessions_and_messages() {
        let store = Store::open_in_memory().unwrap();
        let p = project("Launch", 10);
        store.create_project(&p).unwrap();
        store.create_session(&session(11, Some(10), "A")).unwrap();
        store.create_session(&session(12, Some(10), "B")).unwrap();
        store.create_session(&session(13, None, "Loose")).unwrap();
        store.save_message(&user_message(21, 11, "hi")).unwrap();
        store.delete_project(p.id).unwrap();
        assert!(store.get_session(Uuid::from_u128(11)).unwrap().is_none());
        assert!(store.list_messages(Uuid::from_u128(11)).unwrap().is_empty());
        assert_eq!(
            store
                .get_session(Uuid::from_u128(13))
                .unwrap()
                .unwrap()
                .title,
            "Loose"
        );
    }

    #[test]
    fn delete_session_cascades_messages() {
        let store = Store::open_in_memory().unwrap();
        let s = session(50, None, "Gone");
        store.create_session(&s).unwrap();
        store.save_message(&user_message(51, 50, "bye")).unwrap();
        store.delete_session(s.id).unwrap();
        assert!(store.get_session(s.id).unwrap().is_none());
        assert!(store.list_messages(s.id).unwrap().is_empty());
    }

    #[test]
    fn archive_hides_and_restore_shows() {
        let store = Store::open_in_memory().unwrap();
        let p = project("Launch", 30);
        store.create_project(&p).unwrap();
        store
            .create_session(&session(31, Some(30), "In project"))
            .unwrap();
        store.archive_project(p.id).unwrap();
        assert!(store.list_active_projects().unwrap().is_empty());
        assert_eq!(store.list_archived_projects().unwrap().len(), 1);
        assert!(store.list_visible_sessions().unwrap().is_empty());
        store.restore_project(p.id).unwrap();
        assert_eq!(store.list_active_projects().unwrap().len(), 1);
        assert_eq!(store.list_visible_sessions().unwrap().len(), 1);
    }

    #[test]
    fn project_folder_path_roundtrips() {
        let store = Store::open_in_memory().unwrap();
        let mut p = project("Docs", 50);
        p.folder_path = Some("/Users/me/Projects/Docs".into());
        store.create_project(&p).unwrap();
        let loaded = store.get_project(p.id).unwrap().expect("project");
        assert_eq!(loaded.folder_path.as_deref(), Some("/Users/me/Projects/Docs"));
    }

    #[test]
    fn assignment_locks_after_first_user_message() {
        let store = Store::open_in_memory().unwrap();
        let p = project("Launch", 40);
        store.create_project(&p).unwrap();
        let s = session(41, None, "Chat");
        store.create_session(&s).unwrap();
        store.assign_session_project(s.id, Some(p.id)).unwrap();
        store.assign_session_project(s.id, None).unwrap();
        store.save_message(&user_message(42, 41, "hello")).unwrap();
        let err = store.assign_session_project(s.id, Some(p.id)).unwrap_err();
        match err {
            PersistError::Domain(DomainError::ProjectAssignmentLocked) => {}
            other => panic!("unexpected error: {other}"),
        }
        assert!(store
            .get_session(s.id)
            .unwrap()
            .unwrap()
            .project_id
            .is_none());
    }

    #[test]
    fn search_matches_visible_titles_only() {
        let store = Store::open_in_memory().unwrap();
        let p = project("Hidden", 50);
        store.create_project(&p).unwrap();
        store
            .create_session(&session(51, None, "Landing copy"))
            .unwrap();
        store
            .create_session(&session(52, None, "Budget notes"))
            .unwrap();
        store
            .create_session(&session(53, Some(50), "Landing archived"))
            .unwrap();
        store.archive_project(p.id).unwrap();
        let hits = store.search_sessions("land").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "Landing copy");
    }

    #[test]
    fn agent_binding_roundtrip_and_default_null() {
        let store = Store::open_in_memory().unwrap();
        let s = session(60, None, "Chat");
        store.create_session(&s).unwrap();
        assert_eq!(store.opencode_session_id(s.id).unwrap(), None);
        store
            .bind_opencode_session(s.id, "ses_from_opencode")
            .unwrap();
        assert_eq!(
            store.opencode_session_id(s.id).unwrap().as_deref(),
            Some("ses_from_opencode")
        );
    }

    #[test]
    fn agent_binding_is_write_once() {
        let store = Store::open_in_memory().unwrap();
        let s = session(61, None, "Chat");
        store.create_session(&s).unwrap();
        store.bind_opencode_session(s.id, "ses_first").unwrap();
        store.bind_opencode_session(s.id, "ses_first").unwrap();
        let err = store.bind_opencode_session(s.id, "ses_other").unwrap_err();
        assert!(matches!(err, PersistError::AgentBindingLocked));
        assert_eq!(
            store.opencode_session_id(s.id).unwrap().as_deref(),
            Some("ses_first")
        );
    }

    #[test]
    fn non_opencode_agent_roundtrips() {
        let store = Store::open_in_memory().unwrap();
        let mut s = session(70, None, "CmdCode session");
        s.agent = AgentType::CommandCode;
        store.create_session(&s).unwrap();
        let loaded = store.get_session(s.id).unwrap().unwrap();
        assert_eq!(loaded.agent, AgentType::CommandCode);
        // Existing OpenCode sessions still load correctly.
        let s2 = session(71, None, "OpenCode session");
        store.create_session(&s2).unwrap();
        let loaded2 = store.get_session(s2.id).unwrap().unwrap();
        assert_eq!(loaded2.agent, AgentType::OpenCode);
    }
}

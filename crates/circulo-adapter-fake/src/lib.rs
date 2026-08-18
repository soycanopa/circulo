//! Fake `AgentAdapter` for tests and UI development without OpenCode.

use circulo_adapter::{
    AdapterError, AdapterEvent, AdapterHealth, AgentAdapter, ErrorReason, GenerateRequest,
    ModelCatalogEntry, Task, TaskStatus, ToolCall, ToolCallStatus, ToolOutput,
};

#[derive(Debug, Clone)]
pub struct FakeAdapter {
    fail: bool,
}

impl FakeAdapter {
    pub fn new() -> Self {
        Self { fail: false }
    }

    pub fn failing() -> Self {
        Self { fail: true }
    }
}

impl Default for FakeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentAdapter for FakeAdapter {
    fn name(&self) -> &'static str {
        "fake"
    }

    fn probe(&self) -> AdapterHealth {
        AdapterHealth::Available
    }

    fn list_models(&self) -> Result<Vec<ModelCatalogEntry>, AdapterError> {
        Ok(vec![
            ModelCatalogEntry {
                id: "fake/default".into(),
                name: "Fake Default".into(),
                provider_id: "fake".into(),
                provider_name: "Fake".into(),
                model_id: "default".into(),
                context_window: Some("128K".into()),
                reasoning_variants: vec!["low".into(), "medium".into(), "high".into()],
            },
        ])
    }

    fn generate(
        &self,
        _request: GenerateRequest,
        emit: &mut dyn FnMut(AdapterEvent),
    ) -> Result<(), AdapterError> {
        if self.fail {
            let error =
                AdapterError::failed(ErrorReason::Internal, "The fake agent was asked to fail.");
            emit(AdapterEvent::Failed {
                error: error.clone(),
            });
            return Err(error);
        }

        emit(AdapterEvent::TextDelta {
            content: "Working on it.".into(),
        });
        emit(AdapterEvent::TaskList {
            tasks: vec![
                Task {
                    id: "task_1".into(),
                    title: "Draft the outline".into(),
                    description: None,
                    status: TaskStatus::Completed,
                    order: 0,
                },
                Task {
                    id: "task_2".into(),
                    title: "Write the first section".into(),
                    description: None,
                    status: TaskStatus::InProgress,
                    order: 1,
                },
            ],
        });

        let mut tool = ToolCall {
            id: "tc_fake_1".into(),
            name: "edit_file".into(),
            status: ToolCallStatus::Running,
            input: serde_json::json!({"path": "notes.md"}),
            output: None,
            started_at: None,
            finished_at: None,
        };
        emit(AdapterEvent::ToolCallStarted {
            tool_call: tool.clone(),
        });
        tool.status = ToolCallStatus::Success;
        tool.output = Some(ToolOutput::Diff {
            file_path: "notes.md".into(),
            old_content: None,
            new_content: "Hello from Circulo.\n".into(),
            diff: Some("--- a/notes.md\n+++ b/notes.md\n+Hello from Circulo.\n".into()),
        });
        emit(AdapterEvent::ToolCallUpdated { tool_call: tool });
        emit(AdapterEvent::TextDelta {
            content: "Done.".into(),
        });
        emit(AdapterEvent::Completed);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use circulo_adapter::{
        AdapterEvent, AdapterHealth, AgentAdapter, GenerateRequest, ToolCallStatus, ToolOutput,
        Uuid,
    };

    use super::FakeAdapter;

    fn request() -> GenerateRequest {
        GenerateRequest {
            session_id: Uuid::nil(),
            user_text: "Write a note".into(),
            agent_session_id: None,
            composer_model_id: None,
            composer_model_variant: None,
            composer_permission_mode: None,
            composer_interaction_mode: None,
        }
    }

    fn collect(
        adapter: &FakeAdapter,
    ) -> (Result<(), circulo_adapter::AdapterError>, Vec<AdapterEvent>) {
        let mut events = Vec::new();
        let result = adapter.generate(request(), &mut |event| events.push(event));
        (result, events)
    }

    #[test]
    fn probe_is_available() {
        assert_eq!(FakeAdapter::new().probe(), AdapterHealth::Available);
    }

    #[test]
    fn successful_turn_emits_expected_sequence() {
        let (result, events) = collect(&FakeAdapter::new());
        assert!(result.is_ok());
        assert!(events
            .iter()
            .any(|e| matches!(e, AdapterEvent::TextDelta { .. })));
        assert!(events
            .iter()
            .any(|e| matches!(e, AdapterEvent::TaskList { .. })));
        assert!(events.iter().any(|e| {
            matches!(
                e,
                AdapterEvent::ToolCallUpdated {
                    tool_call
                } if tool_call.status == ToolCallStatus::Success
                    && matches!(tool_call.output, Some(ToolOutput::Diff { .. }))
            )
        }));
        assert!(events.iter().any(|e| matches!(e, AdapterEvent::Completed)));
        assert!(!events
            .iter()
            .any(|e| matches!(e, AdapterEvent::Failed { .. })));
    }

    #[test]
    fn failing_turn_emits_failed_event() {
        let (result, events) = collect(&FakeAdapter::failing());
        assert!(result.is_err());
        match events.last() {
            Some(AdapterEvent::Failed { error }) => {
                assert!(!error.message().is_empty());
            }
            other => panic!("expected failed event, got {other:?}"),
        }
    }
}

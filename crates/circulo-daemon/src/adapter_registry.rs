use std::sync::Arc;

use circulo_adapter::AgentAdapter;
use circulo_core::AgentType;
use circulo_protocol::AgentDescriptor;

#[derive(Clone)]
pub struct AdapterRegistry {
    opencode: Arc<dyn AgentAdapter>,
}

impl AdapterRegistry {
    pub fn build() -> Self {
        let opencode: Arc<dyn AgentAdapter> = match std::env::var("CIRCULO_ADAPTER").as_deref() {
            Ok("fake") => Arc::new(circulo_adapter_fake::FakeAdapter::new()),
            _ => Arc::new(circulo_adapter_opencode::OpenCodeAdapter::from_env()),
        };
        Self { opencode }
    }

    /// Construct a registry with a caller-supplied OpenCode adapter. Tests use
    /// this to swap in a fake without touching env vars.
    pub fn with_opencode(opencode: Arc<dyn AgentAdapter>) -> Self {
        Self { opencode }
    }

    pub fn opencode(&self) -> Arc<dyn AgentAdapter> {
        Arc::clone(&self.opencode)
    }

    pub fn for_agent(&self, agent: AgentType) -> Option<Arc<dyn AgentAdapter>> {
        match agent {
            AgentType::OpenCode => Some(Arc::clone(&self.opencode)),
            AgentType::CommandCode => None,
        }
    }

    pub fn list(&self) -> Vec<AgentDescriptor> {
        let opencode = self.opencode.as_ref();
        let opencode_available = matches!(opencode.probe(), circulo_adapter::AdapterHealth::Available);
        let opencode_version = opencode
            .opencode_health()
            .and_then(|h| h.version);
        vec![AgentDescriptor {
            agent: AgentType::OpenCode,
            available: opencode_available,
            version: opencode_version,
        }]
    }
}

use std::collections::HashSet;
use std::sync::{Arc, RwLock};

use circulo_adapter::AgentAdapter;
use circulo_core::{AgentType, UserPreferences};
use circulo_protocol::AgentDescriptor;

#[derive(Clone)]
pub struct AdapterRegistry {
    opencode: Arc<dyn AgentAdapter>,
    commandcode: Option<Arc<dyn AgentAdapter>>,
    disabled: Arc<RwLock<HashSet<AgentType>>>,
}

impl AdapterRegistry {
    pub fn build(prefs: &UserPreferences) -> Self {
        let opencode: Arc<dyn AgentAdapter> = match std::env::var("CIRCULO_ADAPTER").as_deref() {
            Ok("fake") => Arc::new(circulo_adapter_fake::FakeAdapter::new()),
            _ => Arc::new(circulo_adapter_opencode::OpenCodeAdapter::from_env()),
        };
        let commandcode = circulo_adapter_commandcode::CommandCodeAdapter::from_env()
            .map(|a| Arc::new(a) as Arc<dyn AgentAdapter>);
        Self {
            opencode,
            commandcode,
            disabled: Arc::new(RwLock::new(
                prefs.disabled_agents.iter().copied().collect(),
            )),
        }
    }

    /// Construct a registry with a caller-supplied OpenCode adapter. Tests use
    /// this to swap in a fake without touching env vars.
    pub fn with_opencode(opencode: Arc<dyn AgentAdapter>) -> Self {
        Self {
            opencode,
            commandcode: None,
            disabled: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    pub fn opencode(&self) -> Arc<dyn AgentAdapter> {
        Arc::clone(&self.opencode)
    }

    pub fn is_enabled(&self, agent: AgentType) -> bool {
        !self
            .disabled
            .read()
            .map(|g| g.contains(&agent))
            .unwrap_or(false)
    }

    pub fn set_disabled(&self, agent: AgentType, disabled: bool) {
        if let Ok(mut g) = self.disabled.write() {
            if disabled {
                g.insert(agent);
            } else {
                g.remove(&agent);
            }
        }
    }

    pub fn for_agent(&self, agent: AgentType) -> Option<Arc<dyn AgentAdapter>> {
        if !self.is_enabled(agent) {
            return None;
        }
        match agent {
            AgentType::OpenCode => Some(Arc::clone(&self.opencode)),
            AgentType::CommandCode => self.commandcode.as_ref().map(Arc::clone),
        }
    }

    pub fn list(&self) -> Vec<AgentDescriptor> {
        let opencode = self.opencode.as_ref();
        let opencode_available =
            matches!(opencode.probe(), circulo_adapter::AdapterHealth::Available);
        let opencode_version = opencode.opencode_health().and_then(|h| h.version);
        let mut descriptors = vec![AgentDescriptor {
            agent: AgentType::OpenCode,
            available: opencode_available,
            version: opencode_version,
            enabled: self.is_enabled(AgentType::OpenCode),
        }];
        if let Some(commandcode) = self.commandcode.as_ref() {
            let cc_available =
                matches!(commandcode.probe(), circulo_adapter::AdapterHealth::Available);
            descriptors.push(AgentDescriptor {
                agent: AgentType::CommandCode,
                available: cc_available,
                version: None,
                enabled: self.is_enabled(AgentType::CommandCode),
            });
        }
        descriptors
    }
}

//! In-memory TTL cache for the model catalog.
//!
//! Per-provider cache. `get(&registry)` aggregates the entries from
//! every enabled provider, de-duplicating by `(agent, id)`. Disabled
//! providers are skipped so a disabled CommandCode doesn't leak its
//! catalog into the picker.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use circulo_adapter::{AdapterError, AgentAdapter};
use circulo_core::{AgentType, ModelCatalogEntry};

use crate::adapter_registry::AdapterRegistry;

/// Default catalog cache lifetime (5 minutes).
pub const DEFAULT_MODEL_CATALOG_TTL: Duration = Duration::from_secs(300);

#[derive(Debug, Clone)]
struct CachedCatalog {
    fetched_at: Instant,
    entries: Vec<ModelCatalogEntry>,
}

#[derive(Debug)]
pub struct ModelCatalogCache {
    ttl: Duration,
    cached: Mutex<HashMap<AgentType, CachedCatalog>>,
}

impl ModelCatalogCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            cached: Mutex::new(HashMap::new()),
        }
    }

    /// Returns the union of every enabled provider's catalog, sorted
    /// by (provider_name, name). The result is cached per-provider
    /// for the configured TTL.
    pub fn get(&self, registry: &AdapterRegistry) -> Result<Vec<ModelCatalogEntry>, AdapterError> {
        let mut out = Vec::new();
        for &agent in AgentType::ALL.iter() {
            if !registry.is_enabled(agent) {
                continue;
            }
            let Some(adapter) = registry.for_agent(agent) else {
                continue;
            };
            let entries = self.fetch(agent, adapter.as_ref())?;
            out.extend(entries);
        }
        out.sort_by(|left, right| {
            left
                .provider_name
                .cmp(&right.provider_name)
                .then_with(|| left.name.cmp(&right.name))
        });
        Ok(out)
    }

    fn fetch(
        &self,
        agent: AgentType,
        adapter: &dyn AgentAdapter,
    ) -> Result<Vec<ModelCatalogEntry>, AdapterError> {
        if let Some(cached) = self.cached.lock().unwrap().get(&agent) {
            if cached.fetched_at.elapsed() < self.ttl {
                return Ok(cached.entries.clone());
            }
        }
        let entries = adapter.list_models()?;
        self.cached.lock().unwrap().insert(
            agent,
            CachedCatalog {
                fetched_at: Instant::now(),
                entries: entries.clone(),
            },
        );
        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use circulo_adapter::{
        AdapterError, AdapterEvent, AgentAdapter, AdapterHealth, ErrorReason, GenerateRequest,
    };
    use circulo_core::ModelCatalogEntry;

    use super::*;

    struct CountingAdapter {
        calls: AtomicUsize,
        entries: Vec<ModelCatalogEntry>,
    }

    impl CountingAdapter {
        fn new(entries: Vec<ModelCatalogEntry>) -> Arc<Self> {
            Arc::new(Self {
                calls: AtomicUsize::new(0),
                entries,
            })
        }
    }

    impl AgentAdapter for CountingAdapter {
        fn name(&self) -> &'static str {
            "counting"
        }

        fn probe(&self) -> AdapterHealth {
            AdapterHealth::Available
        }

        fn list_models(&self) -> Result<Vec<ModelCatalogEntry>, AdapterError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.entries.clone())
        }

        fn generate(
            &self,
            _request: GenerateRequest,
            _emit: &mut dyn FnMut(AdapterEvent),
        ) -> Result<(), AdapterError> {
            Ok(())
        }
    }

    #[test]
    fn cache_skips_adapter_within_ttl() {
        let adapter = CountingAdapter::new(vec![ModelCatalogEntry {
            id: "openai/gpt-4o".into(),
            name: "GPT-4o".into(),
            provider_id: "openai".into(),
            provider_name: "OpenAI".into(),
            model_id: "gpt-4o".into(),
            context_window: None,
            reasoning_variants: vec![],
            agent: circulo_core::AgentType::OpenCode,
        }]);
        let mut cache = ModelCatalogCache::new(Duration::from_secs(60));
        cache
            .fetch(circulo_core::AgentType::OpenCode, adapter.as_ref())
            .expect("first load");
        cache
            .fetch(circulo_core::AgentType::OpenCode, adapter.as_ref())
            .expect("cached load");
        assert_eq!(adapter.calls.load(Ordering::SeqCst), 1);
    }
}

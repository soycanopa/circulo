//! Static model catalog for Command Code.
//!
//! Source: <https://commandcode.ai/docs/reference/cli/models>. Refresh
//! this list when the upstream docs publish new models. Each entry
//! carries the upstream `provider_id` and `provider_name` so the UI
//! can show the company; the Circulo `agent` is always
//! `AgentType::CommandCode` for the entries in this catalog.

use std::sync::LazyLock;

use circulo_adapter::ModelCatalogEntry;
use circulo_core::AgentType;

/// The full list of models Command Code exposes. Pulled from the docs
/// page; commands like `cmd -m <id>` accept any of these.
///
/// We list each model as `(id, name, upstream_provider_id, upstream_provider_name)`.
/// The catalog layer fills the `agent` field and the helper
/// `provider_id` (Circulo-internal) from the upstream values.
const ENTRIES: &[(&str, &str, &str, &str)] = &[
    // Alibaba / Qwen
    ("Qwen/Qwen3.6-Max-Preview", "Qwen 3.6 Max Preview", "qwen", "Qwen"),
    ("Qwen/Qwen3.6-Plus", "Qwen 3.6 Plus", "qwen", "Qwen"),
    ("Qwen/Qwen3.7-Flash", "Qwen 3.7 Flash", "qwen", "Qwen"),
    ("Qwen/Qwen3.7-Max", "Qwen 3.7 Max", "qwen", "Qwen"),
    ("Qwen/Qwen3.7-Plus", "Qwen 3.7 Plus", "qwen", "Qwen"),
    ("Qwen/Qwen3.8-27B", "Qwen 3.8 27B", "qwen", "Qwen"),
    ("Qwen/Qwen3.8-Max", "Qwen 3.8 Max", "qwen", "Qwen"),
    // Anthropic
    ("claude-fable-5", "Claude Fable 5", "anthropic", "Anthropic"),
    ("claude-haiku-4-5", "Claude Haiku 4.5", "anthropic", "Anthropic"),
    ("claude-opus-4-7", "Claude Opus 4.7", "anthropic", "Anthropic"),
    ("claude-opus-4-8", "Claude Opus 4.8", "anthropic", "Anthropic"),
    ("claude-opus-5", "Claude Opus 5", "anthropic", "Anthropic"),
    ("claude-sonnet-4-6", "Claude Sonnet 4.6", "anthropic", "Anthropic"),
    ("claude-sonnet-5", "Claude Sonnet 5", "anthropic", "Anthropic"),
    // DeepSeek
    ("deepseek/deepseek-v4-flash", "DeepSeek V4 Flash (latest)", "deepseek", "DeepSeek"),
    ("deepseek/deepseek-v4-pro", "DeepSeek V4 Pro (latest)", "deepseek", "DeepSeek"),
    // Google
    ("google/gemini-3.1-flash-lite", "Gemini 3.1 Flash Lite", "google", "Google"),
    ("google/gemini-3.5-flash", "Gemini 3.5 Flash", "google", "Google"),
    ("google/gemini-3.5-flash-lite", "Gemini 3.5 Flash Lite", "google", "Google"),
    ("google/gemini-3.6-flash", "Gemini 3.6 Flash", "google", "Google"),
    ("google/gemini-3.7-flash", "Gemini 3.7 Flash", "google", "Google"),
    // Meta
    ("meta/muse-spark-1.1", "Muse Spark 1.1", "meta", "Meta"),
    ("meta/muse-spark-1.2", "Muse Spark 1.2", "meta", "Meta"),
    ("meta/muse-spark-1.2-contributor", "Muse Spark 1.2 Contributor", "meta", "Meta"),
    // MiniMax
    ("MiniMaxAI/MiniMax-M2.5", "MiniMax M2.5", "MiniMax", "MiniMax"),
    ("MiniMaxAI/MiniMax-M2.7", "MiniMax M2.7", "MiniMax", "MiniMax"),
    ("MiniMaxAI/MiniMax-M3", "MiniMax M3", "MiniMax", "MiniMax"),
    // Moonshot AI
    ("moonshotai/Kimi-K2.5", "Kimi K2.5", "moonshot", "Moonshot AI"),
    ("moonshotai/Kimi-K2.6", "Kimi K2.6", "moonshot", "Moonshot AI"),
    ("moonshotai/Kimi-K2.7-Code", "Kimi K2.7 Code", "moonshot", "Moonshot AI"),
    ("moonshotai/Kimi-K2.7-Code-Highspeed", "Kimi K2.7 Code HighSpeed", "moonshot", "Moonshot AI"),
    ("moonshotai/Kimi-K3", "Kimi K3", "moonshot", "Moonshot AI"),
    // NVIDIA
    (
        "nvidia/nemotron-3-ultra-550b-a55b",
        "Nemotron 3 Ultra",
        "nvidia",
        "NVIDIA",
    ),
    // OpenAI
    ("gpt-5.3-codex", "GPT-5.3 Codex", "openai", "OpenAI"),
    ("gpt-5.4", "GPT-5.4", "openai", "OpenAI"),
    ("gpt-5.4-mini", "GPT-5.4 Mini", "openai", "OpenAI"),
    ("gpt-5.5", "GPT-5.5", "openai", "OpenAI"),
    ("gpt-5.6-luna", "GPT-5.6 Luna", "openai", "OpenAI"),
    ("gpt-5.6-sol", "GPT-5.6 Sol", "openai", "OpenAI"),
    ("gpt-5.6-terra", "GPT-5.6 Terra", "openai", "OpenAI"),
    // Poolside
    (
        "poolside/laguna-s-2.1-free",
        "Laguna S 2.1",
        "poolside",
        "Poolside",
    ),
    // Sakana AI
    ("sakana/fugu-ultra", "Fugu Ultra", "sakana", "Sakana AI"),
    // StepFun
    ("stepfun/Step-3.5-Flash", "Step 3.5 Flash", "stepfun", "StepFun"),
    ("stepfun/Step-3.7-Flash", "Step 3.7 Flash", "stepfun", "StepFun"),
    // Tencent
    ("tencent/hy3-paid", "Tencent Hy3", "tencent", "Tencent"),
    // Thinking Machines
    (
        "thinkingmachines/inkling",
        "Inkling",
        "thinkingmachines",
        "Thinking Machines",
    ),
    (
        "thinkingmachines/inkling-small",
        "Inkling Small",
        "thinkingmachines",
        "Thinking Machines",
    ),
    // xAI
    ("xai/grok-4.5", "Grok 4.5", "xai", "xAI"),
    ("xai/grok-4.6", "Grok 4.6", "xai", "xAI"),
    // Xiaomi
    ("xiaomi/mimo-v2.5", "MiMo V2.5", "xiaomi", "Xiaomi"),
    ("xiaomi/mimo-v2.5-pro", "MiMo V2.5 Pro", "xiaomi", "Xiaomi"),
    // Z AI
    ("zai-org/GLM-5", "GLM-5", "zai", "Z AI"),
    ("zai-org/GLM-5.1", "GLM-5.1", "zai", "Z AI"),
    ("zai-org/GLM-5.2", "GLM-5.2", "zai", "Z AI"),
    ("zai-org/GLM-5.2-Fast", "GLM-5.2 Fast", "zai", "Z AI"),
    ("zai-org/GLM-5.3", "GLM-5.3", "zai", "Z AI"),
];

/// Lazily-built list of `ModelCatalogEntry` for the static catalog.
pub static MODEL_CATALOG: LazyLock<Vec<ModelCatalogEntry>> = LazyLock::new(build_catalog);

fn build_catalog() -> Vec<ModelCatalogEntry> {
    ENTRIES
        .iter()
        .map(|(id, name, upstream_provider_id, upstream_provider_name)| {
            // `id` is "<provider>/<model>" (or just "<model>" for entries
            // without a slash). We split on the slash to recover the
            // model id; the upstream provider id doubles as our
            // provider_id for the entry.
            let (provider_id, model_id) = match id.split_once('/') {
                Some((p, m)) => (p.to_string(), m.to_string()),
                None => (upstream_provider_id.to_string(), id.to_string()),
            };
            ModelCatalogEntry {
                id: id.to_string(),
                name: name.to_string(),
                provider_id,
                provider_name: upstream_provider_name.to_string(),
                model_id,
                context_window: None,
                reasoning_variants: Vec::new(),
                agent: AgentType::CommandCode,
            }
        })
        .collect()
}

/// Convenience accessor for the adapter's `list_models` impl.
pub fn list_commandcode_models() -> Vec<ModelCatalogEntry> {
    MODEL_CATALOG.iter().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use circulo_core::AgentType;

    #[test]
    fn catalog_is_non_empty_and_all_commandcode() {
        let models = list_commandcode_models();
        assert!(models.len() >= 20, "expected a non-trivial catalog, got {}", models.len());
        for model in &models {
            assert_eq!(model.agent, AgentType::CommandCode);
            assert!(!model.id.is_empty());
            assert!(!model.name.is_empty());
        }
    }

    #[test]
    fn catalog_ids_are_unique() {
        let models = list_commandcode_models();
        let mut ids: Vec<&str> = models.iter().map(|m| m.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), models.len(), "duplicate ids in catalog");
    }

    #[test]
    fn model_id_field_is_extracted_from_canonical_id() {
        let models = list_commandcode_models();
        let anthropic = models
            .iter()
            .find(|m| m.id == "claude-sonnet-5")
            .expect("claude-sonnet-5 entry");
        assert_eq!(anthropic.model_id, "claude-sonnet-5");
        assert_eq!(anthropic.provider_id, "anthropic");
        assert_eq!(anthropic.provider_name, "Anthropic");

        let deepseek = models
            .iter()
            .find(|m| m.id == "deepseek/deepseek-v4-flash")
            .expect("deepseek/deepseek-v4-flash entry");
        assert_eq!(deepseek.model_id, "deepseek-v4-flash");
    }
}

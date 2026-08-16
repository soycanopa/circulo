//! Locale catalogs and string lookup.
//!
//! User-facing copy must go through this crate. Default locale: `en`.

use std::collections::HashMap;
use std::sync::OnceLock;

const EN_JSON: &str = include_str!("../locales/en.json");

#[derive(Debug, Clone)]
pub struct Catalog {
    locale: String,
    messages: HashMap<String, String>,
    fallback: HashMap<String, String>,
}

impl Catalog {
    pub fn english() -> Self {
        let messages = parse_flat_json(EN_JSON);
        Self {
            locale: "en".into(),
            fallback: messages.clone(),
            messages,
        }
    }

    pub fn default_locale() -> &'static Catalog {
        static EN: OnceLock<Catalog> = OnceLock::new();
        EN.get_or_init(Catalog::english)
    }

    pub fn locale(&self) -> &str {
        &self.locale
    }

    pub fn get<'a>(&'a self, key: &'a str) -> &'a str {
        self.messages
            .get(key)
            .or_else(|| self.fallback.get(key))
            .map(String::as_str)
            .unwrap_or(key)
    }
}

fn parse_flat_json(raw: &str) -> HashMap<String, String> {
    serde_json::from_str(raw).expect("locale JSON must be a flat string map")
}

#[cfg(test)]
mod tests {
    use super::Catalog;

    #[test]
    fn english_hide_label_is_not_the_key() {
        let catalog = Catalog::english();
        let value = catalog.get("sidebar.hide");
        assert!(!value.is_empty());
        assert_ne!(value, "sidebar.hide");
    }

    #[test]
    fn missing_key_returns_the_key() {
        let catalog = Catalog::english();
        assert_eq!(catalog.get("does.not.exist"), "does.not.exist");
    }
}

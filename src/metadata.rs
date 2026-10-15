//! Checked-in GitHub Linguist metadata and startup indexes.

use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug, Deserialize)]
pub struct Language {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub filenames: Vec<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub interpreters: Vec<String>,
}

pub struct Registry {
    pub languages: Vec<Language>,
    filenames: HashMap<String, Vec<usize>>,
    extensions: HashMap<String, Vec<usize>>,
    aliases: HashMap<String, usize>,
    interpreters: HashMap<String, Vec<usize>>,
}

impl Registry {
    fn load() -> Self {
        // Checked-in generator output is validated in tests; runtime never fetches data.
        let languages: Vec<Language> = serde_json::from_str(include_str!("../data/languages.json"))
            .expect("generated Linguist metadata must be valid JSON");
        let mut registry = Self {
            languages,
            filenames: HashMap::new(),
            extensions: HashMap::new(),
            aliases: HashMap::new(),
            interpreters: HashMap::new(),
        };
        for (index, language) in registry.languages.iter().enumerate() {
            registry.aliases.insert(language.name.to_lowercase(), index);
            for alias in &language.aliases {
                registry.aliases.insert(alias.to_lowercase(), index);
            }
            for name in &language.filenames {
                registry
                    .filenames
                    .entry(name.clone())
                    .or_default()
                    .push(index);
            }
            for extension in &language.extensions {
                registry
                    .extensions
                    .entry(extension.clone())
                    .or_default()
                    .push(index);
            }
            for interpreter in &language.interpreters {
                registry
                    .interpreters
                    .entry(interpreter.clone())
                    .or_default()
                    .push(index);
            }
        }
        registry
    }

    pub fn by_name(&self, name: &str) -> Option<&Language> {
        self.aliases
            .get(&name.to_lowercase())
            .map(|&index| &self.languages[index])
    }

    pub fn by_filename(&self, name: &str) -> Vec<&Language> {
        self.filenames
            .get(name)
            .map_or_else(Vec::new, |indices| self.select(indices))
    }

    /// Return candidates for the longest matching compound suffix.
    pub fn by_extension(&self, name: &str) -> Vec<&Language> {
        name.match_indices('.')
            .filter_map(|(start, _)| {
                self.extensions
                    .get(&name[start..])
                    .map(|indices| (name.len() - start, indices))
            })
            .max_by_key(|(length, _)| *length)
            .map_or_else(Vec::new, |(_, indices)| self.select(indices))
    }

    pub fn by_interpreter(&self, name: &str) -> Vec<&Language> {
        self.interpreters
            .get(name)
            .map_or_else(Vec::new, |indices| self.select(indices))
    }

    fn select(&self, indices: &[usize]) -> Vec<&Language> {
        indices
            .iter()
            .map(|&index| &self.languages[index])
            .collect()
    }
}

static REGISTRY: OnceLock<Registry> = OnceLock::new();

pub fn registry() -> &'static Registry {
    REGISTRY.get_or_init(Registry::load)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_metadata_has_rust_and_compound_extensions() {
        let data = registry();
        assert!(data.languages.len() > 800);
        assert!(
            data.by_extension("main.rs")
                .iter()
                .any(|lang| lang.name == "Rust")
        );
        assert_eq!(
            data.by_name("rs").map(|lang| lang.name.as_str()),
            Some("Rust")
        );
    }
}

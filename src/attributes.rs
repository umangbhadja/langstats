//! Supported subset of nested Git attributes for Linguist overrides.

use globset::{GlobBuilder, GlobMatcher};
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

#[derive(Default, Clone)]
pub struct Overrides {
    pub language: Option<String>,
    pub detectable: Option<bool>,
    pub generated: Option<bool>,
    pub vendored: Option<bool>,
    pub documentation: Option<bool>,
}

struct Rule {
    base: PathBuf,
    matcher: GlobMatcher,
    basename_only: bool,
    attrs: Overrides,
}

#[derive(Default)]
pub struct Attributes {
    rules: Vec<Rule>,
}

impl Attributes {
    pub fn load(root: &Path) -> (Self, Vec<String>) {
        let mut files = Vec::new();
        let mut walker = WalkBuilder::new(root);
        walker
            .hidden(false)
            .git_ignore(false)
            .git_exclude(false)
            .git_global(false)
            .ignore(false)
            .follow_links(false);
        walker.filter_entry(|entry| entry.file_name() != ".git");
        for entry in walker.build().flatten() {
            if entry.file_type().is_some_and(|kind| kind.is_file())
                && entry.file_name() == ".gitattributes"
            {
                files.push(entry.into_path());
            }
        }
        files.sort_by_key(|path| path.components().count());
        let mut result = Self::default();
        let mut warnings = Vec::new();
        for file in files {
            match std::fs::read_to_string(&file) {
                Ok(text) => result.parse(&file, &text, &mut warnings),
                Err(error) => warnings.push(format!("{}: {error}", file.display())),
            }
        }
        (result, warnings)
    }

    fn parse(&mut self, file: &Path, text: &str, warnings: &mut Vec<String>) {
        let Some(base) = file.parent() else { return };
        for (line_number, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut words = line.split_whitespace();
            let Some(pattern) = words.next() else {
                continue;
            };
            let basename_only = !pattern.contains('/');
            let pattern = pattern.trim_start_matches('/');
            let matcher = match GlobBuilder::new(pattern).literal_separator(true).build() {
                Ok(glob) => glob.compile_matcher(),
                Err(error) => {
                    warnings.push(format!(
                        "{}:{}: invalid attribute pattern: {error}",
                        file.display(),
                        line_number + 1
                    ));
                    continue;
                }
            };
            let mut attrs = Overrides::default();
            for word in words {
                if let Some(lang) = word.strip_prefix("linguist-language=") {
                    attrs.language = Some(lang.to_owned());
                } else {
                    let (value, name) = if let Some(name) = word.strip_prefix('-') {
                        (false, name)
                    } else {
                        (true, word)
                    };
                    match name {
                        "linguist-detectable" => attrs.detectable = Some(value),
                        "linguist-generated" => attrs.generated = Some(value),
                        "linguist-vendored" => attrs.vendored = Some(value),
                        "linguist-documentation" => attrs.documentation = Some(value),
                        _ => {}
                    }
                }
            }
            self.rules.push(Rule {
                base: base.to_path_buf(),
                matcher,
                basename_only,
                attrs,
            });
        }
    }

    pub fn for_path(&self, path: &Path) -> Overrides {
        let mut result = Overrides::default();
        for rule in &self.rules {
            let Ok(relative) = path.strip_prefix(&rule.base) else {
                continue;
            };
            let subject = if rule.basename_only {
                relative.file_name().map(Path::new)
            } else {
                Some(relative)
            };
            if !subject.is_some_and(|subject| rule.matcher.is_match(subject)) {
                continue;
            }
            if let Some(lang) = &rule.attrs.language {
                result.language = Some(lang.clone());
            }
            if let Some(value) = rule.attrs.detectable {
                result.detectable = Some(value);
            }
            if let Some(value) = rule.attrs.generated {
                result.generated = Some(value);
            }
            if let Some(value) = rule.attrs.vendored {
                result.vendored = Some(value);
            }
            if let Some(value) = rule.attrs.documentation {
                result.documentation = Some(value);
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_rule_overrides_parent() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        std::fs::write(
            dir.path().join(".gitattributes"),
            "*.rs linguist-language=Python\n",
        )?;
        std::fs::create_dir(dir.path().join("nested"))?;
        std::fs::write(
            dir.path().join("nested/.gitattributes"),
            "*.rs linguist-language=Rust\n",
        )?;
        let (attrs, warnings) = Attributes::load(dir.path());
        assert!(warnings.is_empty());
        assert_eq!(
            attrs.for_path(&dir.path().join("a.rs")).language.as_deref(),
            Some("Python")
        );
        assert_eq!(
            attrs
                .for_path(&dir.path().join("nested/a.rs"))
                .language
                .as_deref(),
            Some("Rust")
        );
        Ok(())
    }
}

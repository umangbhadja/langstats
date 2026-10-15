//! Language detection with bounded content reads.

use crate::metadata::{Language, registry};
use std::path::Path;

/// Classify a file using Linguist metadata and a bounded prefix.
pub fn detect(path: &Path, prefix: &[u8]) -> Option<&'static Language> {
    if prefix.contains(&0) {
        return None;
    }
    let name = path.file_name()?.to_string_lossy();
    let name = name.as_ref();
    let data = registry();
    let text = String::from_utf8_lossy(prefix);

    if let Some(lang) = modeline(&text).and_then(|name| data.by_name(name)) {
        return Some(lang);
    }
    if let Some(lang) = data.by_filename(name).into_iter().next() {
        return Some(lang);
    }
    if let Some(interpreter) = shebang(&text) {
        let interpreters = data.by_interpreter(interpreter);
        if interpreter == "node" {
            let extensions = data.by_extension(name);
            if let Some(lang) = interpreters.iter().find(|lang| {
                extensions
                    .iter()
                    .any(|candidate| candidate.name == lang.name)
            }) {
                return Some(lang);
            }
            if let Some(lang) = interpreters.iter().find(|lang| lang.name == "TypeScript") {
                return Some(lang);
            }
        }
        if let Some(lang) = interpreters.into_iter().next() {
            return Some(lang);
        }
    }
    let candidates = data.by_extension(name);
    if candidates.len() == 1 {
        return candidates.into_iter().next();
    }
    if candidates.len() > 1 {
        if let Some(lang) = ambiguous(name, &text, &candidates) {
            return Some(lang);
        }
        // Stable fallback for ambiguous suffixes; documented as a compatibility gap.
        return candidates
            .into_iter()
            .find(|lang| lang.kind == "programming");
    }
    None
}

fn modeline(text: &str) -> Option<&str> {
    for line in text.lines().take(5) {
        if let Some(index) = line.find("-*- mode:") {
            return line[index + 9..]
                .split([';', '*', ' '])
                .find(|s| !s.is_empty());
        }
        if let Some(index) = line.find("vim: set ft=") {
            return line[index + 12..].split([':', ' ', '.']).next();
        }
    }
    None
}

fn shebang(text: &str) -> Option<&str> {
    let first = text.lines().next()?.strip_prefix("#!")?.trim();
    let mut words = first.split_whitespace();
    let executable = words.next()?.rsplit('/').next()?;
    if executable == "env" {
        words
            .find(|word| !word.starts_with('-'))
            .and_then(|word| word.rsplit('/').next())
    } else {
        Some(executable)
    }
}

fn ambiguous<'a>(name: &str, text: &str, candidates: &[&'a Language]) -> Option<&'a Language> {
    let prefer = if name.ends_with(".rs") {
        if text.trim_start().starts_with("#pragma version(") {
            "RenderScript"
        } else {
            "Rust"
        }
    } else if name.ends_with(".md") {
        "Markdown"
    } else if name.ends_with(".html") {
        "HTML"
    } else if name.ends_with(".php") {
        if text.trim_start().starts_with("<?hh") {
            "Hack"
        } else {
            "PHP"
        }
    } else if name.ends_with(".h") {
        if text.contains("@interface") || text.contains("@protocol") {
            "Objective-C"
        } else if text.contains("template<") || text.contains("namespace ") {
            "C++"
        } else {
            "C"
        }
    } else if name.ends_with(".m") {
        if text.contains("@interface") || text.contains("#import ") {
            "Objective-C"
        } else {
            "MATLAB"
        }
    } else {
        return None;
    };
    candidates.iter().copied().find(|lang| lang.name == prefer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_ambiguous_rust_and_binary() {
        assert_eq!(
            detect(Path::new("main.rs"), b"fn main() {}\n").map(|x| x.name.as_str()),
            Some("Rust")
        );
        assert!(detect(Path::new("main.rs"), b"fn\0main").is_none());
    }

    #[test]
    fn detects_shebang_and_modeline() {
        assert_eq!(
            detect(Path::new("script"), b"#!/usr/bin/env python3\nprint(1)\n")
                .map(|x| x.name.as_str()),
            Some("Python")
        );
        assert_eq!(
            detect(Path::new("unknown"), b"# -*- mode: ruby -*-\n").map(|x| x.name.as_str()),
            Some("Ruby")
        );
    }

    #[test]
    fn html_wins_ambiguous_markup_extension() {
        assert_eq!(
            detect(Path::new("index.html"), b"<h1>Hello</h1>\n").map(|lang| lang.name.as_str()),
            Some("HTML")
        );
    }

    #[test]
    fn php_and_hack_share_extension() {
        assert_eq!(
            detect(Path::new("index.php"), b"<?php echo 1;\n").map(|lang| lang.name.as_str()),
            Some("PHP")
        );
        assert_eq!(
            detect(Path::new("index.php"), b"<?hh\necho 1;\n").map(|lang| lang.name.as_str()),
            Some("Hack")
        );
    }

    #[test]
    fn node_shebang_uses_extension_when_available() {
        let prefix = b"#!/usr/bin/env node\nconsole.log(1)\n";
        assert_eq!(
            detect(Path::new("serve"), prefix).map(|lang| lang.name.as_str()),
            Some("TypeScript")
        );
        assert_eq!(
            detect(Path::new("serve.js"), prefix).map(|lang| lang.name.as_str()),
            Some("JavaScript")
        );
        assert_eq!(
            detect(Path::new("serve.ts"), prefix).map(|lang| lang.name.as_str()),
            Some("TypeScript")
        );
    }
}

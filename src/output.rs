//! Deterministic JSON and terminal reports.

use crate::scanner::ScanResult;
use serde::Serialize;

#[derive(Clone, Copy)]
pub enum Sort {
    Bytes,
    Name,
    Files,
}

#[derive(Clone, Copy)]
pub struct Filter {
    pub top: Option<usize>,
    pub min_percent: f64,
    pub min_bytes: u64,
    pub sort: Sort,
}

#[derive(Serialize)]
pub struct Report<'a> {
    pub schema_version: u32,
    pub tool: Tool,
    pub repository: Repository<'a>,
    pub summary: Summary,
    pub languages: Vec<Language>,
    pub warnings: Vec<String>,
}

#[derive(Serialize)]
pub struct Tool {
    pub name: &'static str,
    pub version: &'static str,
}
#[derive(Serialize)]
pub struct Repository<'a> {
    pub path: &'a str,
    pub name: &'a str,
}
#[derive(Serialize)]
pub struct Summary {
    pub files_scanned: u64,
    pub files_counted: u64,
    pub total_bytes: u64,
    pub language_count: usize,
    pub warnings_truncated: u64,
}
#[derive(Serialize)]
pub struct Language {
    pub name: String,
    pub bytes: u64,
    pub percentage: f64,
    pub files: u64,
    pub color: Option<String>,
}

pub fn build_report<'a>(
    result: &ScanResult,
    path: &'a str,
    name: &'a str,
    filter: Filter,
) -> Report<'a> {
    let total = result.total_bytes();
    let mut languages: Vec<_> = result
        .languages
        .iter()
        .map(|(name, count)| Language {
            name: name.clone(),
            bytes: count.bytes,
            percentage: if total == 0 {
                0.0
            } else {
                count.bytes as f64 * 100.0 / total as f64
            },
            files: count.files,
            color: count.color.clone(),
        })
        .collect();
    languages
        .retain(|lang| lang.bytes >= filter.min_bytes && lang.percentage >= filter.min_percent);
    languages.sort_by(|a, b| match filter.sort {
        Sort::Bytes => b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)),
        Sort::Name => a.name.cmp(&b.name),
        Sort::Files => b.files.cmp(&a.files).then_with(|| a.name.cmp(&b.name)),
    });
    if let Some(top) = filter.top {
        languages.truncate(top);
    }
    Report {
        schema_version: 1,
        tool: Tool {
            name: "langstats",
            version: env!("CARGO_PKG_VERSION"),
        },
        repository: Repository { path, name },
        summary: Summary {
            files_scanned: result.files_scanned,
            files_counted: result.files_counted,
            total_bytes: total,
            language_count: result.languages.len(),
            warnings_truncated: result.skipped_warnings,
        },
        languages,
        warnings: result.warnings.clone(),
    }
}

/// Escape untrusted text before writing it to a human-readable terminal stream.
pub fn escape_terminal(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        if character.is_control()
            || matches!(character, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
        {
            escaped.extend(character.escape_default());
        } else {
            escaped.push(character);
        }
    }
    escaped
}

pub fn terminal(report: &Report<'_>, color: bool, quiet: bool) -> String {
    let mut output = String::new();
    if !quiet {
        output.push_str(&format!(
            "Repository: {}\nFiles analyzed: {}\nTotal bytes: {}\n\nLanguages\n\n",
            escape_terminal(report.repository.name),
            report.summary.files_counted,
            report.summary.total_bytes
        ));
    }
    for lang in &report.languages {
        let bars = (lang.percentage / 2.0).round() as usize;
        let bar = "█".repeat(bars.max(usize::from(lang.bytes > 0)));
        let bar = if color {
            if let Some((r, g, b)) = lang.color.as_deref().and_then(parse_color) {
                format!("\x1b[38;2;{r};{g};{b}m{bar}\x1b[0m")
            } else {
                bar
            }
        } else {
            bar
        };
        output.push_str(&format!(
            "{:<18} {:>6.2}%  {}\n",
            escape_terminal(&lang.name),
            lang.percentage,
            bar
        ));
    }
    if report.languages.is_empty() && !quiet {
        output.push_str(if report.summary.files_counted == 0 {
            "No counted source files.\n"
        } else {
            "No languages match the display filters.\n"
        });
    }
    output
}

fn parse_color(hex: &str) -> Option<(u8, u8, u8)> {
    let hex = hex.strip_prefix('#')?;
    if hex.len() != 6 || !hex.is_ascii() {
        return None;
    }
    Some((
        u8::from_str_radix(&hex[..2], 16).ok()?,
        u8::from_str_radix(&hex[2..4], 16).ok()?,
        u8::from_str_radix(&hex[4..], 16).ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::Counts;
    use std::collections::BTreeMap;

    #[test]
    fn terminal_text_escapes_controls_and_keeps_unicode() {
        let escaped = escape_terminal("café\x1b[2J\n\r\x07\u{009b}\u{202e}");
        assert!(escaped.starts_with("café\\u{1b}[2J\\n\\r"));
        assert!(!escaped.chars().any(char::is_control));
        assert!(!escaped.contains('\u{202e}'));
        assert_eq!(parse_color("#aé123"), None);
    }

    #[test]
    fn filtered_percent_uses_all_counted_bytes() {
        let result = ScanResult {
            files_scanned: 2,
            files_counted: 2,
            languages: BTreeMap::from([
                (
                    "Rust".into(),
                    Counts {
                        bytes: 3,
                        files: 1,
                        color: None,
                    },
                ),
                (
                    "Python".into(),
                    Counts {
                        bytes: 1,
                        files: 1,
                        color: None,
                    },
                ),
            ]),
            ..ScanResult::default()
        };
        let report = build_report(
            &result,
            ".",
            "repo",
            Filter {
                top: Some(1),
                min_percent: 0.0,
                min_bytes: 0,
                sort: Sort::Bytes,
            },
        );
        assert_eq!(report.summary.total_bytes, 4);
        assert_eq!(report.languages[0].percentage, 75.0);
    }
}

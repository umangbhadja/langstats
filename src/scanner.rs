//! Bounded work-queue scanner and byte aggregation.

use crate::{Error, validate_path};
use crate::{attributes::Attributes, detection, metadata};
use ignore::WalkBuilder;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};

#[derive(Debug, Clone)]
pub struct ScanOptions {
    pub threads: usize,
    pub include_ignored: bool,
    pub include_hidden: bool,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            threads: std::thread::available_parallelism().map_or(1, usize::from),
            include_ignored: false,
            include_hidden: false,
        }
    }
}

#[derive(Debug, Default)]
pub struct Counts {
    pub bytes: u64,
    pub files: u64,
    pub color: Option<String>,
}

#[derive(Debug, Default)]
pub struct ScanResult {
    pub files_scanned: u64,
    pub files_counted: u64,
    pub languages: BTreeMap<String, Counts>,
    pub warnings: Vec<String>,
    pub skipped_warnings: u64,
}

impl ScanResult {
    fn merge(&mut self, other: Self) {
        self.files_scanned = self.files_scanned.saturating_add(other.files_scanned);
        self.files_counted = self.files_counted.saturating_add(other.files_counted);
        for (name, count) in other.languages {
            let target = self.languages.entry(name).or_default();
            target.bytes = target.bytes.saturating_add(count.bytes);
            target.files = target.files.saturating_add(count.files);
            if target.color.is_none() {
                target.color = count.color;
            }
        }
        for warning in other.warnings {
            self.warn(warning);
        }
        self.skipped_warnings = self.skipped_warnings.saturating_add(other.skipped_warnings);
    }

    fn warn(&mut self, warning: String) {
        let index = self.warnings.binary_search(&warning).unwrap_or_else(|i| i);
        if index >= 100 {
            self.skipped_warnings = self.skipped_warnings.saturating_add(1);
            return;
        }
        self.warnings.insert(index, warning);
        if self.warnings.len() > 100 {
            self.warnings.pop();
            self.skipped_warnings = self.skipped_warnings.saturating_add(1);
        }
    }

    pub fn total_bytes(&self) -> u64 {
        self.languages
            .values()
            .fold(0_u64, |total, counts| total.saturating_add(counts.bytes))
    }
}

struct Task {
    path: PathBuf,
    size: u64,
}

/// Scan one local directory without following symlinks.
pub fn scan(root: &Path, options: &ScanOptions) -> Result<ScanResult, Error> {
    validate_path(root)?;
    let workers = options.threads.clamp(1, 256);
    let (sender, receiver) = mpsc::sync_channel::<Task>(workers * 16);
    let receiver = Arc::new(Mutex::new(receiver));
    let mut result = ScanResult::default();
    let (attributes, attribute_warnings) = Attributes::load(root);
    for warning in attribute_warnings {
        result.warn(warning);
    }

    std::thread::scope(|scope| {
        let attributes = &attributes;
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                let receiver = Arc::clone(&receiver);
                scope.spawn(move || {
                    let mut local = ScanResult::default();
                    loop {
                        let next = match receiver.lock() {
                            Ok(queue) => queue.recv(),
                            Err(_) => break,
                        };
                        match next {
                            Ok(task) => process_file(task, root, attributes, &mut local),
                            Err(_) => break,
                        }
                    }
                    local
                })
            })
            .collect();

        let mut walker = WalkBuilder::new(root);
        walker
            .hidden(!options.include_hidden)
            .follow_links(false)
            .require_git(false);
        if options.include_ignored {
            walker
                .git_ignore(false)
                .git_exclude(false)
                .git_global(false)
                .ignore(false);
        }
        walker.filter_entry(|entry| entry.file_name() != ".git");
        for entry in walker.build() {
            match entry {
                Ok(entry) if entry.file_type().is_some_and(|kind| kind.is_file()) => {
                    match entry.metadata() {
                        Ok(metadata) => {
                            if sender
                                .send(Task {
                                    path: entry.into_path(),
                                    size: metadata.len(),
                                })
                                .is_err()
                            {
                                result.warn("scanner workers stopped unexpectedly".into());
                                break;
                            }
                        }
                        Err(error) => result.warn(format!("{}: {error}", entry.path().display())),
                    }
                }
                Err(error) => result.warn(error.to_string()),
                _ => {}
            }
        }
        drop(sender);
        for handle in handles {
            match handle.join() {
                Ok(local) => result.merge(local),
                Err(_) => result.warn("scanner worker panicked".into()),
            }
        }
    });
    Ok(result)
}

fn process_file(task: Task, root: &Path, attributes: &Attributes, result: &mut ScanResult) {
    result.files_scanned = result.files_scanned.saturating_add(1);
    let mut file = match File::open(&task.path) {
        Ok(file) => file,
        Err(error) => {
            result.warn(format!("{}: {error}", task.path.display()));
            return;
        }
    };
    let mut prefix = [0_u8; 8192];
    let mut length = 0;
    while length < prefix.len() {
        match file.read(&mut prefix[length..]) {
            Ok(0) => break,
            Ok(read) => length += read,
            Err(error) => {
                result.warn(format!("{}: {error}", task.path.display()));
                return;
            }
        }
    }
    let overrides = attributes.for_path(&task.path);
    let classifications = classify_path(root, &task.path);
    if overrides.detectable == Some(false)
        || overrides.generated.unwrap_or(classifications.0)
        || overrides.vendored.unwrap_or(classifications.1)
        || overrides.documentation.unwrap_or(classifications.2)
    {
        return;
    }
    let language = if let Some(name) = overrides.language.as_deref() {
        let found = metadata::registry().by_name(name);
        if found.is_none() {
            result.warn(format!(
                "{}: unknown linguist-language: {name}",
                task.path.display()
            ));
        }
        found
    } else {
        detection::detect(&task.path, &prefix[..length])
    };
    let Some(language) = language else {
        return;
    };
    if overrides.detectable != Some(true)
        && language.kind != "programming"
        && language.kind != "markup"
    {
        return;
    }
    let name = language.group.as_deref().unwrap_or(&language.name);
    let count = result.languages.entry(name.to_owned()).or_default();
    count.files = count.files.saturating_add(1);
    count.bytes = count.bytes.saturating_add(task.size);
    if count.color.is_none() {
        count.color = metadata::registry()
            .by_name(name)
            .map(|group| group.color.clone())
            .unwrap_or_else(|| language.color.clone());
    }
    result.files_counted = result.files_counted.saturating_add(1);
}

fn classify_path(root: &Path, path: &Path) -> (bool, bool, bool) {
    let relative = path.strip_prefix(root).unwrap_or(path);
    let name = relative
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    let generated = name.ends_with(".min.js") || name.ends_with(".min.css");
    let mut vendored = generated;
    let mut documentation = false;
    if let Some(parent) = relative.parent() {
        for component in parent.components() {
            let value = component.as_os_str();
            vendored |= value == "vendor" || value == "node_modules" || value == "third_party";
            documentation |= value == "docs" || value == "doc" || value == "documentation";
        }
    }
    (generated, vendored, documentation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_bytes_and_respects_ignores() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join(".gitignore"), "ignored.rs\n")?;
        std::fs::write(dir.path().join("main.rs"), "fn main() {}\n")?;
        std::fs::write(dir.path().join("ignored.rs"), "ignored\n")?;
        std::fs::write(dir.path().join("image.rs"), b"\0binary")?;
        let result = scan(
            dir.path(),
            &ScanOptions {
                threads: 2,
                ..ScanOptions::default()
            },
        )?;
        assert_eq!(result.languages.get("Rust").map(|x| x.bytes), Some(13));
        assert_eq!(result.files_counted, 1);
        let included = scan(
            dir.path(),
            &ScanOptions {
                threads: 2,
                include_ignored: true,
                ..ScanOptions::default()
            },
        )?;
        assert_eq!(included.languages.get("Rust").map(|x| x.bytes), Some(21));
        Ok(())
    }

    #[test]
    fn attributes_override_language_and_exclusions() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        std::fs::write(
            dir.path().join(".gitattributes"),
            "a.rs linguist-language=Python\nb.rs linguist-generated\nREADME.md linguist-detectable\n",
        )?;
        std::fs::write(dir.path().join("a.rs"), "abc")?;
        std::fs::write(dir.path().join("b.rs"), "abc")?;
        std::fs::write(dir.path().join("README.md"), "abc")?;
        let result = scan(
            dir.path(),
            &ScanOptions {
                threads: 2,
                ..ScanOptions::default()
            },
        )?;
        assert_eq!(result.languages.get("Python").map(|c| c.bytes), Some(3));
        assert_eq!(result.languages.get("Markdown").map(|c| c.bytes), Some(3));
        assert_eq!(result.files_counted, 2);
        Ok(())
    }

    #[test]
    fn known_generated_and_vendor_paths_allow_false_override()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        std::fs::create_dir(dir.path().join("vendor"))?;
        std::fs::write(dir.path().join("vendor/a.rs"), "abc")?;
        std::fs::write(dir.path().join("app.min.js"), "abc")?;
        assert_eq!(scan(dir.path(), &ScanOptions::default())?.files_counted, 0);
        std::fs::write(
            dir.path().join(".gitattributes"),
            "vendor/*.rs -linguist-vendored\n*.min.js -linguist-generated\n",
        )?;
        assert_eq!(scan(dir.path(), &ScanOptions::default())?.files_counted, 1);
        std::fs::write(
            dir.path().join(".gitattributes"),
            "vendor/*.rs -linguist-vendored\n*.min.js -linguist-generated -linguist-vendored\n",
        )?;
        assert_eq!(scan(dir.path(), &ScanOptions::default())?.files_counted, 2);
        Ok(())
    }

    #[test]
    fn hidden_and_empty_files_have_defined_counts() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join(".hidden.rs"), "abc")?;
        std::fs::write(dir.path().join("empty.rs"), "")?;
        let normal = scan(dir.path(), &ScanOptions::default())?;
        assert_eq!(normal.files_counted, 1);
        assert_eq!(normal.total_bytes(), 0);
        let shown = scan(
            dir.path(),
            &ScanOptions {
                include_hidden: true,
                ..ScanOptions::default()
            },
        )?;
        assert_eq!(shown.files_counted, 2);
        assert_eq!(shown.total_bytes(), 3);
        Ok(())
    }

    #[test]
    fn unusual_paths_and_nested_ignore() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let source = dir.path().join("source files");
        std::fs::create_dir(&source)?;
        std::fs::write(source.join("café.py"), "print(1)\n")?;
        std::fs::write(source.join(".gitignore"), "generated/\n")?;
        std::fs::create_dir(source.join("generated"))?;
        std::fs::write(source.join("generated/skip.py"), "print(2)\n")?;
        let result = scan(dir.path(), &ScanOptions::default())?;
        assert_eq!(result.files_counted, 1);
        assert_eq!(
            result.languages.get("Python").map(|count| count.bytes),
            Some(9)
        );
        Ok(())
    }

    #[test]
    fn grouped_language_uses_group_color() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join("first.mligo"), "let x = 1\n")?;
        std::fs::write(dir.path().join("second.ligo"), "let x = 2\n")?;
        let result = scan(
            dir.path(),
            &ScanOptions {
                threads: 2,
                ..ScanOptions::default()
            },
        )?;
        let group = result.languages.get("LigoLANG").expect("group present");
        assert_eq!(group.files, 2);
        assert_eq!(group.color.as_deref(), Some("#0e74ff"));
        Ok(())
    }

    #[test]
    fn warnings_keep_smallest_paths_across_worker_merges() {
        let mut first = ScanResult::default();
        let mut second = ScanResult::default();
        for index in (120..240).rev() {
            first.warn(format!("warning-{index:03}"));
        }
        for index in (0..120).rev() {
            second.warn(format!("warning-{index:03}"));
        }
        first.merge(second);
        assert_eq!(first.warnings.len(), 100);
        assert_eq!(
            first.warnings.first().map(String::as_str),
            Some("warning-000")
        );
        assert_eq!(
            first.warnings.last().map(String::as_str),
            Some("warning-099")
        );
        assert_eq!(first.skipped_warnings, 140);
    }

    #[cfg(windows)]
    #[test]
    fn directory_symlink_is_not_followed() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join("main.rs"), "abc")?;
        if let Err(error) = std::os::windows::fs::symlink_dir(dir.path(), dir.path().join("loop")) {
            if error.kind() == std::io::ErrorKind::PermissionDenied {
                eprintln!("Windows symlink creation denied; symlink behavior unverified");
                return Ok(());
            }
            return Err(error.into());
        }
        let result = scan(dir.path(), &ScanOptions::default())?;
        assert_eq!(result.files_counted, 1);
        assert_eq!(result.total_bytes(), 3);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn symlink_skipped_and_non_utf8_name_detected() -> Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::ffi::OsStringExt;
        let dir = tempfile::tempdir()?;
        let name = std::ffi::OsString::from_vec(b"nonutf\xff.rs".to_vec());
        assert_eq!(
            crate::detection::detect(Path::new(&name), b"fn f() {}").map(|lang| lang.name.as_str()),
            Some("Rust")
        );
        std::fs::write(dir.path().join("a.rs"), "abc")?;
        if let Err(error) = std::os::unix::fs::symlink(dir.path(), dir.path().join("loop")) {
            if error.kind() != std::io::ErrorKind::PermissionDenied {
                return Err(error.into());
            }
        }
        let result = scan(dir.path(), &ScanOptions::default())?;
        assert_eq!(result.files_counted, 1);
        assert_eq!(result.total_bytes(), 3);
        Ok(())
    }
}

use std::process::Command;

#[test]
fn json_is_valid_and_deterministic() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    std::fs::write(dir.path().join("main.rs"), "fn main() {}\n")?;
    std::fs::write(dir.path().join("README.md"), "docs\n")?;
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_langstats"))
            .arg("--json")
            .arg(dir.path())
            .output()
    };
    let first = run()?;
    let second = run()?;
    assert!(first.status.success());
    assert_eq!(first.stdout, second.stdout);
    let json: serde_json::Value = serde_json::from_slice(&first.stdout)?;
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["summary"]["total_bytes"], 13);
    assert_eq!(json["languages"][0]["name"], "Rust");
    Ok(())
}

#[test]
fn invalid_directory_fails_cleanly() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_langstats"))
        .arg("/this/path/should/not/exist/langstats")
        .output()?;
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("not a directory"));
    Ok(())
}

#[test]
fn empty_policy_and_numeric_validation() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let empty = Command::new(env!("CARGO_BIN_EXE_langstats"))
        .arg("--fail-on-empty")
        .arg(dir.path())
        .output()?;
    assert_eq!(empty.status.code(), Some(1));
    let invalid = Command::new(env!("CARGO_BIN_EXE_langstats"))
        .args(["--threads", "0"])
        .arg(dir.path())
        .output()?;
    assert_eq!(invalid.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("--threads"));
    Ok(())
}

#[test]
fn binary_override_is_excluded_and_filtered_output_is_accurate()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    std::fs::write(
        dir.path().join(".gitattributes"),
        "*.rs linguist-language=Rust\n",
    )?;
    std::fs::write(dir.path().join("binary.rs"), b"\0binary")?;
    std::fs::write(dir.path().join("main.rs"), "fn main() {}\n")?;
    let output = Command::new(env!("CARGO_BIN_EXE_langstats"))
        .arg("--json")
        .arg(dir.path())
        .output()?;
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(json["summary"]["files_counted"], 1);
    assert_eq!(json["summary"]["total_bytes"], 13);
    let filtered = Command::new(env!("CARGO_BIN_EXE_langstats"))
        .args(["--top", "0"])
        .arg(dir.path())
        .output()?;
    assert!(
        String::from_utf8_lossy(&filtered.stdout)
            .contains("No languages match the display filters.")
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn untrusted_paths_and_warnings_cannot_emit_terminal_controls()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("repo\x1b[2J\n\r\x1b]0;title\x07");
    std::fs::create_dir(&root)?;
    std::fs::write(root.join("main.rs"), "fn main() {}\n")?;
    std::fs::write(
        root.join(".gitattributes"),
        "*.rs linguist-language=Unknown\n",
    )?;
    let output = Command::new(env!("CARGO_BIN_EXE_langstats"))
        .arg("--no-color")
        .arg(&root)
        .output()?;
    assert!(output.status.success());
    for stream in [&output.stdout, &output.stderr] {
        assert!(
            !stream
                .iter()
                .any(|byte| matches!(byte, 0x1b | 0x07 | b'\r'))
        );
        assert!(String::from_utf8_lossy(stream).contains("\\u{1b}[2J\\n\\r"));
    }
    let json = Command::new(env!("CARGO_BIN_EXE_langstats"))
        .arg("--json")
        .arg(&root)
        .output()?;
    let json: serde_json::Value = serde_json::from_slice(&json.stdout)?;
    assert_eq!(json["repository"]["path"], root.to_string_lossy().as_ref());
    Ok(())
}

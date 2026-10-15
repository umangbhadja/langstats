//! Offline language statistics for local directories.

use std::path::{Path, PathBuf};

pub mod attributes;
pub mod detection;
pub mod metadata;
pub mod output;
pub mod scanner;

/// Errors returned before or during a scan.
#[derive(Debug)]
pub enum Error {
    InvalidPath(PathBuf),
    Io(std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPath(path) => write!(f, "not a directory: {}", path.display()),
            Self::Io(err) => write!(f, "I/O error: {err}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

/// Validate scan root without modifying its contents.
pub fn validate_path(path: &Path) -> Result<(), Error> {
    if path.is_dir() {
        Ok(())
    } else {
        Err(Error::InvalidPath(path.to_path_buf()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_directory_and_rejects_file() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let file = dir.path().join("file");
        std::fs::write(&file, b"x")?;
        assert!(validate_path(dir.path()).is_ok());
        assert!(matches!(validate_path(&file), Err(Error::InvalidPath(_))));
        Ok(())
    }
}

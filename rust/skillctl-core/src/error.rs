use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum SkillctlError {
    #[error("config error: {0}")]
    Config(String),
    #[error("filesystem error at {path}: {source}")]
    Fs {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("unmanaged conflict at {0}")]
    UnmanagedConflict(PathBuf),
    #[error("foreign lockfile owner at {path}: expected {expected}, found {found}")]
    ForeignLockOwner {
        path: PathBuf,
        expected: String,
        found: String,
    },
}

pub type Result<T, E = SkillctlError> = std::result::Result<T, E>;

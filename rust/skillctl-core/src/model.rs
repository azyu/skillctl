use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Plan,
    Apply,
    Doctor,
    Update,
    List,
    Prune,
    Unlink {
        skill: String,
        target: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: u8,
}

impl CommandOutput {
    pub fn success(stdout: impl Into<String>) -> Self {
        Self {
            stdout: stdout.into(),
            stderr: String::new(),
            exit_code: 0,
        }
    }

    pub fn failure(stderr: impl Into<String>, exit_code: u8) -> Self {
        Self {
            stdout: String::new(),
            stderr: stderr.into(),
            exit_code,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetRef {
    pub name: String,
    pub path: PathBuf,
}

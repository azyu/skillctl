use crate::error::{Result, SkillctlError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TargetLock {
    pub version: u32,
    pub tool: String,
    pub target: String,
    pub source_root: PathBuf,
    pub target_path: PathBuf,
    #[serde(default)]
    pub managed: BTreeMap<String, ManagedEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManagedEntry {
    pub skill_id: String,
    pub target_name: String,
    pub target_path: PathBuf,
    pub rendered_path: PathBuf,
    pub source_path: PathBuf,
    pub method: String,
    pub source_digest: String,
}

impl TargetLock {
    pub fn new(target: impl Into<String>, source_root: PathBuf, target_path: PathBuf) -> Self {
        Self {
            version: 1,
            tool: "skillctl".to_string(),
            target: target.into(),
            source_root,
            target_path,
            managed: BTreeMap::new(),
        }
    }

    pub fn read_or_empty(
        path: &Path,
        target: &str,
        source_root: &Path,
        target_path: &Path,
    ) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::new(
                target,
                source_root.to_path_buf(),
                target_path.to_path_buf(),
            ));
        }
        let text = fs::read_to_string(path).map_err(|source| SkillctlError::Fs {
            path: path.to_path_buf(),
            source,
        })?;
        let lock: TargetLock = serde_json::from_str(&text)
            .map_err(|error| SkillctlError::Config(error.to_string()))?;
        if lock.tool != "skillctl"
            || lock.target != target
            || lock.source_root != source_root
            || lock.target_path != target_path
        {
            return Err(SkillctlError::ForeignLockOwner {
                path: path.to_path_buf(),
                expected: format!(
                    "skillctl:{target}:{}:{}",
                    source_root.display(),
                    target_path.display()
                ),
                found: format!(
                    "{}:{}:{}:{}",
                    lock.tool,
                    lock.target,
                    lock.source_root.display(),
                    lock.target_path.display()
                ),
            });
        }
        Ok(lock)
    }

    pub fn write_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| SkillctlError::Fs {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let text = serde_json::to_string_pretty(self)
            .map_err(|error| SkillctlError::Config(error.to_string()))?;
        fs::write(path, text).map_err(|source| SkillctlError::Fs {
            path: path.to_path_buf(),
            source,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_and_validates_target_owned_lockfile() {
        let temp = tempfile::tempdir().unwrap();
        let lock_path = temp.path().join(".skillctl.lock.json");
        let mut lock = TargetLock::new(
            "claude",
            temp.path().join("source"),
            temp.path().join("target"),
        );
        lock.managed.insert(
            "sample".to_string(),
            ManagedEntry {
                skill_id: "sample".to_string(),
                target_name: "sample".to_string(),
                target_path: temp.path().join("target/sample"),
                rendered_path: temp.path().join("rendered/claude/sample"),
                source_path: temp.path().join("source/skills/sample"),
                method: "symlink".to_string(),
                source_digest: "sha256:test".to_string(),
            },
        );

        lock.write_to(&lock_path).unwrap();
        let loaded = TargetLock::read_or_empty(
            &lock_path,
            "claude",
            &temp.path().join("source"),
            &temp.path().join("target"),
        )
        .unwrap();
        assert_eq!(loaded.managed["sample"].target_name, "sample");
    }

    #[test]
    fn rejects_foreign_lockfile_owner_before_mutation() {
        let temp = tempfile::tempdir().unwrap();
        let lock_path = temp.path().join(".skillctl.lock.json");
        let foreign = TargetLock::new(
            "claude",
            temp.path().join("foreign-source"),
            temp.path().join("target"),
        );
        foreign.write_to(&lock_path).unwrap();

        let error = TargetLock::read_or_empty(
            &lock_path,
            "claude",
            &temp.path().join("source"),
            &temp.path().join("target"),
        )
        .unwrap_err();
        assert!(error.to_string().contains("foreign lockfile owner"));
    }
}

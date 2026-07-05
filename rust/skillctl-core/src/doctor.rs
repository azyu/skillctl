use crate::error::SkillctlError;
use crate::lockfile::TargetLock;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetHealthInput {
    pub target: String,
    pub source_root: PathBuf,
    pub target_root: PathBuf,
    pub lock_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorReport {
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub target: String,
    pub path: PathBuf,
    pub message: String,
    pub hint: String,
}

impl DoctorReport {
    pub fn exit_code(&self) -> u8 {
        if self.diagnostics.is_empty() { 0 } else { 1 }
    }

    pub fn render(&self) -> String {
        if self.diagnostics.is_empty() {
            return "skillctl doctor: ok\n".to_string();
        }
        let mut output = String::new();
        for diagnostic in &self.diagnostics {
            output.push_str(&format!(
                "{}: {} at {}\n  hint: {}\n",
                diagnostic.target,
                diagnostic.message,
                diagnostic.path.display(),
                diagnostic.hint
            ));
        }
        output
    }
}

pub fn check(inputs: &[TargetHealthInput]) -> DoctorReport {
    let mut diagnostics = Vec::new();
    for input in inputs {
        if !input.source_root.exists() {
            diagnostics.push(Diagnostic {
                target: input.target.clone(),
                path: input.source_root.clone(),
                message: "missing source root".to_string(),
                hint: "create ~/.skillctl/skills and add canonical skill packages".to_string(),
            });
        }
        if !input.target_root.exists() {
            diagnostics.push(Diagnostic {
                target: input.target.clone(),
                path: input.target_root.clone(),
                message: "missing target root".to_string(),
                hint: "create the target directory or disable the target in config.yaml"
                    .to_string(),
            });
        }

        let lock = match TargetLock::read_or_empty(
            &input.lock_path,
            &input.target,
            &input.source_root,
            &input.target_root,
        ) {
            Ok(lock) => lock,
            Err(SkillctlError::ForeignLockOwner { found, .. }) => {
                diagnostics.push(Diagnostic {
                    target: input.target.clone(),
                    path: input.lock_path.clone(),
                    message: "foreign lockfile owner".to_string(),
                    hint: format!("remove or migrate the lockfile owned by {found}"),
                });
                continue;
            }
            Err(error) => {
                diagnostics.push(Diagnostic {
                    target: input.target.clone(),
                    path: input.lock_path.clone(),
                    message: "invalid lockfile".to_string(),
                    hint: error.to_string(),
                });
                continue;
            }
        };

        for entry in lock.managed.values() {
            check_managed_entry(&input.target, entry, &mut diagnostics);
        }
        check_unmanaged_conflicts(input, &lock, &mut diagnostics);
    }
    DoctorReport { diagnostics }
}

fn check_managed_entry(
    target: &str,
    entry: &crate::lockfile::ManagedEntry,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let metadata = match entry.target_path.symlink_metadata() {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            diagnostics.push(Diagnostic {
                target: target.to_string(),
                path: entry.target_path.clone(),
                message: "missing managed target path".to_string(),
                hint: "run skillctl apply to recreate it or skillctl prune to clean stale locks"
                    .to_string(),
            });
            return;
        }
        Err(error) => {
            diagnostics.push(Diagnostic {
                target: target.to_string(),
                path: entry.target_path.clone(),
                message: "cannot inspect managed target path".to_string(),
                hint: error.to_string(),
            });
            return;
        }
    };

    if !metadata.file_type().is_symlink() {
        diagnostics.push(Diagnostic {
            target: target.to_string(),
            path: entry.target_path.clone(),
            message: "managed target path is not a symlink".to_string(),
            hint: "remove the unmanaged path or restore the skillctl-managed symlink".to_string(),
        });
    } else {
        match fs::read_link(&entry.target_path) {
            Ok(actual) if actual == entry.rendered_path => {}
            Ok(actual) => diagnostics.push(Diagnostic {
                target: target.to_string(),
                path: entry.target_path.clone(),
                message: "managed symlink target mismatch".to_string(),
                hint: format!(
                    "expected {}, found {}",
                    entry.rendered_path.display(),
                    actual.display()
                ),
            }),
            Err(error) => diagnostics.push(Diagnostic {
                target: target.to_string(),
                path: entry.target_path.clone(),
                message: "cannot read managed symlink".to_string(),
                hint: error.to_string(),
            }),
        }
    }

    if !entry.rendered_path.exists() {
        diagnostics.push(Diagnostic {
            target: target.to_string(),
            path: entry.rendered_path.clone(),
            message: "missing rendered path".to_string(),
            hint: "run skillctl apply to rebuild rendered skill packages".to_string(),
        });
    }
}

fn check_unmanaged_conflicts(
    input: &TargetHealthInput,
    lock: &TargetLock,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !input.target_root.exists() {
        return;
    }
    let entries = match fs::read_dir(&input.target_root) {
        Ok(entries) => entries,
        Err(error) => {
            diagnostics.push(Diagnostic {
                target: input.target.clone(),
                path: input.target_root.clone(),
                message: "cannot inspect target root".to_string(),
                hint: error.to_string(),
            });
            return;
        }
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name == ".skillctl.lock.json" || lock.managed.contains_key(&name) {
            continue;
        }
        diagnostics.push(Diagnostic {
            target: input.target.clone(),
            path: entry.path(),
            message: "unmanaged target conflict".to_string(),
            hint: "move this path aside before running skillctl apply".to_string(),
        });
    }
}

#[allow(dead_code)]
fn _path_exists(path: &Path) -> bool {
    path.exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lockfile::ManagedEntry;

    #[test]
    fn reports_broken_root_lock_and_conflict_states() {
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("missing");
        let report = check(&[TargetHealthInput {
            target: "claude".to_string(),
            source_root: missing.clone(),
            target_root: temp.path().join("target"),
            lock_path: temp.path().join("target/.skillctl.lock.json"),
        }]);
        assert!(!report.diagnostics.is_empty());
        assert!(
            report.diagnostics[0]
                .message
                .contains("missing source root")
        );
        assert_eq!(report.exit_code(), 1);
        assert!(!report.render().contains("skillctl init"));
    }

    #[test]
    fn returns_clean_status_for_healthy_tree() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let target = temp.path().join("target");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::create_dir_all(&target).unwrap();
        let report = check(&[TargetHealthInput {
            target: "claude".to_string(),
            source_root: source,
            target_root: target.clone(),
            lock_path: target.join(".skillctl.lock.json"),
        }]);
        assert!(report.diagnostics.is_empty());
        assert_eq!(report.exit_code(), 0);
    }

    #[test]
    fn reports_foreign_lockfile_owner() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let target = temp.path().join("target");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(
            target.join(".skillctl.lock.json"),
            r#"{"version":1,"tool":"other","target":"claude","source_root":"/x","target_path":"/y","managed":{}}"#,
        )
        .unwrap();

        let report = check(&[TargetHealthInput {
            target: "claude".to_string(),
            source_root: source,
            target_root: target.clone(),
            lock_path: target.join(".skillctl.lock.json"),
        }]);

        assert!(report.render().contains("foreign lockfile owner"));
    }

    #[test]
    fn reports_managed_path_drift_and_missing_rendered_path() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let target = temp.path().join("target");
        let rendered = temp.path().join("rendered/sample");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("sample"), "not a symlink").unwrap();
        let mut lock = TargetLock::new("claude", source.clone(), target.clone());
        lock.managed.insert(
            "sample".to_string(),
            ManagedEntry {
                skill_id: "sample".to_string(),
                target_name: "sample".to_string(),
                target_path: target.join("sample"),
                rendered_path: rendered,
                source_path: source.join("sample"),
                method: "symlink".to_string(),
                source_digest: "sha256:test".to_string(),
            },
        );
        lock.write_to(&target.join(".skillctl.lock.json")).unwrap();

        let report = check(&[TargetHealthInput {
            target: "claude".to_string(),
            source_root: source,
            target_root: target.clone(),
            lock_path: target.join(".skillctl.lock.json"),
        }]);
        let rendered = report.render();

        assert!(rendered.contains("managed target path is not a symlink"));
        assert!(rendered.contains("missing rendered path"));
    }

    #[test]
    fn reports_unmanaged_target_conflicts() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let target = temp.path().join("target");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("foreign"), "foreign").unwrap();

        let report = check(&[TargetHealthInput {
            target: "claude".to_string(),
            source_root: source,
            target_root: target.clone(),
            lock_path: target.join(".skillctl.lock.json"),
        }]);

        assert!(report.render().contains("unmanaged target conflict"));
    }
}

use crate::error::SkillctlError;
use crate::lockfile::TargetLock;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetHealthInput {
    pub target: String,
    pub source_root: PathBuf,
    pub source_root_required: bool,
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
        if input.source_root_required && !input.source_root.exists() {
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

pub fn check_sources(
    root: &Path,
    config: &crate::config::Config,
    source_lock: &crate::sources::SourceLock,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (source_id, source) in &config.sources {
        let checkout = root.join("repos").join(source_id);
        let Some(entry) = source_lock.sources.get(source_id) else {
            diagnostics.push(Diagnostic {
                target: "sources".to_string(),
                path: root.join("source-lock.json"),
                message: format!("missing source lock entry for {source_id}"),
                hint: "run skillctl update".to_string(),
            });
            continue;
        };

        if entry.kind != "git"
            || entry.repo != source.repo
            || entry.ref_name != source.ref_name
            || entry.path != source.path
        {
            diagnostics.push(Diagnostic {
                target: "sources".to_string(),
                path: root.join("source-lock.json"),
                message: format!("stale source lock entry for {source_id}"),
                hint: "run skillctl update".to_string(),
            });
        }

        if !checkout.exists() {
            diagnostics.push(Diagnostic {
                target: "sources".to_string(),
                path: checkout,
                message: format!("missing checkout for source {source_id}"),
                hint: "run skillctl update".to_string(),
            });
            continue;
        }

        match checkout_head(&checkout) {
            Ok(head) if head != entry.commit => diagnostics.push(Diagnostic {
                target: "sources".to_string(),
                path: checkout.clone(),
                message: format!("checkout HEAD mismatch for source {source_id}"),
                hint: format!("expected {}, found {}", entry.commit, head),
            }),
            Ok(_) => {}
            Err(error) => diagnostics.push(Diagnostic {
                target: "sources".to_string(),
                path: checkout.clone(),
                message: format!("cannot inspect checkout HEAD for source {source_id}"),
                hint: error,
            }),
        }

        for (skill_id, skill) in config
            .skills
            .iter()
            .filter(|(_, skill)| skill.source.as_deref() == Some(source_id.as_str()))
        {
            let skill_path = checkout.join(&source.path).join(&skill.path);
            if !skill_path.exists() {
                diagnostics.push(Diagnostic {
                    target: "sources".to_string(),
                    path: skill_path,
                    message: format!("missing configured skill path for {skill_id}"),
                    hint: "run skillctl update".to_string(),
                });
            }
        }
    }
}
pub fn check_target_provenance(
    target: &str,
    lock_path: &Path,
    lock: &TargetLock,
    config: &crate::config::Config,
    source_lock: &crate::sources::SourceLock,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for entry in lock.managed.values() {
        let Some(source) = entry.source.as_ref() else {
            continue;
        };
        let Some(source_config) = config.sources.get(&source.id) else {
            diagnostics.push(Diagnostic {
                target: target.to_string(),
                path: lock_path.to_path_buf(),
                message: format!(
                    "missing configured source for target provenance {}",
                    source.id
                ),
                hint: "restore the source config or run skillctl prune".to_string(),
            });
            continue;
        };
        let Some(source_entry) = source_lock.sources.get(&source.id) else {
            diagnostics.push(Diagnostic {
                target: target.to_string(),
                path: lock_path.to_path_buf(),
                message: format!(
                    "missing source lock entry for target provenance {}",
                    source.id
                ),
                hint: "run skillctl update, then skillctl apply".to_string(),
            });
            continue;
        };

        let expected_kind = match &source_config.kind {
            crate::config::SourceKind::Git => "git",
        };
        if source.kind != expected_kind
            || source.repo != source_config.repo
            || source.repo != source_entry.repo
            || source.ref_name != source_config.ref_name
            || source.ref_name != source_entry.ref_name
            || source.commit != source_entry.commit
            || source_entry.kind != expected_kind
            || source_entry.path != source_config.path
        {
            diagnostics.push(Diagnostic {
                target: target.to_string(),
                path: lock_path.to_path_buf(),
                message: format!(
                    "stale or mismatched target source provenance for {}",
                    entry.target_name
                ),
                hint: "run skillctl update, then skillctl apply".to_string(),
            });
        }
    }
}

fn checkout_head(checkout: &Path) -> std::result::Result<String, String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(checkout)
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{SourceConfig, SourceKind};
    use crate::lockfile::ManagedEntry;
    use crate::sources::{SourceLock, SourceLockEntry, SourceProvenance};
    use std::path::Path;

    #[test]
    fn reports_broken_root_lock_and_conflict_states() {
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("missing");
        let report = check(&[TargetHealthInput {
            target: "claude".to_string(),
            source_root: missing.clone(),
            source_root_required: true,
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
    fn skips_missing_source_root_for_git_only_sources() {
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("missing");
        let target = temp.path().join("target");
        std::fs::create_dir_all(&target).unwrap();
        let report = check(&[TargetHealthInput {
            target: "claude".to_string(),
            source_root: missing,
            source_root_required: false,
            target_root: target.clone(),
            lock_path: target.join(".skillctl.lock.json"),
        }]);
        assert!(report.diagnostics.is_empty());
        assert_eq!(report.exit_code(), 0);
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
            source_root_required: true,
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
            source_root_required: true,
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
                source: None,
            },
        );
        lock.write_to(&target.join(".skillctl.lock.json")).unwrap();

        let report = check(&[TargetHealthInput {
            target: "claude".to_string(),
            source_root: source,
            source_root_required: true,
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
            source_root_required: true,
            target_root: target.clone(),
            lock_path: target.join(".skillctl.lock.json"),
        }]);

        assert!(report.render().contains("unmanaged target conflict"));
    }

    #[test]
    fn reports_stale_target_source_provenance_when_source_commit_changes() {
        let temp = tempfile::tempdir().unwrap();
        let lock_path = temp.path().join(".skillctl.lock.json");
        let mut config = crate::config::Config::default_for_home(temp.path());
        config.sources.insert(
            "shared".to_string(),
            SourceConfig {
                kind: SourceKind::Git,
                repo: "file:///tmp/shared.git".to_string(),
                ref_name: "main".to_string(),
                path: std::path::PathBuf::from("skills"),
            },
        );

        let mut source_lock = SourceLock::default();
        source_lock.sources.insert(
            "shared".to_string(),
            SourceLockEntry {
                kind: "git".to_string(),
                repo: "file:///tmp/shared.git".to_string(),
                ref_name: "main".to_string(),
                commit: "new".to_string(),
                path: std::path::PathBuf::from("skills"),
            },
        );

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
                source_path: temp.path().join("repos/shared/skills/sample"),
                method: "symlink".to_string(),
                source_digest: "sha256:test".to_string(),
                source: Some(SourceProvenance {
                    kind: "git".to_string(),
                    id: "shared".to_string(),
                    repo: "file:///tmp/shared.git".to_string(),
                    ref_name: "main".to_string(),
                    commit: "old".to_string(),
                }),
            },
        );

        let mut diagnostics = Vec::new();
        check_target_provenance(
            "claude",
            &lock_path,
            &lock,
            &config,
            &source_lock,
            &mut diagnostics,
        );

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("stale or mismatched target source provenance for sample")
        }));
    }

    #[test]
    fn reports_stale_target_source_provenance_when_source_repo_or_ref_changes_without_commit_change()
     {
        let temp = tempfile::tempdir().unwrap();
        let lock_path = temp.path().join(".skillctl.lock.json");
        let mut config = crate::config::Config::default_for_home(temp.path());
        config.sources.insert(
            "shared".to_string(),
            SourceConfig {
                kind: SourceKind::Git,
                repo: "file:///tmp/current.git".to_string(),
                ref_name: "main".to_string(),
                path: std::path::PathBuf::from("skills"),
            },
        );

        let mut source_lock = SourceLock::default();
        source_lock.sources.insert(
            "shared".to_string(),
            SourceLockEntry {
                kind: "git".to_string(),
                repo: "file:///tmp/current.git".to_string(),
                ref_name: "main".to_string(),
                commit: "same".to_string(),
                path: std::path::PathBuf::from("skills"),
            },
        );

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
                source_path: temp.path().join("repos/shared/skills/sample"),
                method: "symlink".to_string(),
                source_digest: "sha256:test".to_string(),
                source: Some(SourceProvenance {
                    kind: "git".to_string(),
                    id: "shared".to_string(),
                    repo: "file:///tmp/old.git".to_string(),
                    ref_name: "release".to_string(),
                    commit: "same".to_string(),
                }),
            },
        );

        let mut diagnostics = Vec::new();
        check_target_provenance(
            "claude",
            &lock_path,
            &lock,
            &config,
            &source_lock,
            &mut diagnostics,
        );

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("stale or mismatched target source provenance for sample")
        }));
    }

    #[test]
    fn reports_missing_source_lock_entry_for_target_provenance() {
        let temp = tempfile::tempdir().unwrap();
        let lock_path = temp.path().join(".skillctl.lock.json");
        let mut config = crate::config::Config::default_for_home(temp.path());
        config.sources.insert(
            "shared".to_string(),
            SourceConfig {
                kind: SourceKind::Git,
                repo: "file:///tmp/shared.git".to_string(),
                ref_name: "main".to_string(),
                path: std::path::PathBuf::from("skills"),
            },
        );

        let source_lock = SourceLock::default();
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
                source_path: temp.path().join("repos/shared/skills/sample"),
                method: "symlink".to_string(),
                source_digest: "sha256:test".to_string(),
                source: Some(SourceProvenance {
                    kind: "git".to_string(),
                    id: "shared".to_string(),
                    repo: "file:///tmp/shared.git".to_string(),
                    ref_name: "main".to_string(),
                    commit: "old".to_string(),
                }),
            },
        );

        let mut diagnostics = Vec::new();
        check_target_provenance(
            "claude",
            &lock_path,
            &lock,
            &config,
            &source_lock,
            &mut diagnostics,
        );

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("missing source lock entry for target provenance shared")
        }));
    }

    #[test]
    fn reports_missing_git_source_lock_entry() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join(".skillctl");
        let mut config = crate::config::Config::default_for_home(temp.path());
        config.sources.insert(
            "shared".to_string(),
            crate::config::SourceConfig {
                kind: crate::config::SourceKind::Git,
                repo: "file:///tmp/shared.git".to_string(),
                ref_name: "main".to_string(),
                path: std::path::PathBuf::from("skills"),
            },
        );
        let lock = crate::sources::SourceLock::default();

        let mut diagnostics = Vec::new();
        check_sources(&root, &config, &lock, &mut diagnostics);

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("missing source lock entry for shared")
        }));
    }

    #[test]
    fn reports_missing_git_checkout() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join(".skillctl");
        let mut config = crate::config::Config::default_for_home(temp.path());
        config.sources.insert(
            "shared".to_string(),
            crate::config::SourceConfig {
                kind: crate::config::SourceKind::Git,
                repo: "file:///tmp/shared.git".to_string(),
                ref_name: "main".to_string(),
                path: std::path::PathBuf::from("skills"),
            },
        );
        let mut lock = crate::sources::SourceLock {
            version: 1,
            sources: Default::default(),
        };
        lock.sources.insert(
            "shared".to_string(),
            crate::sources::SourceLockEntry {
                kind: "git".to_string(),
                repo: "file:///tmp/shared.git".to_string(),
                ref_name: "main".to_string(),
                commit: "abc123".to_string(),
                path: std::path::PathBuf::from("skills"),
            },
        );

        let mut diagnostics = Vec::new();
        check_sources(&root, &config, &lock, &mut diagnostics);

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic
                .message
                .contains("missing checkout for source shared")
        }));
    }
    #[test]
    fn reports_checkout_head_mismatch_against_source_lock_commit() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join(".skillctl");
        let checkout = root.join("repos/shared");
        let initial_commit = init_checkout_repo(&checkout, "recap", "initial skill text");
        run_git(&checkout, &["commit", "--allow-empty", "-m", "advance"]);
        let advanced_commit = run_git_stdout(&checkout, &["rev-parse", "HEAD"]);
        assert_ne!(initial_commit, advanced_commit);

        let mut config = crate::config::Config::default_for_home(temp.path());
        config.sources.insert(
            "shared".to_string(),
            crate::config::SourceConfig {
                kind: crate::config::SourceKind::Git,
                repo: "file:///tmp/shared.git".to_string(),
                ref_name: "main".to_string(),
                path: std::path::PathBuf::from("skills"),
            },
        );
        config.skills.insert(
            "recap".to_string(),
            crate::config::SkillConfig {
                source: Some("shared".to_string()),
                path: std::path::PathBuf::from("recap"),
                expose: vec!["claude".to_string()],
            },
        );

        let mut lock = crate::sources::SourceLock::default();
        lock.sources.insert(
            "shared".to_string(),
            crate::sources::SourceLockEntry {
                kind: "git".to_string(),
                repo: "file:///tmp/shared.git".to_string(),
                ref_name: "main".to_string(),
                commit: initial_commit,
                path: std::path::PathBuf::from("skills"),
            },
        );

        let mut diagnostics = Vec::new();
        check_sources(&root, &config, &lock, &mut diagnostics);

        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("checkout HEAD mismatch"))
        );
    }

    #[test]
    fn reports_missing_configured_skill_path_inside_existing_checkout() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join(".skillctl");
        let checkout = root.join("repos/shared");
        let commit = init_checkout_repo(&checkout, "other", "initial skill text");

        let mut config = crate::config::Config::default_for_home(temp.path());
        config.sources.insert(
            "shared".to_string(),
            crate::config::SourceConfig {
                kind: crate::config::SourceKind::Git,
                repo: "file:///tmp/shared.git".to_string(),
                ref_name: "main".to_string(),
                path: std::path::PathBuf::from("skills"),
            },
        );
        config.skills.insert(
            "recap".to_string(),
            crate::config::SkillConfig {
                source: Some("shared".to_string()),
                path: std::path::PathBuf::from("recap"),
                expose: vec!["claude".to_string()],
            },
        );

        let mut lock = crate::sources::SourceLock::default();
        lock.sources.insert(
            "shared".to_string(),
            crate::sources::SourceLockEntry {
                kind: "git".to_string(),
                repo: "file:///tmp/shared.git".to_string(),
                ref_name: "main".to_string(),
                commit,
                path: std::path::PathBuf::from("skills"),
            },
        );

        let mut diagnostics = Vec::new();
        check_sources(&root, &config, &lock, &mut diagnostics);

        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| { diagnostic.message.contains("missing configured skill path") })
        );
    }

    fn init_checkout_repo(checkout: &Path, skill_name: &str, body: &str) -> String {
        std::fs::create_dir_all(checkout.join(format!("skills/{skill_name}"))).unwrap();
        run_git(checkout, &["init"]);
        run_git(checkout, &["config", "user.email", "test@example.com"]);
        run_git(checkout, &["config", "user.name", "Test User"]);
        std::fs::write(
            checkout.join(format!("skills/{skill_name}/SKILL.md")),
            format!("---\nname: {skill_name}\ndescription: test skill\n---\n\n{body}\n"),
        )
        .unwrap();
        run_git(checkout, &["add", "."]);
        run_git(checkout, &["commit", "-m", "initial"]);
        run_git_stdout(checkout, &["rev-parse", "HEAD"])
    }

    fn run_git(cwd: &Path, args: &[&str]) {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(cwd)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
            args,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn run_git_stdout(cwd: &Path, args: &[&str]) -> String {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(cwd)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
            args,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    }
}

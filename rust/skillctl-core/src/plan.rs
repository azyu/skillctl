use crate::error::{Result, SkillctlError};
use crate::lockfile::{ManagedEntry, TargetLock};
use std::collections::BTreeSet;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesiredLink {
    pub skill_id: String,
    pub target_name: String,
    pub target_path: PathBuf,
    pub rendered_path: PathBuf,
    pub source_path: PathBuf,
    pub source_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub operations: Vec<PlanOperation>,
    pub errors: Vec<String>,
}

impl Plan {
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanOperation {
    Link {
        skill_id: String,
        target_name: String,
        target_path: PathBuf,
        rendered_path: PathBuf,
        source_path: PathBuf,
        source_digest: String,
        previous: Option<ManagedEntry>,
    },
    RemoveStale {
        target_name: String,
        target_path: PathBuf,
        expected_rendered_path: PathBuf,
    },
}

impl PlanOperation {
    pub fn label(&self) -> &'static str {
        match self {
            PlanOperation::Link { previous, .. } => {
                if previous.is_some() {
                    "UPDATE"
                } else {
                    "CREATE"
                }
            }
            PlanOperation::RemoveStale { .. } => "REMOVE_STALE",
        }
    }

    pub fn target_name(&self) -> &str {
        match self {
            PlanOperation::Link { target_name, .. }
            | PlanOperation::RemoveStale { target_name, .. } => target_name,
        }
    }

    pub fn target_path(&self) -> &Path {
        match self {
            PlanOperation::Link { target_path, .. }
            | PlanOperation::RemoveStale { target_path, .. } => target_path,
        }
    }
}

pub fn build_plan(
    _target_root: &Path,
    lock: &TargetLock,
    desired: Vec<DesiredLink>,
) -> Result<Plan> {
    let mut operations = Vec::new();
    let mut errors = Vec::new();
    let mut desired_names = BTreeSet::new();

    for item in desired {
        desired_names.insert(item.target_name.clone());
        match lock.managed.get(&item.target_name) {
            Some(existing) => {
                let ownership = ownership_state(&item.target_path, &existing.rendered_path)?;
                match &ownership {
                    OwnershipState::Drifted(reason) => errors.push(format!(
                        "managed target drift at {}: {reason}",
                        item.target_path.display()
                    )),
                    OwnershipState::Missing | OwnershipState::Matches => {}
                }

                let needs_link = existing.rendered_path != item.rendered_path
                    || existing.source_digest != item.source_digest
                    || matches!(ownership, OwnershipState::Missing);
                if needs_link && !matches!(ownership, OwnershipState::Drifted(_)) {
                    operations.push(PlanOperation::Link {
                        skill_id: item.skill_id,
                        target_name: item.target_name,
                        target_path: item.target_path,
                        rendered_path: item.rendered_path,
                        source_path: item.source_path,
                        source_digest: item.source_digest,
                        previous: Some(existing.clone()),
                    });
                }
            }
            None => {
                if item.target_path.symlink_metadata().is_ok() {
                    errors.push(format!(
                        "unmanaged conflict at {}",
                        item.target_path.display()
                    ));
                } else {
                    operations.push(PlanOperation::Link {
                        skill_id: item.skill_id,
                        target_name: item.target_name,
                        target_path: item.target_path,
                        rendered_path: item.rendered_path,
                        source_path: item.source_path,
                        source_digest: item.source_digest,
                        previous: None,
                    });
                }
            }
        }
    }

    for (target_name, entry) in &lock.managed {
        if desired_names.contains(target_name) {
            continue;
        }
        match ownership_state(&entry.target_path, &entry.rendered_path)? {
            OwnershipState::Drifted(reason) => errors.push(format!(
                "managed target drift at {}: {reason}",
                entry.target_path.display()
            )),
            OwnershipState::Missing | OwnershipState::Matches => {
                operations.push(PlanOperation::RemoveStale {
                    target_name: target_name.clone(),
                    target_path: entry.target_path.clone(),
                    expected_rendered_path: entry.rendered_path.clone(),
                });
            }
        }
    }

    errors.sort();
    errors.dedup();
    Ok(Plan { operations, errors })
}

pub fn build_plan_rejecting_unmanaged_entries(
    target_root: &Path,
    lock: &TargetLock,
    desired: Vec<DesiredLink>,
) -> Result<Plan> {
    let desired_names = desired
        .iter()
        .map(|item| item.target_name.clone())
        .collect::<BTreeSet<_>>();
    let mut plan = build_plan(target_root, lock, desired)?;
    plan.errors
        .extend(unmanaged_conflicts(target_root, lock, &desired_names)?);
    plan.errors.sort();
    plan.errors.dedup();
    Ok(plan)
}

fn unmanaged_conflicts(
    target_root: &Path,
    lock: &TargetLock,
    desired_names: &BTreeSet<String>,
) -> Result<Vec<String>> {
    if !target_root.exists() {
        return Ok(Vec::new());
    }
    let mut errors = Vec::new();
    for entry in fs::read_dir(target_root).map_err(|source| SkillctlError::Fs {
        path: target_root.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| SkillctlError::Fs {
            path: target_root.to_path_buf(),
            source,
        })?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name == ".skillctl.lock.json"
            || desired_names.contains(&name)
            || lock.managed.contains_key(&name)
        {
            continue;
        }
        errors.push(format!("unmanaged conflict at {}", entry.path().display()));
    }
    errors.sort();
    Ok(errors)
}

pub fn apply_plan(plan: &Plan) -> Result<()> {
    if let Some(error) = plan.errors.first() {
        return Err(SkillctlError::Config(error.clone()));
    }
    for operation in &plan.operations {
        match operation {
            PlanOperation::Link {
                target_path,
                rendered_path,
                previous,
                ..
            } => {
                if let Some(previous) = previous {
                    ensure_owned_or_missing(target_path, &previous.rendered_path)?;
                } else if target_path.symlink_metadata().is_ok() {
                    return Err(SkillctlError::UnmanagedConflict(target_path.clone()));
                }
                crate::fs::create_symlink_dir(rendered_path, target_path)?;
            }
            PlanOperation::RemoveStale {
                target_path,
                expected_rendered_path,
                ..
            } => {
                remove_owned_symlink_or_missing(target_path, expected_rendered_path)?;
            }
        }
    }
    Ok(())
}

pub fn validate_remove_stale_ownership(
    target_path: &Path,
    expected_rendered_path: &Path,
) -> Result<()> {
    ensure_owned_or_missing(target_path, expected_rendered_path)
}

fn ensure_owned_or_missing(target_path: &Path, expected_rendered_path: &Path) -> Result<()> {
    match ownership_state(target_path, expected_rendered_path)? {
        OwnershipState::Matches | OwnershipState::Missing => Ok(()),
        OwnershipState::Drifted(reason) => Err(SkillctlError::Config(format!(
            "managed target drift at {}: {reason}",
            target_path.display()
        ))),
    }
}

fn remove_owned_symlink_or_missing(
    target_path: &Path,
    expected_rendered_path: &Path,
) -> Result<()> {
    match ownership_state(target_path, expected_rendered_path)? {
        OwnershipState::Missing => Ok(()),
        OwnershipState::Matches => {
            fs::remove_file(target_path).map_err(|source| SkillctlError::Fs {
                path: target_path.to_path_buf(),
                source,
            })
        }
        OwnershipState::Drifted(reason) => Err(SkillctlError::Config(format!(
            "managed target drift at {}: {reason}",
            target_path.display()
        ))),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum OwnershipState {
    Missing,
    Matches,
    Drifted(String),
}

fn ownership_state(target_path: &Path, expected_rendered_path: &Path) -> Result<OwnershipState> {
    let metadata = match target_path.symlink_metadata() {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(OwnershipState::Missing),
        Err(source) => {
            return Err(SkillctlError::Fs {
                path: target_path.to_path_buf(),
                source,
            });
        }
    };
    if !metadata.file_type().is_symlink() {
        return Ok(OwnershipState::Drifted("path is not a symlink".to_string()));
    }
    let actual = fs::read_link(target_path).map_err(|source| SkillctlError::Fs {
        path: target_path.to_path_buf(),
        source,
    })?;
    if actual == expected_rendered_path {
        Ok(OwnershipState::Matches)
    } else {
        Ok(OwnershipState::Drifted(format!(
            "expected symlink to {}, found {}",
            expected_rendered_path.display(),
            actual.display()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lockfile::{ManagedEntry, TargetLock};
    use std::fs;

    #[test]
    fn plan_ignores_unrelated_unmanaged_target_entries() {
        let temp = tempfile::tempdir().unwrap();
        let target_root = temp.path().join("target");
        let rendered_root = temp.path().join("rendered");
        fs::create_dir_all(&target_root).unwrap();
        fs::create_dir_all(&rendered_root).unwrap();
        std::os::unix::fs::symlink(rendered_root.join("sample-old"), target_root.join("sample"))
            .unwrap();
        fs::write(target_root.join("other"), "unmanaged").unwrap();

        let mut lock = TargetLock::new("claude", temp.path().join("source"), target_root.clone());
        lock.managed.insert(
            "sample".to_string(),
            ManagedEntry {
                skill_id: "sample".to_string(),
                target_name: "sample".to_string(),
                target_path: target_root.join("sample"),
                rendered_path: rendered_root.join("sample-old"),
                source_path: temp.path().join("source/skills/sample"),
                method: "symlink".to_string(),
                source_digest: "sha256:old".to_string(),
            },
        );

        let desired = vec![DesiredLink {
            skill_id: "sample".to_string(),
            target_name: "sample".to_string(),
            target_path: target_root.join("sample"),
            rendered_path: rendered_root.join("sample"),
            source_path: temp.path().join("source/skills/sample"),
            source_digest: "sha256:new".to_string(),
        }];
        let plan = build_plan(&target_root, &lock, desired).unwrap();
        assert_eq!(plan.operations.len(), 1);
        assert_eq!(plan.operations[0].label(), "UPDATE");
        assert!(plan.errors.is_empty());
    }

    #[test]
    fn compares_source_digest_when_planning_managed_updates() {
        let temp = tempfile::tempdir().unwrap();
        let target_root = temp.path().join("target");
        let rendered = temp.path().join("rendered/sample");
        fs::create_dir_all(&target_root).unwrap();
        std::os::unix::fs::symlink(&rendered, target_root.join("sample")).unwrap();
        let mut lock = TargetLock::new("claude", temp.path().join("source"), target_root.clone());
        lock.managed.insert(
            "sample".to_string(),
            managed_entry(&target_root, &rendered, "sha256:old"),
        );

        let desired = vec![DesiredLink {
            skill_id: "sample".to_string(),
            target_name: "sample".to_string(),
            target_path: target_root.join("sample"),
            rendered_path: rendered.clone(),
            source_path: temp.path().join("source/skills/sample"),
            source_digest: "sha256:new".to_string(),
        }];

        let plan = build_plan(&target_root, &lock, desired).unwrap();

        assert_eq!(plan.operations.len(), 1);
        assert_eq!(plan.operations[0].label(), "UPDATE");
    }

    #[test]
    fn refuses_to_replace_drifted_managed_target() {
        let temp = tempfile::tempdir().unwrap();
        let target_root = temp.path().join("target");
        let rendered = temp.path().join("rendered/sample");
        fs::create_dir_all(&target_root).unwrap();
        fs::write(target_root.join("sample"), "not a symlink").unwrap();
        let mut lock = TargetLock::new("claude", temp.path().join("source"), target_root.clone());
        lock.managed.insert(
            "sample".to_string(),
            managed_entry(&target_root, &rendered, "sha256:old"),
        );
        let desired = vec![DesiredLink {
            skill_id: "sample".to_string(),
            target_name: "sample".to_string(),
            target_path: target_root.join("sample"),
            rendered_path: rendered,
            source_path: temp.path().join("source/skills/sample"),
            source_digest: "sha256:new".to_string(),
        }];

        let plan = build_plan(&target_root, &lock, desired).unwrap();

        assert!(plan.operations.is_empty());
        assert!(plan.errors.iter().any(|error| error.contains("drift")));
    }

    #[test]
    fn remove_stale_carries_expected_rendered_path_and_refuses_drift() {
        let temp = tempfile::tempdir().unwrap();
        let target_root = temp.path().join("target");
        let rendered = temp.path().join("rendered/sample");
        fs::create_dir_all(&target_root).unwrap();
        std::os::unix::fs::symlink(&rendered, target_root.join("sample")).unwrap();
        let mut lock = TargetLock::new("claude", temp.path().join("source"), target_root.clone());
        lock.managed.insert(
            "sample".to_string(),
            managed_entry(&target_root, &rendered, "sha256:old"),
        );

        let plan = build_plan(&target_root, &lock, Vec::new()).unwrap();

        assert_eq!(plan.operations.len(), 1);
        assert_eq!(plan.operations[0].label(), "REMOVE_STALE");
        match &plan.operations[0] {
            PlanOperation::RemoveStale {
                expected_rendered_path,
                ..
            } => assert_eq!(expected_rendered_path, &rendered),
            _ => panic!("expected stale removal"),
        }
    }

    #[test]
    fn apply_aborts_on_unmanaged_conflict_without_mutation() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("target/sample");
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(&target, "original").unwrap();
        let plan = Plan {
            operations: vec![PlanOperation::Link {
                skill_id: "sample".to_string(),
                target_name: "sample".to_string(),
                target_path: target.clone(),
                rendered_path: temp.path().join("rendered/sample"),
                source_path: temp.path().join("source/skills/sample"),
                source_digest: "sha256:test".to_string(),
                previous: None,
            }],
            errors: vec!["unmanaged conflict at target/sample".to_string()],
        };

        let error = apply_plan(&plan).unwrap_err();
        assert!(error.to_string().contains("unmanaged conflict"));
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "original");
    }

    fn managed_entry(target_root: &Path, rendered: &Path, source_digest: &str) -> ManagedEntry {
        ManagedEntry {
            skill_id: "sample".to_string(),
            target_name: "sample".to_string(),
            target_path: target_root.join("sample"),
            rendered_path: rendered.to_path_buf(),
            source_path: target_root.join("../source/skills/sample"),
            method: "symlink".to_string(),
            source_digest: source_digest.to_string(),
        }
    }
}

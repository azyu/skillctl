use crate::error::{Result, SkillctlError};
use crate::lockfile::TargetLock;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesiredLink {
    pub skill_id: String,
    pub target_name: String,
    pub target_path: PathBuf,
    pub rendered_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub operations: Vec<PlanOperation>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanOperation {
    Link {
        skill_id: String,
        target_path: PathBuf,
        rendered_path: PathBuf,
    },
    RemoveStale {
        target_name: String,
        target_path: PathBuf,
    },
}

pub fn build_plan(
    target_root: &Path,
    lock: &TargetLock,
    desired: Vec<DesiredLink>,
) -> Result<Plan> {
    let mut operations = Vec::new();
    let mut desired_names = BTreeSet::new();
    for item in desired {
        desired_names.insert(item.target_name.clone());
        let needs_link = match lock.managed.get(&item.target_name) {
            Some(existing) => existing.rendered_path != item.rendered_path,
            None => true,
        };
        if needs_link {
            operations.push(PlanOperation::Link {
                skill_id: item.skill_id,
                target_path: item.target_path,
                rendered_path: item.rendered_path,
            });
        }
    }
    for (target_name, entry) in &lock.managed {
        if !desired_names.contains(target_name) {
            operations.push(PlanOperation::RemoveStale {
                target_name: target_name.clone(),
                target_path: entry.target_path.clone(),
            });
        }
    }
    let errors = unmanaged_conflicts(target_root, lock, &desired_names)?;
    Ok(Plan { operations, errors })
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
                ..
            } => {
                crate::fs::create_symlink_dir(rendered_path, target_path)?;
            }
            PlanOperation::RemoveStale { target_path, .. } => {
                if target_path.exists() || target_path.symlink_metadata().is_ok() {
                    std::fs::remove_file(target_path)
                        .or_else(|_| std::fs::remove_dir_all(target_path))
                        .map_err(|source| SkillctlError::Fs {
                            path: target_path.clone(),
                            source,
                        })?;
                }
            }
        }
    }
    Ok(())
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
        if name == ".skillctl.lock.json" {
            continue;
        }
        if desired_names.contains(&name) || lock.managed.contains_key(&name) {
            continue;
        }
        errors.push(format!("unmanaged conflict at {}", entry.path().display()));
    }
    errors.sort();
    Ok(errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lockfile::{ManagedEntry, TargetLock};
    use std::fs;

    #[test]
    fn plan_distinguishes_managed_updates_from_unmanaged_conflicts() {
        let temp = tempfile::tempdir().unwrap();
        let target_root = temp.path().join("target");
        let rendered_root = temp.path().join("rendered");
        fs::create_dir_all(&target_root).unwrap();
        fs::create_dir_all(&rendered_root).unwrap();
        fs::write(target_root.join("sample"), "managed old target placeholder").unwrap();
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
        }];
        let plan = build_plan(&target_root, &lock, desired).unwrap();
        assert_eq!(plan.operations.len(), 1);
        assert!(plan.errors.iter().all(|error| !error.contains("sample")));
        assert!(plan.errors.iter().any(|error| error.contains("other")));
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
                target_path: target.clone(),
                rendered_path: temp.path().join("rendered/sample"),
            }],
            errors: vec!["unmanaged conflict at target/sample".to_string()],
        };

        let error = apply_plan(&plan).unwrap_err();
        assert!(error.to_string().contains("unmanaged conflict"));
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "original");
    }
}

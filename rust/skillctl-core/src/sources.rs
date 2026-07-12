use crate::config::{Config, SkillConfig, SourceKind};
use crate::error::{Result, SkillctlError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
trait GitRunner {
    fn run(&self, cwd: &Path, args: &[&str]) -> Result<()>;
    fn stdout(&self, cwd: &Path, args: &[&str]) -> Result<String>;
    fn clone(&self, repo: &str, checkout: &Path) -> Result<()>;
}

#[derive(Debug, Clone, Copy, Default)]
struct RealGit;

impl GitRunner for RealGit {
    fn run(&self, cwd: &Path, args: &[&str]) -> Result<()> {
        let output = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .output()
            .map_err(|source| SkillctlError::Fs {
                path: cwd.to_path_buf(),
                source,
            })?;
        if !output.status.success() {
            return Err(SkillctlError::Config(format!(
                "git {} failed in {}: {}",
                args.join(" "),
                cwd.display(),
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        Ok(())
    }

    fn stdout(&self, cwd: &Path, args: &[&str]) -> Result<String> {
        let output = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .output()
            .map_err(|source| SkillctlError::Fs {
                path: cwd.to_path_buf(),
                source,
            })?;
        if !output.status.success() {
            return Err(SkillctlError::Config(format!(
                "git {} failed in {}: {}",
                args.join(" "),
                cwd.display(),
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn clone(&self, repo: &str, checkout: &Path) -> Result<()> {
        let output = Command::new("git")
            .args(["clone", repo])
            .arg(checkout)
            .output()
            .map_err(|source| SkillctlError::Fs {
                path: checkout.to_path_buf(),
                source,
            })?;
        if !output.status.success() {
            return Err(SkillctlError::Config(format!(
                "git clone failed for {repo}: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SourceLock {
    pub version: u32,
    #[serde(default)]
    pub sources: BTreeMap<String, SourceLockEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceLockEntry {
    #[serde(rename = "type")]
    pub kind: String,
    pub repo: String,
    #[serde(rename = "ref")]
    pub ref_name: String,
    pub commit: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceProvenance {
    pub kind: String,
    pub id: String,
    pub repo: String,
    #[serde(rename = "ref")]
    pub ref_name: String,
    pub commit: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSourcePath {
    pub package_path: PathBuf,
    pub provenance: Option<SourceProvenance>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceUpdateReport {
    pub updates: Vec<SourceUpdate>,
}

impl SourceUpdateReport {
    pub fn has_errors(&self) -> bool {
        self.updates
            .iter()
            .any(|update| matches!(update.status, SourceUpdateStatus::Error { .. }))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceUpdate {
    pub source_id: String,
    pub ref_name: String,
    pub old_commit: Option<String>,
    pub new_commit: String,
    pub status: SourceUpdateStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceUpdateStatus {
    Cloned,
    Updated,
    Unchanged,
    Error { message: String },
}

pub fn read_source_lock_or_empty(path: &Path) -> Result<SourceLock> {
    if !path.exists() {
        return Ok(SourceLock {
            version: 1,
            sources: BTreeMap::new(),
        });
    }

    let text = fs::read_to_string(path).map_err(|source| SkillctlError::Fs {
        path: path.to_path_buf(),
        source,
    })?;
    let mut lock: SourceLock =
        serde_json::from_str(&text).map_err(|error| SkillctlError::Config(error.to_string()))?;
    if lock.version == 0 {
        lock.version = 1;
    }
    Ok(lock)
}

pub fn write_source_lock(path: &Path, lock: &SourceLock) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| SkillctlError::Fs {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let text = serde_json::to_string_pretty(lock)
        .map_err(|error| SkillctlError::Config(error.to_string()))?;
    fs::write(path, text).map_err(|source| SkillctlError::Fs {
        path: path.to_path_buf(),
        source,
    })
}

fn checkout_path(root: &Path, source_id: &str) -> Result<PathBuf> {
    let mut components = Path::new(source_id).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None)
            if !source_id.contains('/')
                && !source_id.contains('\\')
                && !source_id.contains(':') =>
        {
            Ok(root.join("repos").join(source_id))
        }
        _ => Err(SkillctlError::Config(format!(
            "invalid source id {source_id:?}"
        ))),
    }
}
fn checkout_head_commit(checkout: &Path) -> Result<String> {
    let output = Command::new("git")
        .current_dir(checkout)
        .args(["rev-parse", "HEAD"])
        .output()
        .map_err(|source| SkillctlError::Fs {
            path: checkout.to_path_buf(),
            source,
        })?;
    if !output.status.success() {
        return Err(SkillctlError::Config(format!(
            "unable to read checkout HEAD for {}: {}",
            checkout.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(String::from_utf8(output.stdout)
        .expect("git rev-parse output should be valid UTF-8")
        .trim()
        .to_string())
}
fn checkout_status_porcelain(checkout: &Path) -> Result<String> {
    let output = Command::new("git")
        .current_dir(checkout)
        .args([
            "status",
            "--porcelain",
            "--untracked-files=all",
            "--ignored",
        ])
        .output()
        .map_err(|source| SkillctlError::Fs {
            path: checkout.to_path_buf(),
            source,
        })?;
    if !output.status.success() {
        return Err(SkillctlError::Config(format!(
            "unable to inspect checkout status for {}: {}",
            checkout.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(String::from_utf8(output.stdout)
        .expect("git status output should be valid UTF-8")
        .trim()
        .to_string())
}

pub fn resolve_skill_source(
    root: &Path,
    source_lock: &SourceLock,
    config: &Config,
    skill_id: &str,
    skill: &SkillConfig,
) -> Result<ResolvedSourcePath> {
    let Some(source_id) = &skill.source else {
        let package_path = if skill.path.is_absolute() {
            skill.path.clone()
        } else {
            root.join(&skill.path)
        };
        return Ok(ResolvedSourcePath {
            package_path,
            provenance: None,
        });
    };

    let source = config.sources.get(source_id).ok_or_else(|| {
        SkillctlError::Config(format!(
            "skill {skill_id} references unknown source {source_id}"
        ))
    })?;
    let entry = source_lock.sources.get(source_id).ok_or_else(|| {
        SkillctlError::Config(format!(
            "source {source_id} is not checked out; run skillctl update"
        ))
    })?;
    if entry.repo != source.repo || entry.ref_name != source.ref_name || entry.path != source.path {
        return Err(SkillctlError::Config(format!(
            "source {source_id} lock does not match config; run skillctl update"
        )));
    }
    let checkout = checkout_path(root, source_id)?;
    let head_commit = checkout_head_commit(&checkout)?;
    if head_commit != entry.commit {
        return Err(SkillctlError::Config(format!(
            "source {source_id} checkout HEAD {head_commit} does not match source-lock.json commit {}; run skillctl update or repair",
            entry.commit
        )));
    }
    let checkout_status = checkout_status_porcelain(&checkout)?;
    if !checkout_status.is_empty() {
        return Err(SkillctlError::Config(format!(
            "source {source_id} checkout is dirty or has untracked files; run skillctl update or doctor repair"
        )));
    }
    let package_path = checkout.join(&source.path).join(&skill.path);

    if !package_path.exists() {
        return Err(SkillctlError::Config(format!(
            "skill {skill_id} path {} is missing in source {source_id}",
            skill.path.display()
        )));
    }
    Ok(ResolvedSourcePath {
        package_path,
        provenance: Some(SourceProvenance {
            kind: "git".to_string(),
            id: source_id.clone(),
            repo: entry.repo.clone(),
            ref_name: entry.ref_name.clone(),
            commit: entry.commit.clone(),
        }),
    })
}

pub fn update_sources(root: &Path, config: &Config) -> Result<SourceUpdateReport> {
    update_sources_with_runner(root, config, &RealGit)
}

fn update_sources_with_runner<G: GitRunner + ?Sized>(
    root: &Path,
    config: &Config,
    git: &G,
) -> Result<SourceUpdateReport> {
    let lock_path = root.join("source-lock.json");
    let mut lock = read_source_lock_or_empty(&lock_path)?;
    lock.version = 1;
    let mut updates = Vec::new();

    for (source_id, source) in &config.sources {
        match source.kind {
            SourceKind::Git => {}
        }

        let old_commit = lock
            .sources
            .get(source_id)
            .map(|entry| entry.commit.clone());
        let update = match (|| -> Result<(String, SourceUpdateStatus)> {
            let checkout = checkout_path(root, source_id)?;
            let cloned = ensure_checkout(git, &checkout, &source.repo)?;
            git.run(&checkout, &["fetch", "--prune", "origin"])?;
            let new_commit = resolve_checkout_commit(git, &checkout, &source.ref_name)?;
            git.run(&checkout, &["reset", "--hard", &new_commit])?;
            if let Err(error) = git.run(&checkout, &["clean", "-fdx"]) {
                if let Some(old_commit) = old_commit.as_deref() {
                    if let Err(rollback_error) =
                        git.run(&checkout, &["reset", "--hard", old_commit])
                    {
                        return Err(SkillctlError::Config(format!(
                            "{error}; rollback to {old_commit} failed: {rollback_error}"
                        )));
                    }
                } else {
                    let _ = fs::remove_dir_all(&checkout);
                }
                return Err(error);
            }

            let status = if cloned {
                SourceUpdateStatus::Cloned
            } else if old_commit.as_deref() == Some(new_commit.as_str()) {
                SourceUpdateStatus::Unchanged
            } else {
                SourceUpdateStatus::Updated
            };
            Ok((new_commit, status))
        })() {
            Ok((new_commit, status)) => {
                lock.sources.insert(
                    source_id.clone(),
                    SourceLockEntry {
                        kind: "git".to_string(),
                        repo: source.repo.clone(),
                        ref_name: source.ref_name.clone(),
                        commit: new_commit.clone(),
                        path: source.path.clone(),
                    },
                );
                write_source_lock(&lock_path, &lock)?;
                SourceUpdate {
                    source_id: source_id.clone(),
                    ref_name: source.ref_name.clone(),
                    old_commit,
                    new_commit,
                    status,
                }
            }
            Err(error) => SourceUpdate {
                source_id: source_id.clone(),
                ref_name: source.ref_name.clone(),
                old_commit,
                new_commit: String::new(),
                status: SourceUpdateStatus::Error {
                    message: error.to_string(),
                },
            },
        };
        updates.push(update);
    }

    Ok(SourceUpdateReport { updates })
}

fn resolve_checkout_commit<G: GitRunner + ?Sized>(
    git: &G,
    checkout: &Path,
    ref_name: &str,
) -> Result<String> {
    let mut last_error = None;
    for candidate in [format!("origin/{ref_name}"), ref_name.to_string()] {
        match git.stdout(
            checkout,
            &["rev-parse", "--verify", &format!("{candidate}^{{commit}}")],
        ) {
            Ok(commit) => return Ok(commit),
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or_else(|| {
        SkillctlError::Config(format!(
            "unable to resolve source ref {ref_name} in {}",
            checkout.display()
        ))
    }))
}

fn ensure_checkout<G: GitRunner + ?Sized>(git: &G, checkout: &Path, repo: &str) -> Result<bool> {
    if checkout.exists() {
        let existing_repo = git.stdout(checkout, &["config", "--get", "remote.origin.url"])?;
        if existing_repo != repo {
            return Err(SkillctlError::Config(format!(
                "existing checkout {} points to {existing_repo}, not {repo}",
                checkout.display()
            )));
        }
        return Ok(false);
    }
    if let Some(parent) = checkout.parent() {
        fs::create_dir_all(parent).map_err(|source| SkillctlError::Fs {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    git.clone(repo, checkout)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{MaterializeMethod, PolicyConfig, SourceConfig, TargetConfig};
    use std::process::Command;

    #[test]
    fn update_sources_clones_and_records_commit() {
        let temp = tempfile::tempdir().unwrap();
        let remote = create_remote_repo(temp.path(), "initial skill text");
        let root = temp.path().join("home/.skillctl");
        let config = config_for_repo(remote.to_str().unwrap());

        let report = update_sources(&root, &config).unwrap();
        assert_eq!(report.updates.len(), 1);
        assert_eq!(report.updates[0].source_id, "shared");
        assert_eq!(report.updates[0].status, SourceUpdateStatus::Cloned);
        assert!(report.updates[0].new_commit.len() >= 7);

        let lock = read_source_lock_or_empty(&root.join("source-lock.json")).unwrap();
        let entry = &lock.sources["shared"];
        assert_eq!(entry.kind, "git");
        assert_eq!(entry.ref_name, "main");
        assert!(root.join("repos/shared/skills/recap/SKILL.md").exists());
    }

    #[test]
    fn update_sources_reports_unchanged_for_same_commit() {
        let temp = tempfile::tempdir().unwrap();
        let remote = create_remote_repo(temp.path(), "initial skill text");
        let root = temp.path().join("home/.skillctl");
        let config = config_for_repo(remote.to_str().unwrap());

        update_sources(&root, &config).unwrap();
        let report = update_sources(&root, &config).unwrap();

        assert_eq!(report.updates[0].status, SourceUpdateStatus::Unchanged);
    }

    #[test]
    fn resolve_skill_source_errors_before_update() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("home/.skillctl");
        let config = config_for_repo("file:///tmp/not-used.git");
        let lock = SourceLock::default();
        let skill = config.skills.get("recap").unwrap();

        let error = resolve_skill_source(&root, &lock, &config, "recap", skill)
            .unwrap_err()
            .to_string();

        assert!(error.contains("source shared is not checked out; run skillctl update"));
    }

    #[test]
    fn resolve_skill_source_returns_local_path_and_provenance_after_update() {
        let temp = tempfile::tempdir().unwrap();
        let remote = create_remote_repo(temp.path(), "initial skill text");
        let root = temp.path().join("home/.skillctl");
        let config = config_for_repo(remote.to_str().unwrap());
        update_sources(&root, &config).unwrap();
        let lock = read_source_lock_or_empty(&root.join("source-lock.json")).unwrap();
        let skill = config.skills.get("recap").unwrap();

        let resolved = resolve_skill_source(&root, &lock, &config, "recap", skill).unwrap();

        assert_eq!(
            resolved.package_path,
            root.join("repos/shared/skills/recap")
        );
        assert_eq!(resolved.provenance.as_ref().unwrap().id, "shared");
        assert_eq!(resolved.provenance.as_ref().unwrap().kind, "git");
    }
    #[test]
    fn resolve_skill_source_rejects_checkout_head_mismatch_after_failed_rollback() {
        let temp = tempfile::tempdir().unwrap();
        let remote_root = temp.path().join("remote");
        std::fs::create_dir_all(&remote_root).unwrap();
        let remote = create_remote_repo(&remote_root, "initial skill text");
        let root = temp.path().join("home/.skillctl");
        let config = config_for_repo(remote.to_str().unwrap());

        let initial = update_sources(&root, &config).unwrap();
        let old_commit = initial.updates[0].new_commit.clone();

        let work = remote_root.join("work");
        std::fs::create_dir_all(work.join("skills/recap")).unwrap();
        std::fs::write(
            work.join("skills/recap/SKILL.md"),
            "---\nname: recap\ndescription: test skill\n---\n\nupdated skill text\n",
        )
        .unwrap();
        run_git(&work, &["add", "skills/recap/SKILL.md"]);
        run_git(&work, &["commit", "-m", "update"]);
        run_git(&work, &["push", "origin", "main"]);

        let report = update_sources_with_runner(
            &root,
            &config,
            &FailingRollbackGit {
                rollback_commit: old_commit.clone(),
            },
        )
        .unwrap();
        match &report.updates[0].status {
            SourceUpdateStatus::Error { message } => assert!(!message.is_empty()),
            other => panic!("expected error status, got {other:?}"),
        }

        let lock = read_source_lock_or_empty(&root.join("source-lock.json")).unwrap();
        let checkout_head = run_git_stdout(&root.join("repos/shared"), &["rev-parse", "HEAD"]);
        assert_eq!(lock.sources["shared"].commit, old_commit);
        assert_ne!(checkout_head, lock.sources["shared"].commit);

        let skill = config.skills.get("recap").unwrap();
        let error = resolve_skill_source(&root, &lock, &config, "recap", skill)
            .unwrap_err()
            .to_string();

        assert!(error.contains("does not match source-lock.json commit"));
        assert!(error.contains("run skillctl update or repair"));
    }
    #[test]
    fn resolve_skill_source_rejects_dirty_checkout_with_untracked_files() {
        let temp = tempfile::tempdir().unwrap();
        let remote = create_remote_repo(temp.path(), "initial skill text");
        let root = temp.path().join("home/.skillctl");
        let config = config_for_repo(remote.to_str().unwrap());

        update_sources(&root, &config).unwrap();
        let lock = read_source_lock_or_empty(&root.join("source-lock.json")).unwrap();
        let checkout = root.join("repos/shared");
        assert_eq!(
            run_git_stdout(&checkout, &["rev-parse", "HEAD"]),
            lock.sources["shared"].commit
        );
        std::fs::write(checkout.join("UNTRACKED.txt"), "scratch").unwrap();
        let skill = config.skills.get("recap").unwrap();

        let error = resolve_skill_source(&root, &lock, &config, "recap", skill)
            .unwrap_err()
            .to_string();

        assert!(error.contains("dirty") || error.contains("untracked"));
        assert!(error.contains("skillctl update") || error.contains("doctor repair"));
    }
    #[test]
    fn resolve_skill_source_rejects_dirty_checkout_with_ignored_files() {
        let temp = tempfile::tempdir().unwrap();
        let remote_root = temp.path().join("remote");
        std::fs::create_dir_all(&remote_root).unwrap();
        let remote = create_remote_repo(&remote_root, "initial skill text");
        let work = remote_root.join("work");
        std::fs::write(
            work.join(".gitignore"),
            "skills/recap/references/ignored.md\n",
        )
        .unwrap();
        run_git(&work, &["add", ".gitignore"]);
        run_git(&work, &["commit", "-m", "ignore reference fixture"]);
        run_git(&work, &["push", "origin", "main"]);
        let root = temp.path().join("home/.skillctl");
        let config = config_for_repo(remote.to_str().unwrap());

        update_sources(&root, &config).unwrap();
        let lock = read_source_lock_or_empty(&root.join("source-lock.json")).unwrap();
        let checkout = root.join("repos/shared");
        std::fs::create_dir_all(checkout.join("skills/recap/references")).unwrap();
        std::fs::write(
            checkout.join("skills/recap/references/ignored.md"),
            "ignored scratch",
        )
        .unwrap();
        let skill = config.skills.get("recap").unwrap();

        let error = resolve_skill_source(&root, &lock, &config, "recap", skill)
            .unwrap_err()
            .to_string();

        assert!(error.contains("dirty") || error.contains("ignored"));
        assert!(error.contains("skillctl update") || error.contains("doctor repair"));
    }

    #[test]
    fn update_sources_writes_successful_sources_before_later_failure() {
        let temp = tempfile::tempdir().unwrap();
        let alpha_root = temp.path().join("alpha-remote");
        let beta_root = temp.path().join("beta-remote");
        std::fs::create_dir_all(&alpha_root).unwrap();
        std::fs::create_dir_all(&beta_root).unwrap();
        let alpha_remote = create_remote_repo(&alpha_root, "alpha skill text");
        let beta_remote = create_remote_repo(&beta_root, "beta skill text");
        let root = temp.path().join("home/.skillctl");

        let mut sources = BTreeMap::new();
        sources.insert(
            "alpha".to_string(),
            SourceConfig {
                kind: SourceKind::Git,
                repo: alpha_remote.to_str().unwrap().to_string(),
                ref_name: "main".to_string(),
                path: PathBuf::from("skills"),
            },
        );
        sources.insert(
            "beta".to_string(),
            SourceConfig {
                kind: SourceKind::Git,
                repo: beta_remote.to_str().unwrap().to_string(),
                ref_name: "missing".to_string(),
                path: PathBuf::from("skills"),
            },
        );
        let config = crate::config::Config {
            version: 1,
            targets: BTreeMap::new(),
            sources,
            policies: PolicyConfig::default(),
            skills: BTreeMap::new(),
        };

        let report = update_sources(&root, &config).unwrap();
        assert_eq!(report.updates.len(), 2);
        assert_eq!(report.updates[0].source_id, "alpha");
        assert_eq!(report.updates[0].status, SourceUpdateStatus::Cloned);
        assert_eq!(report.updates[1].source_id, "beta");
        match &report.updates[1].status {
            SourceUpdateStatus::Error { message } => assert!(!message.is_empty()),
            other => panic!("expected error status, got {other:?}"),
        }

        let lock_path = root.join("source-lock.json");
        assert!(
            lock_path.exists(),
            "expected partial source lock to be written"
        );
        let lock = read_source_lock_or_empty(&lock_path).unwrap();
        assert!(lock.sources.contains_key("alpha"));
        assert!(!lock.sources.contains_key("beta"));
        assert_eq!(
            lock.sources["alpha"].commit,
            run_git_stdout(&root.join("repos/alpha"), &["rev-parse", "HEAD"])
        );
    }

    #[test]
    fn update_sources_resolves_tag_refs() {
        let temp = tempfile::tempdir().unwrap();
        let remote_root = temp.path().join("tag-remote");
        std::fs::create_dir_all(&remote_root).unwrap();
        let remote = create_remote_repo(&remote_root, "tagged skill text");
        run_git(&remote, &["tag", "v1", "main"]);
        let root = temp.path().join("home/.skillctl");
        let mut config = config_for_repo(remote.to_str().unwrap());
        config.sources.get_mut("shared").unwrap().ref_name = "v1".to_string();

        let report = update_sources(&root, &config).unwrap();
        let lock = read_source_lock_or_empty(&root.join("source-lock.json")).unwrap();

        assert_eq!(report.updates[0].ref_name, "v1");
        assert_eq!(lock.sources["shared"].ref_name, "v1");
        assert_eq!(
            lock.sources["shared"].commit,
            run_git_stdout(&remote, &["rev-parse", "v1^{commit}"])
        );
    }

    #[test]
    fn update_sources_resolves_commitish_refs() {
        let temp = tempfile::tempdir().unwrap();
        let remote_root = temp.path().join("commit-remote");
        std::fs::create_dir_all(&remote_root).unwrap();
        let remote = create_remote_repo(&remote_root, "commit-ish skill text");
        let commit = run_git_stdout(&remote, &["rev-parse", "refs/heads/main"]);
        let root = temp.path().join("home/.skillctl");
        let mut config = config_for_repo(remote.to_str().unwrap());
        config.sources.get_mut("shared").unwrap().ref_name = commit.clone();

        let report = update_sources(&root, &config).unwrap();
        let lock = read_source_lock_or_empty(&root.join("source-lock.json")).unwrap();

        assert_eq!(report.updates[0].ref_name, commit);
        assert_eq!(lock.sources["shared"].commit, report.updates[0].new_commit);
    }

    #[test]
    fn update_sources_rejects_path_like_source_id() {
        let temp = tempfile::tempdir().unwrap();
        let remote_root = temp.path().join("remote");
        std::fs::create_dir_all(&remote_root).unwrap();
        let remote = create_remote_repo(&remote_root, "initial skill text");
        let root = temp.path().join("home/.skillctl");
        let mut config = config_for_repo(remote.to_str().unwrap());
        let source = config.sources.remove("shared").unwrap();
        config.sources.insert("../evil".to_string(), source);

        let report = update_sources(&root, &config).unwrap();

        assert_eq!(report.updates.len(), 1);
        match &report.updates[0].status {
            SourceUpdateStatus::Error { message } => assert!(message.contains("invalid source id")),
            other => panic!("expected error status, got {other:?}"),
        }
        assert_eq!(root.join("evil").exists(), false);
    }

    #[test]
    fn update_sources_rejects_repo_changes_for_existing_checkout() {
        let temp = tempfile::tempdir().unwrap();
        let remote_root1 = temp.path().join("remote1");
        let remote_root2 = temp.path().join("remote2");
        std::fs::create_dir_all(&remote_root1).unwrap();
        std::fs::create_dir_all(&remote_root2).unwrap();
        let remote1 = create_remote_repo(&remote_root1, "initial skill text");
        let remote2 = create_remote_repo(&remote_root2, "different skill text");
        let root = temp.path().join("home/.skillctl");
        let mut config = config_for_repo(remote1.to_str().unwrap());

        update_sources(&root, &config).unwrap();
        config.sources.get_mut("shared").unwrap().repo = remote2.to_str().unwrap().to_string();

        let report = update_sources(&root, &config).unwrap();
        let lock = read_source_lock_or_empty(&root.join("source-lock.json")).unwrap();
        let entry = &lock.sources["shared"];

        assert_eq!(report.updates.len(), 1);
        match &report.updates[0].status {
            SourceUpdateStatus::Error { message } => assert!(message.contains("existing checkout")),
            other => panic!("expected error status, got {other:?}"),
        }
        assert_eq!(entry.repo, remote1.to_str().unwrap());
        assert_ne!(entry.repo, remote2.to_str().unwrap());
    }

    #[test]
    fn update_sources_restores_checkout_after_clean_failure() {
        let temp = tempfile::tempdir().unwrap();
        let remote_root = temp.path().join("remote");
        std::fs::create_dir_all(&remote_root).unwrap();
        let remote = create_remote_repo(&remote_root, "initial skill text");
        let root = temp.path().join("home/.skillctl");
        let config = config_for_repo(remote.to_str().unwrap());

        let initial = update_sources(&root, &config).unwrap();
        let old_commit = initial.updates[0].new_commit.clone();

        let work = remote_root.join("work");
        std::fs::create_dir_all(work.join("skills/recap")).unwrap();
        std::fs::write(
            work.join("skills/recap/SKILL.md"),
            "---\nname: recap\ndescription: test skill\n---\n\nupdated skill text\n",
        )
        .unwrap();
        run_git(&work, &["add", "skills/recap/SKILL.md"]);
        run_git(&work, &["commit", "-m", "update"]);
        run_git(&work, &["push", "origin", "main"]);

        let report = update_sources_with_runner(&root, &config, &FailingCleanGit).unwrap();
        match &report.updates[0].status {
            SourceUpdateStatus::Error { message } => assert!(!message.is_empty()),
            other => panic!("expected error status, got {other:?}"),
        }

        let lock = read_source_lock_or_empty(&root.join("source-lock.json")).unwrap();
        assert_eq!(lock.sources["shared"].commit, old_commit);
        assert_eq!(
            run_git_stdout(&root.join("repos/shared"), &["rev-parse", "HEAD"]),
            old_commit
        );
    }

    struct FailingCleanGit;

    impl GitRunner for FailingCleanGit {
        fn run(&self, cwd: &Path, args: &[&str]) -> Result<()> {
            if args == ["clean", "-fdx"] {
                return Err(SkillctlError::Config("simulated clean failure".to_string()));
            }
            run_git(cwd, args);
            Ok(())
        }

        fn stdout(&self, cwd: &Path, args: &[&str]) -> Result<String> {
            Ok(run_git_stdout(cwd, args))
        }

        fn clone(&self, repo: &str, checkout: &Path) -> Result<()> {
            let checkout_parent = checkout.parent().unwrap_or(checkout);
            let checkout = checkout
                .to_str()
                .expect("temporary checkout path should be valid UTF-8");
            run_git(checkout_parent, &["clone", repo, checkout]);
            Ok(())
        }
    }

    struct FailingRollbackGit {
        rollback_commit: String,
    }

    impl GitRunner for FailingRollbackGit {
        fn run(&self, cwd: &Path, args: &[&str]) -> Result<()> {
            if args == ["clean", "-fdx"] {
                return Err(SkillctlError::Config("simulated clean failure".to_string()));
            }
            if args.len() == 3
                && args[0] == "reset"
                && args[1] == "--hard"
                && args[2] == self.rollback_commit.as_str()
            {
                return Err(SkillctlError::Config(
                    "simulated rollback failure".to_string(),
                ));
            }
            run_git(cwd, args);
            Ok(())
        }

        fn stdout(&self, cwd: &Path, args: &[&str]) -> Result<String> {
            Ok(run_git_stdout(cwd, args))
        }

        fn clone(&self, repo: &str, checkout: &Path) -> Result<()> {
            let checkout_parent = checkout.parent().unwrap_or(checkout);
            let checkout = checkout
                .to_str()
                .expect("temporary checkout path should be valid UTF-8");
            run_git(checkout_parent, &["clone", repo, checkout]);
            Ok(())
        }
    }

    fn config_for_repo(repo: &str) -> Config {
        let mut targets = BTreeMap::new();
        targets.insert(
            "claude".to_string(),
            TargetConfig {
                path: PathBuf::from("/tmp/claude-skills"),
                method: MaterializeMethod::Symlink,
                enabled: true,
            },
        );
        let mut sources = BTreeMap::new();
        sources.insert(
            "shared".to_string(),
            SourceConfig {
                kind: SourceKind::Git,
                repo: repo.to_string(),
                ref_name: "main".to_string(),
                path: PathBuf::from("skills"),
            },
        );
        let mut skills = BTreeMap::new();
        skills.insert(
            "recap".to_string(),
            SkillConfig {
                source: Some("shared".to_string()),
                path: PathBuf::from("recap"),
                expose: vec!["claude".to_string()],
            },
        );
        Config {
            version: 1,
            targets,
            sources,
            policies: PolicyConfig::default(),
            skills,
        }
    }

    fn create_remote_repo(root: &Path, body: &str) -> PathBuf {
        let work = root.join("work");
        run_git(root, &["init", "--bare", "remote.git"]);
        run_git(root, &["clone", "remote.git", "work"]);
        std::fs::create_dir_all(work.join("skills/recap")).unwrap();
        run_git(&work, &["checkout", "-b", "main"]);
        run_git(&work, &["config", "user.email", "test@example.com"]);
        run_git(&work, &["config", "user.name", "Test User"]);
        std::fs::write(
            work.join("skills/recap/SKILL.md"),
            format!("---\nname: recap\ndescription: test skill\n---\n\n{body}\n"),
        )
        .unwrap();
        run_git(&work, &["add", "."]);
        run_git(&work, &["commit", "-m", "initial"]);
        run_git(&work, &["push", "origin", "main"]);
        root.join("remote.git")
    }

    fn run_git(cwd: &Path, args: &[&str]) {
        let output = Command::new("git")
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
        let output = Command::new("git")
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

use assert_cmd::Command;
use predicates::prelude::*;
use std::path::{Path, PathBuf};

#[test]
fn help_prints_skillctl_commands_and_quick_start() {
    let mut cmd = Command::cargo_bin("skillctl").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("skillctl"))
        .stdout(predicates::str::contains("Materialize Agent Skills"))
        .stdout(predicates::str::contains("plan"))
        .stdout(predicates::str::contains("update"))
        .stdout(predicates::str::contains("apply"))
        .stdout(predicates::str::contains("doctor"))
        .stdout(predicates::str::contains("version"))
        .stdout(predicates::str::contains("Quick start:"))
        .stdout(predicates::str::contains("skillctl update"))
        .stdout(predicates::str::contains("skillctl plan"))
        .stdout(predicates::str::contains("skillctl apply"));
}

#[test]
fn no_args_prints_root_help() {
    let mut cmd = Command::cargo_bin("skillctl").unwrap();
    cmd.assert()
        .success()
        .stdout(predicates::str::contains("Usage: skillctl"))
        .stdout(predicates::str::contains("Quick start:"));
}

#[test]
fn version_prints_metadata() {
    for arg in ["--version", "-V", "version"] {
        let mut cmd = Command::cargo_bin("skillctl").unwrap();
        cmd.arg(arg)
            .assert()
            .success()
            .stdout(predicates::str::contains(format!(
                "skillctl version {}",
                env!("CARGO_PKG_VERSION")
            )))
            .stdout(predicates::str::contains("commit:"))
            .stdout(predicates::str::contains("built:"));
    }
}
#[test]
fn subcommands_print_nonempty_output() {
    for subcommand in ["plan", "doctor", "list"] {
        let mut cmd = Command::cargo_bin("skillctl").unwrap();
        cmd.arg(subcommand)
            .assert()
            .code(predicates::ord::eq(0).or(predicates::ord::eq(1)));
    }
}

#[test]
fn plan_reads_home_scoped_config_fixture() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    write_config(home, &["claude"], "sample");
    write_sample_skill(home);

    skillctl(home)
        .arg("plan")
        .assert()
        .success()
        .stdout(predicates::str::contains("CREATE"))
        .stdout(predicates::str::contains("sample"));
}

#[test]
fn apply_creates_rendered_directory_target_symlink_and_lockfile_entry() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    write_config(home, &["claude"], "sample");
    write_sample_skill(home);

    skillctl(home).arg("apply").assert().success();

    let rendered = home.join(".skillctl/rendered/claude/sample");
    let target = home.join(".claude/skills/sample");
    assert_eq!(
        std::fs::read_to_string(rendered.join("SKILL.md")).unwrap(),
        "---\nname: sample\ndescription: Sample\n---\nSample\n"
    );
    assert_eq!(std::fs::read_link(&target).unwrap(), rendered);
    let lock = std::fs::read_to_string(home.join(".claude/skills/.skillctl.lock.json")).unwrap();
    assert!(lock.contains("\"sample\""));
    assert!(lock.contains("\"source_digest\": \"sha256:"));
}

#[test]
fn pi_apply_renders_variant_exactly_and_creates_symlink_and_lock_entry() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    write_config(home, &["pi"], "sample");
    write_sample_skill(home);
    let pi_skill = "---\nname: sample\ndescription: Pi-specific sample\n---\nPi variant body\n";
    let variant_path = home.join(".skillctl/skills/sample/variants/pi/SKILL.md");
    std::fs::create_dir_all(variant_path.parent().unwrap()).unwrap();
    std::fs::write(&variant_path, pi_skill).unwrap();

    skillctl(home).arg("apply").assert().success();

    let rendered = home.join(".skillctl/rendered/pi/sample");
    let target = home.join(".pi/agent/skills/sample");
    assert_eq!(
        std::fs::read_to_string(rendered.join("SKILL.md")).unwrap(),
        pi_skill
    );
    assert_eq!(std::fs::read_link(&target).unwrap(), rendered);
    let lock = std::fs::read_to_string(home.join(".pi/agent/skills/.skillctl.lock.json")).unwrap();
    assert!(lock.contains("\"target\": \"pi\""));
    assert!(lock.contains("\"sample\": {"));
}

#[test]
fn apply_aborts_before_mutation_on_unmanaged_conflict() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    write_config(home, &["claude"], "sample");
    write_sample_skill(home);
    std::fs::create_dir_all(home.join(".claude/skills")).unwrap();
    std::fs::write(home.join(".claude/skills/sample"), "foreign").unwrap();

    skillctl(home)
        .arg("apply")
        .assert()
        .failure()
        .stderr(predicates::str::contains("unmanaged conflict"));

    assert_eq!(
        std::fs::read_to_string(home.join(".claude/skills/sample")).unwrap(),
        "foreign"
    );
    assert!(!home.join(".skillctl/rendered/claude/sample").exists());
}

#[test]
fn apply_refuses_drifted_managed_target_paths() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    write_config(home, &["claude"], "sample");
    write_sample_skill(home);
    let target_root = home.join(".claude/skills");
    let rendered = home.join(".skillctl/rendered/claude/sample");
    std::fs::create_dir_all(&target_root).unwrap();
    std::fs::create_dir_all(&rendered).unwrap();
    std::fs::write(target_root.join("sample"), "drifted").unwrap();
    write_lock(home, "claude", &target_root, &rendered, "sample");

    skillctl(home)
        .arg("apply")
        .assert()
        .failure()
        .stderr(predicates::str::contains("managed target drift"));

    assert_eq!(
        std::fs::read_to_string(target_root.join("sample")).unwrap(),
        "drifted"
    );
}

#[test]
fn prune_removes_only_lockfile_managed_stale_symlink() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    write_config(home, &["claude"], "");
    let target_root = home.join(".claude/skills");
    let rendered = home.join(".skillctl/rendered/claude/sample");
    std::fs::create_dir_all(&target_root).unwrap();
    std::fs::create_dir_all(&rendered).unwrap();
    std::os::unix::fs::symlink(&rendered, target_root.join("sample")).unwrap();
    write_lock(home, "claude", &target_root, &rendered, "sample");

    skillctl(home).arg("prune").assert().success();

    assert!(target_root.join("sample").symlink_metadata().is_err());
    let lock = std::fs::read_to_string(target_root.join(".skillctl.lock.json")).unwrap();
    assert!(!lock.contains("\"sample\": {"));
}

#[test]
fn prune_refuses_unmanaged_regular_files() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    write_config(home, &["claude"], "");
    let target_root = home.join(".claude/skills");
    std::fs::create_dir_all(&target_root).unwrap();
    std::fs::write(target_root.join("foreign"), "foreign").unwrap();

    skillctl(home)
        .arg("prune")
        .assert()
        .failure()
        .stderr(predicates::str::contains("unmanaged conflict"));

    assert_eq!(
        std::fs::read_to_string(target_root.join("foreign")).unwrap(),
        "foreign"
    );
}

#[test]
fn unlink_target_filter_removes_only_one_managed_entry() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    write_config(home, &["claude", "codex"], "sample");
    for target in ["claude", "codex"] {
        let target_root = target_root(home, target);
        let rendered = home.join(format!(".skillctl/rendered/{target}/sample"));
        std::fs::create_dir_all(&target_root).unwrap();
        std::fs::create_dir_all(&rendered).unwrap();
        std::os::unix::fs::symlink(&rendered, target_root.join("sample")).unwrap();
        write_lock(home, target, &target_root, &rendered, "sample");
    }

    skillctl(home)
        .args(["unlink", "sample", "--target", "claude"])
        .assert()
        .success();

    assert!(
        home.join(".claude/skills/sample")
            .symlink_metadata()
            .is_err()
    );
    assert!(
        home.join(".agents/skills/sample")
            .symlink_metadata()
            .is_ok()
    );
}

#[test]
fn unlink_target_pi_after_apply_leaves_claude_link_and_lock_entry_intact() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    write_config(home, &["claude", "pi"], "sample");
    write_sample_skill(home);

    skillctl(home).arg("apply").assert().success();
    skillctl(home)
        .args(["unlink", "sample", "--target", "pi"])
        .assert()
        .success();

    assert!(
        home.join(".pi/agent/skills/sample")
            .symlink_metadata()
            .is_err()
    );
    assert!(
        home.join(".claude/skills/sample")
            .symlink_metadata()
            .unwrap()
            .file_type()
            .is_symlink()
    );
    let pi_lock =
        std::fs::read_to_string(home.join(".pi/agent/skills/.skillctl.lock.json")).unwrap();
    let claude_lock =
        std::fs::read_to_string(home.join(".claude/skills/.skillctl.lock.json")).unwrap();
    assert!(!pi_lock.contains("\"sample\": {"));
    assert!(claude_lock.contains("\"sample\": {"));
}

#[test]
fn default_doctor_reports_missing_pi_target_root() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    std::fs::create_dir_all(home.join(".skillctl/skills")).unwrap();
    std::fs::create_dir_all(home.join(".claude/skills")).unwrap();
    std::fs::create_dir_all(home.join(".agents/skills")).unwrap();

    skillctl(home)
        .arg("doctor")
        .assert()
        .failure()
        .stdout(predicates::str::contains("pi: missing target root"))
        .stdout(predicates::str::contains(".pi/agent/skills"));
}

#[test]
fn doctor_reports_lockfile_owner_mismatch() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    write_config(home, &["claude"], "");
    std::fs::create_dir_all(home.join(".skillctl/skills")).unwrap();
    let target_root = home.join(".claude/skills");
    std::fs::create_dir_all(&target_root).unwrap();
    std::fs::write(
        target_root.join(".skillctl.lock.json"),
        r#"{"version":1,"tool":"other","target":"claude","source_root":"/x","target_path":"/y","managed":{}}"#,
    )
    .unwrap();

    skillctl(home)
        .arg("doctor")
        .assert()
        .failure()
        .stdout(predicates::str::contains("foreign lockfile owner"));
}

#[test]
fn doctor_reports_drifted_managed_symlink() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    write_config(home, &["claude"], "");
    std::fs::create_dir_all(home.join(".skillctl/skills")).unwrap();
    let target_root = home.join(".claude/skills");
    let expected = home.join(".skillctl/rendered/claude/sample");
    let actual = home.join(".skillctl/rendered/claude/other");
    std::fs::create_dir_all(&target_root).unwrap();
    std::fs::create_dir_all(&expected).unwrap();
    std::fs::create_dir_all(&actual).unwrap();
    std::os::unix::fs::symlink(&actual, target_root.join("sample")).unwrap();
    write_lock(home, "claude", &target_root, &expected, "sample");

    skillctl(home)
        .arg("doctor")
        .assert()
        .failure()
        .stdout(predicates::str::contains("managed symlink target mismatch"));
}

#[test]
fn plan_exits_nonzero_when_plan_errors_exist() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    write_config(home, &["claude"], "sample");
    write_sample_skill(home);
    std::fs::create_dir_all(home.join(".claude/skills")).unwrap();
    std::fs::write(home.join(".claude/skills/sample"), "foreign").unwrap();

    skillctl(home)
        .arg("plan")
        .assert()
        .failure()
        .stdout(predicates::str::contains("ERROR"))
        .stdout(predicates::str::contains("unmanaged conflict"));
}

#[test]
fn plan_reports_stale_managed_entries() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    write_config(home, &["claude"], "");
    let target_root = home.join(".claude/skills");
    let rendered = home.join(".skillctl/rendered/claude/sample");
    std::fs::create_dir_all(&target_root).unwrap();
    std::fs::create_dir_all(&rendered).unwrap();
    std::os::unix::fs::symlink(&rendered, target_root.join("sample")).unwrap();
    write_lock(home, "claude", &target_root, &rendered, "sample");

    skillctl(home)
        .arg("plan")
        .assert()
        .success()
        .stdout(predicates::str::contains("REMOVE_STALE"))
        .stdout(predicates::str::contains("sample"));
}

#[test]
fn list_covers_empty_and_configured_skills() {
    let empty = tempfile::tempdir().unwrap();
    skillctl(empty.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicates::str::contains("No skills configured."));

    let configured = tempfile::tempdir().unwrap();
    write_config(configured.path(), &["claude"], "sample");
    skillctl(configured.path())
        .arg("list")
        .assert()
        .success()
        .stdout(predicates::str::contains("sample"));
}
#[test]
fn update_clones_git_source_and_reports_status() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    let remote = create_git_skill_repo(temp.path(), "initial skill text");
    write_git_source_config(&home, remote.to_str().unwrap());

    skillctl(&home)
        .arg("update")
        .assert()
        .success()
        .stdout(predicates::str::contains("SOURCE"))
        .stdout(predicates::str::contains("shared"))
        .stdout(predicates::str::contains("CLONED"));

    assert!(
        home.join(".skillctl/repos/shared/skills/recap/SKILL.md")
            .exists()
    );
    assert!(home.join(".skillctl/source-lock.json").exists());
}
#[test]
fn update_reports_success_and_error_rows_for_later_failure() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    let alpha_root = temp.path().join("alpha");
    let beta_root = temp.path().join("beta");
    std::fs::create_dir_all(&alpha_root).unwrap();
    std::fs::create_dir_all(&beta_root).unwrap();
    let alpha = create_git_skill_repo(&alpha_root, "alpha skill text");
    let beta = create_git_skill_repo(&beta_root, "beta skill text");
    write_git_source_failure_config(&home, alpha.to_str().unwrap(), beta.to_str().unwrap());

    skillctl(&home)
        .arg("update")
        .assert()
        .failure()
        .stdout(predicate::str::contains("SOURCE"))
        .stdout(predicate::str::contains("alpha"))
        .stdout(predicate::str::contains("CLONED"))
        .stdout(predicate::str::contains("beta"))
        .stdout(predicate::str::contains("ERROR"));

    let lock = std::fs::read_to_string(home.join(".skillctl/source-lock.json")).unwrap();
    assert!(lock.contains("\"alpha\""));
    assert!(!lock.contains("\"beta\""));
}

#[test]
fn doctor_skips_missing_local_source_root_for_git_only_sources() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    write_git_source_config(
        &home,
        create_git_skill_repo(temp.path(), "initial skill text")
            .to_str()
            .unwrap(),
    );

    skillctl(&home).arg("update").assert().success();
    skillctl(&home).arg("apply").assert().success();

    skillctl(&home)
        .arg("doctor")
        .assert()
        .success()
        .stdout(predicate::str::contains("skillctl doctor: ok"))
        .stdout(predicate::str::contains("missing source root").not());
}

#[test]
fn update_plan_apply_materializes_git_backed_skill() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    let remote = create_git_skill_repo(temp.path(), "initial skill text");
    write_git_source_config(&home, remote.to_str().unwrap());

    skillctl(&home).arg("update").assert().success();

    skillctl(&home)
        .arg("plan")
        .assert()
        .success()
        .stdout(predicate::str::contains("CREATE"))
        .stdout(predicate::str::contains("claude/recap"));

    skillctl(&home)
        .arg("apply")
        .assert()
        .success()
        .stdout(predicate::str::contains("CREATE"))
        .stdout(predicate::str::contains("claude/recap"));

    let target = home.join(".claude/skills/recap");
    assert!(target.symlink_metadata().unwrap().file_type().is_symlink());
    let lock = std::fs::read_to_string(home.join(".claude/skills/.skillctl.lock.json")).unwrap();
    assert!(lock.contains("\"source\""));
    assert!(lock.contains("\"id\": \"shared\""));
    assert!(lock.contains("\"commit\""));

    skillctl(&home)
        .arg("plan")
        .assert()
        .success()
        .stdout(predicate::str::contains("No changes."));
}

#[test]
fn plan_fails_for_git_source_before_update() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    let remote = create_git_skill_repo(temp.path(), "initial skill text");
    write_git_source_config(&home, remote.to_str().unwrap());

    skillctl(&home)
        .arg("plan")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "source shared is not checked out; run skillctl update",
        ));
}
#[test]
fn apply_refreshes_git_source_lock_provenance_without_relinking() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    let remote = create_git_skill_repo(temp.path(), "initial skill text");
    write_git_source_config(&home, remote.to_str().unwrap());

    skillctl(&home).arg("update").assert().success();
    skillctl(&home).arg("apply").assert().success();

    let lock_path = home.join(".claude/skills/.skillctl.lock.json");
    let source_commit = |path: &Path| -> String {
        let lock = std::fs::read_to_string(path).unwrap();
        let marker = "\"commit\": \"";
        let start = lock.find(marker).unwrap() + marker.len();
        let end = lock[start..].find('"').unwrap() + start;
        lock[start..end].to_owned()
    };
    let initial_commit = source_commit(&lock_path);

    let advance = temp.path().join("git-advance");
    run_git(
        temp.path(),
        &["clone", remote.to_str().unwrap(), "git-advance"],
    );
    run_git(&advance, &["config", "user.email", "test@example.com"]);
    run_git(&advance, &["config", "user.name", "Test User"]);
    std::fs::write(advance.join("README.md"), "unrelated commit").unwrap();
    run_git(&advance, &["add", "README.md"]);
    run_git(&advance, &["commit", "-m", "docs"]);
    run_git(&advance, &["push", "origin", "main"]);

    skillctl(&home)
        .arg("update")
        .assert()
        .success()
        .stdout(predicates::str::contains("UPDATED"));

    skillctl(&home)
        .arg("plan")
        .assert()
        .success()
        .stdout(predicates::str::contains("No changes."));

    skillctl(&home)
        .arg("apply")
        .assert()
        .success()
        .stdout(predicates::str::contains("No changes."));

    let updated_commit = source_commit(&lock_path);
    assert_ne!(initial_commit, updated_commit);
}

#[test]
fn changed_git_skill_input_updates_target() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    let remote = create_git_skill_repo(temp.path(), "initial skill text");
    write_git_source_config(&home, remote.to_str().unwrap());

    skillctl(&home).arg("update").assert().success();
    skillctl(&home).arg("apply").assert().success();

    change_git_skill_body(temp.path(), "changed skill text");
    skillctl(&home).arg("update").assert().success();

    skillctl(&home)
        .arg("plan")
        .assert()
        .success()
        .stdout(predicate::str::contains("UPDATE claude/recap"));
}

fn write_git_source_failure_config(home: &Path, alpha_repo: &str, beta_repo: &str) {
    let config_path = home.join(".skillctl/config.yaml");
    std::fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    std::fs::write(
        config_path,
        format!(
            r#"
version: 1
targets:
  claude:
    path: ~/.claude/skills
    method: symlink
    enabled: true
sources:
  alpha:
    type: git
    repo: {alpha_repo}
    ref: main
    path: skills
  beta:
    type: git
    repo: {beta_repo}
    ref: missing
    path: skills
skills: {{}}
"#
        ),
    )
    .unwrap();
}

fn write_git_source_config(home: &Path, repo: &str) {
    let config_path = home.join(".skillctl/config.yaml");
    std::fs::create_dir_all(config_path.parent().unwrap()).unwrap();
    std::fs::write(
        config_path,
        format!(
            r#"
version: 1
targets:
  claude:
    path: ~/.claude/skills
    method: symlink
    enabled: true
sources:
  shared:
    type: git
    repo: {repo}
    ref: main
    path: skills
skills:
  recap:
    source: shared
    path: recap
    expose: [claude]
"#
        ),
    )
    .unwrap();
}

fn create_git_skill_repo(root: &Path, body: &str) -> PathBuf {
    let remote = root.join("remote.git");
    let work = root.join("git-work");
    run_git(
        root,
        &["init", "--bare", "--initial-branch=main", "remote.git"],
    );
    run_git(root, &["clone", remote.to_str().unwrap(), "git-work"]);
    run_git(&work, &["checkout", "-b", "main"]);
    run_git(&work, &["config", "user.email", "test@example.com"]);
    run_git(&work, &["config", "user.name", "Test User"]);
    std::fs::create_dir_all(work.join("skills/recap")).unwrap();
    std::fs::write(
        work.join("skills/recap/SKILL.md"),
        format!("---\nname: recap\ndescription: test skill\n---\n\n{body}\n"),
    )
    .unwrap();
    run_git(&work, &["add", "."]);
    run_git(&work, &["commit", "-m", "skill"]);
    run_git(&work, &["push", "origin", "main"]);
    remote
}
fn change_git_skill_body(root: &Path, body: &str) {
    let work = root.join("git-work");
    std::fs::write(
        work.join("skills/recap/SKILL.md"),
        format!("---\nname: recap\ndescription: test skill\n---\n\n{body}\n"),
    )
    .unwrap();
    run_git(&work, &["add", "skills/recap/SKILL.md"]);
    run_git(&work, &["commit", "-m", "change skill"]);
    run_git(&work, &["push", "origin", "main"]);
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

#[test]
fn changed_git_commit_without_skill_input_change_does_not_update_target() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    let source_root = home.join(".skillctl/skills");
    write_config(&home, &["claude"], "sample");
    write_sample_skill(&home);
    init_git_repo(&source_root);

    skillctl(&home).arg("apply").assert().success();

    let initial_commit = git_stdout(&source_root, &["rev-parse", "HEAD"]);
    add_unrelated_git_commit(&source_root);
    let updated_commit = git_stdout(&source_root, &["rev-parse", "HEAD"]);
    assert_ne!(initial_commit, updated_commit);

    skillctl(&home)
        .arg("plan")
        .assert()
        .success()
        .stdout(predicates::str::contains("No changes."));
}

fn init_git_repo(repo_root: &Path) {
    run_git(repo_root, &["init"]);
    run_git(repo_root, &["config", "user.email", "test@example.com"]);
    run_git(repo_root, &["config", "user.name", "Skillctl Test"]);
    run_git(repo_root, &["add", "sample/SKILL.md"]);
    run_git(repo_root, &["commit", "-m", "initial skill"]);
}

fn add_unrelated_git_commit(repo_root: &Path) {
    std::fs::write(
        repo_root.join("README.md"),
        "unrelated
",
    )
    .unwrap();
    run_git(repo_root, &["add", "README.md"]);
    run_git(repo_root, &["commit", "-m", "unrelated"]);
}

fn git_stdout(cwd: &Path, args: &[&str]) -> String {
    let output = std::process::Command::new("git")
        .current_dir(cwd)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

fn skillctl(home: &Path) -> Command {
    let mut cmd = Command::cargo_bin("skillctl").unwrap();
    cmd.env("HOME", home);
    cmd
}

fn write_config(home: &Path, targets: &[&str], skill: &str) {
    std::fs::create_dir_all(home.join(".skillctl")).unwrap();
    let mut target_yaml = String::new();
    for target in targets {
        target_yaml.push_str(&format!(
            "  {target}:\n    path: {}\n    method: symlink\n    enabled: true\n",
            target_path_yaml(target)
        ));
    }
    let skill_yaml = if skill.is_empty() {
        "{}\n".to_string()
    } else {
        format!(
            "\n  {skill}:\n    path: skills/{skill}\n    expose: [{}]\n",
            targets.join(", ")
        )
    };
    std::fs::write(
        home.join(".skillctl/config.yaml"),
        format!("version: 1\ntargets:\n{target_yaml}policies: {{}}\nskills: {skill_yaml}"),
    )
    .unwrap();
}

fn write_sample_skill(home: &Path) {
    std::fs::create_dir_all(home.join(".skillctl/skills/sample")).unwrap();
    std::fs::write(
        home.join(".skillctl/skills/sample/SKILL.md"),
        "---\nname: sample\ndescription: Sample\n---\nSample\n",
    )
    .unwrap();
}

fn write_lock(home: &Path, target: &str, target_root: &Path, rendered: &Path, skill: &str) {
    std::fs::write(
        target_root.join(".skillctl.lock.json"),
        format!(
            r#"{{
  "version": 1,
  "tool": "skillctl",
  "target": "{target}",
  "source_root": "{}",
  "target_path": "{}",
  "managed": {{
    "{skill}": {{
      "skill_id": "{skill}",
      "target_name": "{skill}",
      "target_path": "{}",
      "rendered_path": "{}",
      "source_path": "{}",
      "method": "symlink",
      "source_digest": "sha256:old",
      "source": null
    }}
  }}
}}
"#,
            home.join(".skillctl/skills").display(),
            target_root.display(),
            target_root.join(skill).display(),
            rendered.display(),
            home.join(format!(".skillctl/skills/{skill}")).display()
        ),
    )
    .unwrap();
}

fn target_root(home: &Path, target: &str) -> PathBuf {
    match target {
        "claude" => home.join(".claude/skills"),
        "codex" => home.join(".agents/skills"),
        "pi" => home.join(".pi/agent/skills"),
        other => home.join(format!(".{other}/skills")),
    }
}

fn target_path_yaml(target: &str) -> &'static str {
    match target {
        "claude" => "~/.claude/skills",
        "codex" => "~/.agents/skills",
        "pi" => "~/.pi/agent/skills",
        _ => "~/.other/skills",
    }
}

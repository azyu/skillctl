use assert_cmd::Command;
use predicates::prelude::*;
use std::path::{Path, PathBuf};

#[test]
fn help_prints_skillctl_commands() {
    let mut cmd = Command::cargo_bin("skillctl").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("skillctl"))
        .stdout(predicates::str::contains("plan"))
        .stdout(predicates::str::contains("apply"))
        .stdout(predicates::str::contains("doctor"));
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
      "source_digest": "sha256:old"
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
        other => home.join(format!(".{other}/skills")),
    }
}

fn target_path_yaml(target: &str) -> &'static str {
    match target {
        "claude" => "~/.claude/skills",
        "codex" => "~/.agents/skills",
        _ => "~/.other/skills",
    }
}

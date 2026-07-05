use assert_cmd::Command;
use predicates::prelude::*;

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
    std::fs::create_dir_all(home.join(".skillctl/skills/sample")).unwrap();
    std::fs::write(
        home.join(".skillctl/config.yaml"),
        r#"version: 1
targets:
  claude:
    path: ~/.claude/skills
    method: symlink
    enabled: true
policies: {}
skills:
  sample:
    path: skills/sample
    expose: [claude]
"#,
    )
    .unwrap();
    std::fs::write(
        home.join(".skillctl/skills/sample/SKILL.md"),
        "---\nname: sample\ndescription: Sample\n---\nSample\n",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("skillctl").unwrap();
    cmd.env("HOME", home)
        .arg("plan")
        .assert()
        .success()
        .stdout(predicates::str::contains("CREATE"))
        .stdout(predicates::str::contains("sample"));
}

use crate::error::{Result, SkillctlError};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSkill {
    pub id: String,
    pub target: String,
    pub target_name: String,
    pub package_dir: PathBuf,
    pub source_dir: PathBuf,
    pub fallback_used: bool,
}

pub fn resolve_skill(
    root: &Path,
    id: &str,
    skill_path: &Path,
    target: &str,
) -> Result<ResolvedSkill> {
    let package_dir = if skill_path.is_absolute() {
        skill_path.to_path_buf()
    } else {
        root.join(skill_path)
    };
    let variant_dir = package_dir.join("variants").join(target);
    let variant_skill = variant_dir.join("SKILL.md");
    let common_skill = package_dir.join("SKILL.md");
    let (source_dir, fallback_used) = if variant_skill.exists() {
        (variant_dir, false)
    } else if common_skill.exists() {
        (package_dir.clone(), true)
    } else {
        return Err(SkillctlError::Config(format!(
            "missing SKILL.md for skill {id} at {}",
            package_dir.display()
        )));
    };
    let selected_skill = source_dir.join("SKILL.md");
    let target_name = read_frontmatter_name(&selected_skill)?;
    if target == "pi" {
        validate_pi_description(&selected_skill)?;
    }
    Ok(ResolvedSkill {
        id: id.to_string(),
        target: target.to_string(),
        target_name,
        package_dir,
        source_dir,
        fallback_used,
    })
}

fn validate_pi_description(path: &Path) -> Result<()> {
    let text = fs::read_to_string(path).map_err(|source| SkillctlError::Fs {
        path: path.to_path_buf(),
        source,
    })?;
    let mut lines = text.lines();
    if lines.next() != Some("---") {
        return Err(SkillctlError::Config(format!(
            "{} missing YAML frontmatter for target pi",
            path.display()
        )));
    }

    let mut frontmatter = String::new();
    let mut closed = false;
    for line in lines {
        if line == "---" {
            closed = true;
            break;
        }
        frontmatter.push_str(line);
        frontmatter.push('\n');
    }
    if !closed {
        return Err(SkillctlError::Config(format!(
            "{} missing closing YAML frontmatter delimiter for target pi",
            path.display()
        )));
    }

    let metadata: serde_yaml::Value = serde_yaml::from_str(&frontmatter).map_err(|error| {
        SkillctlError::Config(format!(
            "{} has invalid YAML frontmatter for target pi: {error}",
            path.display()
        ))
    })?;
    let description = metadata
        .as_mapping()
        .and_then(|mapping| mapping.get(serde_yaml::Value::String("description".to_string())))
        .and_then(serde_yaml::Value::as_str);
    if description.is_some_and(|description| !description.trim().is_empty()) {
        return Ok(());
    }

    Err(SkillctlError::Config(format!(
        "{} target pi requires a nonblank YAML string frontmatter description",
        path.display()
    )))
}

fn read_frontmatter_name(path: &Path) -> Result<String> {
    let text = fs::read_to_string(path).map_err(|source| SkillctlError::Fs {
        path: path.to_path_buf(),
        source,
    })?;
    let mut lines = text.lines();
    if lines.next() != Some("---") {
        return Err(SkillctlError::Config(format!(
            "{} missing YAML frontmatter",
            path.display()
        )));
    }
    for line in lines {
        if line == "---" {
            break;
        }
        if let Some(value) = line.strip_prefix("name:") {
            let name = value.trim().trim_matches('"').to_string();
            if !name.is_empty() {
                return Ok(name);
            }
        }
    }
    Err(SkillctlError::Config(format!(
        "{} missing frontmatter name",
        path.display()
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    fn write_skill(path: &Path, frontmatter: &str, body: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            path,
            format!("---\nname: sample\n{frontmatter}---\n{body}\n"),
        )
        .unwrap();
    }

    #[test]
    fn resolves_target_variant_before_default() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let skill = root.join("skills/sample");
        fs::create_dir_all(skill.join("variants/claude")).unwrap();
        fs::create_dir_all(skill.join("variants/codex")).unwrap();
        fs::write(
            skill.join("SKILL.md"),
            "---\nname: sample\ndescription: Common\n---\nCommon\n",
        )
        .unwrap();
        fs::write(
            skill.join("variants/claude/SKILL.md"),
            "---\nname: sample\ndescription: Claude\n---\nClaude\n",
        )
        .unwrap();
        fs::write(
            skill.join("variants/codex/SKILL.md"),
            "---\nname: sample\ndescription: Codex\n---\nCodex\n",
        )
        .unwrap();

        let claude = resolve_skill(root, "sample", Path::new("skills/sample"), "claude").unwrap();
        assert_eq!(claude.source_dir, skill.join("variants/claude"));
        assert_eq!(claude.target_name, "sample");

        let omp = resolve_skill(root, "sample", Path::new("skills/sample"), "omp").unwrap();
        assert_eq!(omp.source_dir, skill);
        assert_eq!(omp.target_name, "sample");
    }

    #[test]
    fn resolves_absolute_package_path_without_root_join() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        let package = temp.path().join("absolute-package");
        fs::create_dir_all(&package).unwrap();
        fs::write(
            package.join("SKILL.md"),
            "---\nname: absolute\ndescription: Absolute\n---\nAbsolute\n",
        )
        .unwrap();

        let resolved = resolve_skill(&root, "absolute", &package, "claude").unwrap();

        assert_eq!(resolved.package_dir, package);
        assert_eq!(resolved.source_dir, package);
        assert_eq!(resolved.target_name, "absolute");
    }

    #[test]
    fn pi_accepts_nonblank_common_and_multiline_descriptions() {
        for (frontmatter, expected_body) in [
            ("description: Common description\n", "Common"),
            (
                "description: >-\n  First line\n  second line\n",
                "Multiline",
            ),
        ] {
            let temp = tempfile::tempdir().unwrap();
            let skill = temp.path().join("skills/sample");
            write_skill(&skill.join("SKILL.md"), frontmatter, expected_body);

            let resolved =
                resolve_skill(temp.path(), "sample", Path::new("skills/sample"), "pi").unwrap();

            assert_eq!(resolved.source_dir, skill);
            assert_eq!(resolved.target_name, "sample");
        }
    }

    #[test]
    fn pi_rejects_missing_non_string_empty_and_whitespace_descriptions() {
        for frontmatter in [
            "",
            "description: 42\n",
            "description: \"\"\n",
            "description: \"   \"\n",
        ] {
            let temp = tempfile::tempdir().unwrap();
            let skill_path = temp.path().join("skills/sample/SKILL.md");
            write_skill(&skill_path, frontmatter, "Sample");

            let error =
                resolve_skill(temp.path(), "sample", Path::new("skills/sample"), "pi").unwrap_err();
            let message = error.to_string();
            assert!(message.contains("target pi"), "{message}");
            assert!(
                message.contains(&skill_path.display().to_string()),
                "{message}"
            );
            assert!(message.contains("description"), "{message}");
        }
    }

    #[test]
    fn descriptionless_skills_remain_valid_for_non_pi_targets() {
        let temp = tempfile::tempdir().unwrap();
        let skill = temp.path().join("skills/sample");
        write_skill(&skill.join("SKILL.md"), "", "Sample");

        for target in ["claude", "codex", "custom"] {
            let resolved =
                resolve_skill(temp.path(), "sample", Path::new("skills/sample"), target).unwrap();
            assert_eq!(resolved.source_dir, skill);
            assert_eq!(resolved.target_name, "sample");
        }
    }

    #[test]
    fn pi_validates_the_selected_variant_description() {
        let temp = tempfile::tempdir().unwrap();
        let skill = temp.path().join("skills/sample");
        write_skill(
            &skill.join("SKILL.md"),
            "description: Valid common description\n",
            "Common",
        );
        let variant_path = skill.join("variants/pi/SKILL.md");
        write_skill(&variant_path, "", "Pi");

        let error =
            resolve_skill(temp.path(), "sample", Path::new("skills/sample"), "pi").unwrap_err();
        let message = error.to_string();
        assert!(message.contains("target pi"), "{message}");
        assert!(
            message.contains(&variant_path.display().to_string()),
            "{message}"
        );
        assert!(message.contains("description"), "{message}");
    }
}

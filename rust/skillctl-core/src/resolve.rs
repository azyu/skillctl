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
    let target_name = read_frontmatter_name(&source_dir.join("SKILL.md"))?;
    Ok(ResolvedSkill {
        id: id.to_string(),
        target: target.to_string(),
        target_name,
        package_dir,
        source_dir,
        fallback_used,
    })
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
}

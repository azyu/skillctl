use crate::error::{Result, SkillctlError};
use crate::resolve::ResolvedSkill;
use std::fs;
use std::path::{Path, PathBuf};

pub fn render_skill(_root: &Path, rendered_root: &Path, skill: &ResolvedSkill) -> Result<PathBuf> {
    let rendered = rendered_root.join(&skill.target).join(&skill.target_name);
    if rendered.exists() {
        fs::remove_dir_all(&rendered).map_err(|source| SkillctlError::Fs {
            path: rendered.clone(),
            source,
        })?;
    }
    fs::create_dir_all(&rendered).map_err(|source| SkillctlError::Fs {
        path: rendered.clone(),
        source,
    })?;
    copy_file(
        &skill.source_dir.join("SKILL.md"),
        &rendered.join("SKILL.md"),
    )?;
    copy_optional_dir(
        &skill.package_dir.join("references"),
        &rendered.join("references"),
    )?;
    copy_optional_dir(
        &skill.package_dir.join("scripts"),
        &rendered.join("scripts"),
    )?;
    Ok(rendered)
}

fn copy_optional_dir(source: &Path, dest: &Path) -> Result<()> {
    if !source.exists() {
        return Ok(());
    }
    copy_dir_recursive(source, dest)
}

fn copy_dir_recursive(source: &Path, dest: &Path) -> Result<()> {
    fs::create_dir_all(dest).map_err(|source_error| SkillctlError::Fs {
        path: dest.to_path_buf(),
        source: source_error,
    })?;
    for entry in fs::read_dir(source).map_err(|source_error| SkillctlError::Fs {
        path: source.to_path_buf(),
        source: source_error,
    })? {
        let entry = entry.map_err(|source_error| SkillctlError::Fs {
            path: source.to_path_buf(),
            source: source_error,
        })?;
        let source_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir_recursive(&source_path, &dest_path)?;
        } else {
            copy_file(&source_path, &dest_path)?;
        }
    }
    Ok(())
}

fn copy_file(source: &Path, dest: &Path) -> Result<()> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|source_error| SkillctlError::Fs {
            path: parent.to_path_buf(),
            source: source_error,
        })?;
    }
    fs::copy(source, dest).map_err(|source_error| SkillctlError::Fs {
        path: dest.to_path_buf(),
        source: source_error,
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::ResolvedSkill;
    use std::fs;

    #[test]
    fn builds_rendered_tree_with_shared_resources() {
        let temp = tempfile::tempdir().unwrap();
        let package = temp.path().join("skills/sample");
        let variant = package.join("variants/claude");
        fs::create_dir_all(&variant).unwrap();
        fs::create_dir_all(package.join("references")).unwrap();
        fs::write(
            variant.join("SKILL.md"),
            "---\nname: sample\ndescription: Claude\n---\nClaude\n",
        )
        .unwrap();
        fs::write(package.join("references/ref.md"), "reference").unwrap();

        let resolved = ResolvedSkill {
            id: "sample".to_string(),
            target: "claude".to_string(),
            target_name: "sample".to_string(),
            package_dir: package.clone(),
            source_dir: variant,
            fallback_used: false,
        };

        let rendered = render_skill(temp.path(), &temp.path().join("rendered"), &resolved).unwrap();
        assert_eq!(
            fs::read_to_string(rendered.join("SKILL.md")).unwrap(),
            "---\nname: sample\ndescription: Claude\n---\nClaude\n"
        );
        assert!(rendered.join("references/ref.md").exists());
    }
}

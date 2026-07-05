use crate::error::{Result, SkillctlError};
use crate::resolve::ResolvedSkill;
use sha2::{Digest, Sha256};
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

pub fn resolved_source_digest(skill: &ResolvedSkill) -> Result<String> {
    let mut files = Vec::new();
    let skill_file = skill.source_dir.join("SKILL.md");
    if skill_file.exists() {
        files.push((PathBuf::from("SKILL.md"), skill_file));
    }
    collect_files_with_prefix(
        &skill.package_dir.join("references"),
        Path::new("references"),
        &mut files,
    )?;
    collect_files_with_prefix(
        &skill.package_dir.join("scripts"),
        Path::new("scripts"),
        &mut files,
    )?;
    digest_named_files(files)
}

pub fn tree_digest(root: &Path) -> Result<String> {
    let mut files = Vec::new();
    collect_files(root, root, &mut files)?;
    let files = files
        .into_iter()
        .map(|relative| {
            let path = root.join(&relative);
            (relative, path)
        })
        .collect();
    digest_named_files(files)
}

fn digest_named_files(mut files: Vec<(PathBuf, PathBuf)>) -> Result<String> {
    files.sort_by(|left, right| left.0.cmp(&right.0));

    let mut hasher = Sha256::new();
    for (relative, path) in files {
        update_digest_part(&mut hasher, relative.to_string_lossy().as_bytes());
        let bytes = fs::read(&path).map_err(|source| SkillctlError::Fs { path, source })?;
        update_digest_part(&mut hasher, &bytes);
    }

    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn update_digest_part(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update(bytes.len().to_le_bytes());
    hasher.update(bytes);
}

fn collect_files(root: &Path, current: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(current).map_err(|source| SkillctlError::Fs {
        path: current.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| SkillctlError::Fs {
            path: current.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|source| SkillctlError::Fs {
            path: path.clone(),
            source,
        })?;
        if file_type.is_dir() {
            collect_files(root, &path, files)?;
        } else if file_type.is_file() {
            files.push(path.strip_prefix(root).unwrap().to_path_buf());
        }
    }
    Ok(())
}

fn collect_files_with_prefix(
    root: &Path,
    prefix: &Path,
    files: &mut Vec<(PathBuf, PathBuf)>,
) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }
    let mut relative_files = Vec::new();
    collect_files(root, root, &mut relative_files)?;
    for relative in relative_files {
        files.push((prefix.join(&relative), root.join(relative)));
    }
    Ok(())
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

    #[test]
    fn includes_package_level_scripts_as_shared_resources() {
        let temp = tempfile::tempdir().unwrap();
        let package = temp.path().join("skills/sample");
        let variant = package.join("variants/claude");
        fs::create_dir_all(&variant).unwrap();
        fs::create_dir_all(package.join("scripts")).unwrap();
        fs::write(
            variant.join("SKILL.md"),
            "---\nname: sample\ndescription: Claude\n---\nClaude\n",
        )
        .unwrap();
        fs::write(package.join("scripts/common.sh"), "echo common").unwrap();

        let resolved = ResolvedSkill {
            id: "sample".to_string(),
            target: "claude".to_string(),
            target_name: "sample".to_string(),
            package_dir: package,
            source_dir: variant,
            fallback_used: false,
        };

        let rendered = render_skill(temp.path(), &temp.path().join("rendered"), &resolved).unwrap();

        assert_eq!(
            fs::read_to_string(rendered.join("scripts/common.sh")).unwrap(),
            "echo common"
        );
    }

    #[test]
    fn tree_digest_is_deterministic_and_tracks_file_content() {
        let temp = tempfile::tempdir().unwrap();
        let tree = temp.path().join("tree");
        fs::create_dir_all(tree.join("nested")).unwrap();
        fs::write(tree.join("b.txt"), "b").unwrap();
        fs::write(tree.join("nested/a.txt"), "a").unwrap();

        let first = tree_digest(&tree).unwrap();
        let second = tree_digest(&tree).unwrap();
        assert_eq!(first, second);
        assert!(first.starts_with("sha256:"));

        fs::write(tree.join("nested/a.txt"), "changed").unwrap();
        let changed = tree_digest(&tree).unwrap();
        assert_ne!(first, changed);
    }

    #[test]
    fn resolved_source_digest_tracks_skill_and_shared_resources() {
        let temp = tempfile::tempdir().unwrap();
        let package = temp.path().join("skills/sample");
        let variant = package.join("variants/claude");
        fs::create_dir_all(&variant).unwrap();
        fs::create_dir_all(package.join("references")).unwrap();
        fs::write(variant.join("SKILL.md"), "---\nname: sample\n---\nClaude\n").unwrap();
        fs::write(package.join("references/ref.md"), "reference").unwrap();
        let resolved = ResolvedSkill {
            id: "sample".to_string(),
            target: "claude".to_string(),
            target_name: "sample".to_string(),
            package_dir: package.clone(),
            source_dir: variant,
            fallback_used: false,
        };

        let first = resolved_source_digest(&resolved).unwrap();
        fs::write(package.join("references/ref.md"), "changed").unwrap();
        let changed = resolved_source_digest(&resolved).unwrap();

        assert!(first.starts_with("sha256:"));
        assert_ne!(first, changed);
    }
}

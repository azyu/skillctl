use crate::error::{Result, SkillctlError};
use std::fs;
use std::path::Path;

#[cfg(unix)]
pub fn create_symlink_dir(source: &Path, link: &Path) -> Result<()> {
    if let Some(parent) = link.parent() {
        fs::create_dir_all(parent).map_err(|source_error| SkillctlError::Fs {
            path: parent.to_path_buf(),
            source: source_error,
        })?;
    }
    if link.exists() || link.symlink_metadata().is_ok() {
        fs::remove_file(link)
            .or_else(|_| fs::remove_dir_all(link))
            .map_err(|source_error| SkillctlError::Fs {
                path: link.to_path_buf(),
                source: source_error,
            })?;
    }
    std::os::unix::fs::symlink(source, link).map_err(|source_error| SkillctlError::Fs {
        path: link.to_path_buf(),
        source: source_error,
    })
}

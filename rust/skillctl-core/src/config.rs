use crate::error::{Result, SkillctlError};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Config {
    pub version: u32,
    pub targets: BTreeMap<String, TargetConfig>,
    #[serde(default)]
    pub policies: PolicyConfig,
    #[serde(default)]
    pub skills: BTreeMap<String, SkillConfig>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct TargetConfig {
    pub path: PathBuf,
    #[serde(default = "default_method")]
    pub method: MaterializeMethod,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MaterializeMethod {
    Symlink,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct PolicyConfig {
    #[serde(default = "default_unmanaged_conflict")]
    pub unmanaged_conflict: String,
    #[serde(default = "default_stale_managed_link")]
    pub stale_managed_link: String,
    #[serde(default = "default_missing_variant")]
    pub missing_variant: String,
    #[serde(default = "default_duplicate_name")]
    pub duplicate_name: String,
    #[serde(default = "default_name_mismatch")]
    pub name_mismatch: String,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            unmanaged_conflict: default_unmanaged_conflict(),
            stale_managed_link: default_stale_managed_link(),
            missing_variant: default_missing_variant(),
            duplicate_name: default_duplicate_name(),
            name_mismatch: default_name_mismatch(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct SkillConfig {
    pub path: PathBuf,
    #[serde(default)]
    pub expose: Vec<String>,
}

impl Config {
    pub fn load_from(path: &Path, home: &Path) -> Result<Self> {
        let text = fs::read_to_string(path).map_err(|source| SkillctlError::Fs {
            path: path.to_path_buf(),
            source,
        })?;
        let mut config: Config = serde_yaml::from_str(&text)
            .map_err(|error| SkillctlError::Config(error.to_string()))?;
        for target in config.targets.values_mut() {
            target.path = expand_home(&target.path, home);
        }
        Ok(config)
    }

    pub fn default_for_home(home: &Path) -> Self {
        let mut targets = BTreeMap::new();
        targets.insert(
            "claude".to_string(),
            TargetConfig {
                path: home.join(".claude/skills"),
                method: MaterializeMethod::Symlink,
                enabled: true,
            },
        );
        targets.insert(
            "codex".to_string(),
            TargetConfig {
                path: home.join(".agents/skills"),
                method: MaterializeMethod::Symlink,
                enabled: true,
            },
        );
        Self {
            version: 1,
            targets,
            policies: PolicyConfig::default(),
            skills: BTreeMap::new(),
        }
    }
}

fn expand_home(path: &Path, home: &Path) -> PathBuf {
    let raw = path.to_string_lossy();
    if raw == "~" {
        return home.to_path_buf();
    }
    if let Some(rest) = raw.strip_prefix("~/") {
        return home.join(rest);
    }
    path.to_path_buf()
}

fn default_method() -> MaterializeMethod {
    MaterializeMethod::Symlink
}

fn default_true() -> bool {
    true
}

fn default_unmanaged_conflict() -> String {
    "error".to_string()
}

fn default_stale_managed_link() -> String {
    "remove".to_string()
}

fn default_missing_variant() -> String {
    "fallback_common".to_string()
}

fn default_duplicate_name() -> String {
    "error".to_string()
}

fn default_name_mismatch() -> String {
    "error".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn parses_canonical_root_and_targets() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path();
        let config_path = home.join(".skillctl/config.yaml");
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        fs::write(
            &config_path,
            r#"
version: 1
targets:
  claude:
    path: ~/.claude/skills
    method: symlink
    enabled: true
  codex:
    path: ~/.agents/skills
    method: symlink
    enabled: true
policies:
  unmanaged_conflict: error
  stale_managed_link: remove
  missing_variant: fallback_common
  duplicate_name: error
  name_mismatch: error
skills:
  sample:
    path: skills/sample
    expose: [claude, codex]
"#,
        )
        .unwrap();

        let config = Config::load_from(&config_path, home).unwrap();
        assert_eq!(config.version, 1);
        assert_eq!(config.targets["claude"].path, home.join(".claude/skills"));
        assert_eq!(config.targets["codex"].path, home.join(".agents/skills"));
        assert_eq!(config.skills["sample"].path, PathBuf::from("skills/sample"));
    }
}

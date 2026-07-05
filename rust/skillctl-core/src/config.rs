use crate::error::{Result, SkillctlError};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

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
        config.validate(&home.join(".skillctl"))?;
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
    fn validate(&self, root: &Path) -> Result<()> {
        if self.version != 1 {
            return Err(SkillctlError::Config(format!(
                "config version must be 1, found {}",
                self.version
            )));
        }
        validate_policy(
            "unmanaged_conflict",
            &self.policies.unmanaged_conflict,
            "error",
        )?;
        validate_policy(
            "stale_managed_link",
            &self.policies.stale_managed_link,
            "remove",
        )?;
        validate_policy(
            "missing_variant",
            &self.policies.missing_variant,
            "fallback_common",
        )?;
        validate_policy("duplicate_name", &self.policies.duplicate_name, "error")?;
        validate_policy("name_mismatch", &self.policies.name_mismatch, "error")?;

        for (skill_id, skill) in &self.skills {
            for target in &skill.expose {
                if !self.targets.contains_key(target) {
                    return Err(SkillctlError::Config(format!(
                        "skill {skill_id} exposes unknown expose target {target}"
                    )));
                }
            }

            let resolved = if skill.path.is_absolute() {
                normalize_lexical(&skill.path)
            } else {
                normalize_lexical(&root.join(&skill.path))
            };
            let normalized_root = normalize_lexical(root);
            if !resolved.starts_with(&normalized_root) {
                return Err(SkillctlError::Config(format!(
                    "skill {skill_id} path {} escapes {}",
                    skill.path.display(),
                    normalized_root.display()
                )));
            }
        }

        Ok(())
    }
}

fn validate_policy(name: &str, actual: &str, expected: &str) -> Result<()> {
    if actual == expected {
        return Ok(());
    }
    Err(SkillctlError::Config(format!(
        "policy {name} must be {expected}, found {actual}"
    )))
}

fn normalize_lexical(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
            Component::RootDir | Component::Prefix(_) => normalized.push(component.as_os_str()),
        }
    }
    normalized
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

    #[test]
    fn rejects_unsupported_config_version() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path();
        let config_path = write_config(
            home,
            r#"
version: 2
targets:
  claude:
    path: ~/.claude/skills
skills: {}
"#,
        );

        let error = Config::load_from(&config_path, home).unwrap_err();
        assert!(error.to_string().contains("version must be 1"));
    }

    #[test]
    fn rejects_expose_targets_not_declared_in_config() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path();
        let config_path = write_config(
            home,
            r#"
version: 1
targets:
  claude:
    path: ~/.claude/skills
skills:
  sample:
    path: skills/sample
    expose: [codex]
"#,
        );

        let error = Config::load_from(&config_path, home).unwrap_err();
        assert!(error.to_string().contains("unknown expose target"));
        assert!(error.to_string().contains("codex"));
    }

    #[test]
    fn rejects_policy_values_outside_v1_allowlist() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path();
        let config_path = write_config(
            home,
            r#"
version: 1
targets:
  claude:
    path: ~/.claude/skills
policies:
  unmanaged_conflict: warn
skills: {}
"#,
        );

        let error = Config::load_from(&config_path, home).unwrap_err();
        assert!(error.to_string().contains("unmanaged_conflict"));
        assert!(error.to_string().contains("error"));
    }

    #[test]
    fn rejects_skill_paths_that_escape_skillctl_root() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path();
        let config_path = write_config(
            home,
            r#"
version: 1
targets:
  claude:
    path: ~/.claude/skills
skills:
  sample:
    path: ../outside
    expose: [claude]
"#,
        );

        let error = Config::load_from(&config_path, home).unwrap_err();
        assert!(error.to_string().contains("escapes"));
        assert!(error.to_string().contains(".skillctl"));
    }

    fn write_config(home: &Path, text: &str) -> PathBuf {
        let config_path = home.join(".skillctl/config.yaml");
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        fs::write(&config_path, text).unwrap();
        config_path
    }
}

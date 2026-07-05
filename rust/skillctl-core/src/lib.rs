pub mod config;
pub mod doctor;
pub mod error;
pub mod fs;
pub mod lockfile;
pub mod model;
pub mod plan;
pub mod render;
pub mod resolve;

use std::path::{Path, PathBuf};

pub use error::{Result, SkillctlError};
pub use model::{CommandOutput, Request};

pub fn run(request: Request) -> Result<CommandOutput> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| SkillctlError::Config("HOME is not set".to_string()))?;
    let root = home.join(".skillctl");
    let config_path = root.join("config.yaml");
    let config = if config_path.exists() {
        config::Config::load_from(&config_path, &home)?
    } else {
        config::Config::default_for_home(&home)
    };

    match request {
        Request::Plan => run_plan(&home, &root, &config),
        Request::Apply => run_apply(&home, &root, &config),
        Request::Doctor => run_doctor(&home, &root, &config),
        Request::List => run_list(&config),
        Request::Prune => Ok(CommandOutput::success(
            "Prune is planned after apply lock orchestration.\n",
        )),
        Request::Unlink { skill, target } => Ok(CommandOutput::success(format!(
            "Unlink planned for {skill} target {target:?}.\n"
        ))),
    }
}

fn run_plan(_home: &Path, root: &Path, config: &config::Config) -> Result<CommandOutput> {
    let mut output = String::new();
    for (skill_id, skill) in &config.skills {
        for target_name in &skill.expose {
            let Some(target) = config.targets.get(target_name) else {
                continue;
            };
            if !target.enabled {
                continue;
            }
            let resolved = resolve::resolve_skill(root, skill_id, &skill.path, target_name)?;
            let rendered_path = root
                .join("rendered")
                .join(target_name)
                .join(&resolved.target_name);
            let target_path = target.path.join(&resolved.target_name);
            output.push_str(&format!(
                "CREATE {} -> {}\n",
                target_path.display(),
                rendered_path.display()
            ));
        }
    }
    if output.is_empty() {
        output.push_str("No changes.\n");
    }
    Ok(CommandOutput::success(output))
}

fn run_apply(_home: &Path, _root: &Path, _config: &config::Config) -> Result<CommandOutput> {
    Ok(CommandOutput::success(
        "Apply orchestration will execute rendered plans after Task 9 follow-up hardening.\n",
    ))
}

fn run_doctor(_home: &Path, root: &Path, config: &config::Config) -> Result<CommandOutput> {
    let inputs: Vec<_> = config
        .targets
        .iter()
        .map(|(name, target)| doctor::TargetHealthInput {
            target: name.clone(),
            source_root: root.join("skills"),
            target_root: target.path.clone(),
            lock_path: target.path.join(".skillctl.lock.json"),
        })
        .collect();
    let report = doctor::check(&inputs);
    Ok(CommandOutput {
        stdout: report.render(),
        stderr: String::new(),
        exit_code: report.exit_code(),
    })
}

fn run_list(config: &config::Config) -> Result<CommandOutput> {
    let mut output = String::new();
    for skill_id in config.skills.keys() {
        output.push_str(skill_id);
        output.push('\n');
    }
    if output.is_empty() {
        output.push_str("No skills configured.\n");
    }
    Ok(CommandOutput::success(output))
}

pub mod config;
pub mod doctor;
pub mod error;
pub mod fs;
pub mod lockfile;
pub mod model;
pub mod plan;
pub mod render;
pub mod resolve;

use crate::lockfile::{ManagedEntry, TargetLock};
use crate::plan::{DesiredLink, Plan, PlanOperation};
use crate::resolve::ResolvedSkill;
use std::collections::BTreeMap;
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
        Request::Plan => run_plan(&root, &config),
        Request::Apply => run_apply(&root, &config),
        Request::Doctor => run_doctor(&root, &config),
        Request::List => run_list(&config),
        Request::Prune => run_prune(&root, &config),
        Request::Unlink { skill, target } => run_unlink(&root, &config, &skill, target.as_deref()),
    }
}

fn run_plan(root: &Path, config: &config::Config) -> Result<CommandOutput> {
    let target_plans = build_target_plans(root, config, TargetSelection::EnabledOnly)?;
    let mut output = String::new();
    for target_plan in &target_plans {
        output.push_str(&render_plan_for_target(
            &target_plan.target_name,
            &target_plan.plan,
        ));
    }
    if output.is_empty() {
        output.push_str("No changes.\n");
    }
    let exit_code = if target_plans
        .iter()
        .any(|target_plan| target_plan.plan.has_errors())
    {
        1
    } else {
        0
    };
    Ok(CommandOutput {
        stdout: output,
        stderr: String::new(),
        exit_code,
    })
}

fn run_apply(root: &Path, config: &config::Config) -> Result<CommandOutput> {
    let mut target_plans = build_target_plans(root, config, TargetSelection::EnabledOnly)?;
    let errors = collect_plan_errors(&target_plans);
    if !errors.is_empty() {
        return Ok(CommandOutput::failure(errors, 1));
    }

    for target_plan in &target_plans {
        for resolved in &target_plan.resolved {
            render::render_skill(root, &root.join("rendered"), resolved)?;
        }
    }

    let mut output = String::new();
    for target_plan in &mut target_plans {
        plan::apply_plan(&target_plan.plan)?;
        update_lock_after_plan(
            &mut target_plan.lock,
            &target_plan.desired,
            &target_plan.plan,
        );
        target_plan.lock.write_to(&target_plan.lock_path)?;
        output.push_str(&render_plan_for_target(
            &target_plan.target_name,
            &target_plan.plan,
        ));
    }
    if output.is_empty() {
        output.push_str("No changes.\n");
    }
    Ok(CommandOutput::success(output))
}

fn run_doctor(root: &Path, config: &config::Config) -> Result<CommandOutput> {
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

fn run_prune(root: &Path, config: &config::Config) -> Result<CommandOutput> {
    let mut target_plans = build_target_plans(root, config, TargetSelection::AllConfigured)?;
    let errors = collect_plan_errors(&target_plans);
    if !errors.is_empty() {
        return Ok(CommandOutput::failure(errors, 1));
    }

    let mut output = String::new();
    for target_plan in &mut target_plans {
        let stale_plan = Plan {
            operations: target_plan
                .plan
                .operations
                .iter()
                .filter(|operation| matches!(operation, PlanOperation::RemoveStale { .. }))
                .cloned()
                .collect(),
            errors: Vec::new(),
        };
        if stale_plan.operations.is_empty() {
            continue;
        }
        plan::apply_plan(&stale_plan)?;
        for operation in &stale_plan.operations {
            target_plan.lock.managed.remove(operation.target_name());
        }
        target_plan.lock.write_to(&target_plan.lock_path)?;
        output.push_str(&render_plan_for_target(
            &target_plan.target_name,
            &stale_plan,
        ));
    }
    if output.is_empty() {
        output.push_str("No stale managed links.\n");
    }
    Ok(CommandOutput::success(output))
}

fn run_unlink(
    root: &Path,
    config: &config::Config,
    skill: &str,
    target_filter: Option<&str>,
) -> Result<CommandOutput> {
    if let Some(target) = target_filter {
        if !config.targets.contains_key(target) {
            return Ok(CommandOutput::failure(
                format!("unknown target {target}\n"),
                1,
            ));
        }
    }

    let mut targets = load_target_locks(root, config, TargetSelection::AllConfigured)?;
    let mut plans = Vec::new();
    let mut errors = Vec::new();
    for target_state in &targets {
        if target_filter.is_some_and(|filter| filter != target_state.target_name) {
            plans.push(Plan {
                operations: Vec::new(),
                errors: Vec::new(),
            });
            continue;
        }
        let mut operations = Vec::new();
        for (target_name, entry) in &target_state.lock.managed {
            if entry.skill_id == skill {
                match plan::validate_remove_stale_ownership(
                    &entry.target_path,
                    &entry.rendered_path,
                ) {
                    Ok(()) => operations.push(PlanOperation::RemoveStale {
                        target_name: target_name.clone(),
                        target_path: entry.target_path.clone(),
                        expected_rendered_path: entry.rendered_path.clone(),
                    }),
                    Err(error) => errors.push(error.to_string()),
                }
            }
        }
        plans.push(Plan {
            operations,
            errors: Vec::new(),
        });
    }
    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Ok(CommandOutput::failure(format_errors(&errors), 1));
    }

    let mut output = String::new();
    for (target_state, unlink_plan) in targets.iter_mut().zip(plans) {
        if unlink_plan.operations.is_empty() {
            continue;
        }
        plan::apply_plan(&unlink_plan)?;
        for operation in &unlink_plan.operations {
            target_state.lock.managed.remove(operation.target_name());
        }
        target_state.lock.write_to(&target_state.lock_path)?;
        output.push_str(&render_plan_for_target(
            &target_state.target_name,
            &unlink_plan,
        ));
    }
    if output.is_empty() {
        output.push_str("No matching managed links.\n");
    }
    Ok(CommandOutput::success(output))
}

#[derive(Debug, Clone, Copy)]
enum TargetSelection {
    EnabledOnly,
    AllConfigured,
}

struct TargetPlan {
    target_name: String,
    lock_path: PathBuf,
    lock: TargetLock,
    desired: Vec<DesiredLink>,
    resolved: Vec<ResolvedSkill>,
    plan: Plan,
}

struct TargetState {
    target_name: String,
    lock_path: PathBuf,
    lock: TargetLock,
}

fn build_target_plans(
    root: &Path,
    config: &config::Config,
    selection: TargetSelection,
) -> Result<Vec<TargetPlan>> {
    let mut plans = Vec::new();
    for (target_name, target) in selected_targets(config, selection) {
        let lock_path = target.path.join(".skillctl.lock.json");
        let lock =
            TargetLock::read_or_empty(&lock_path, target_name, &root.join("skills"), &target.path)?;
        let (desired, resolved) = desired_for_target(root, config, target_name, &target.path)?;
        let plan = plan::build_plan(&target.path, &lock, desired.clone())?;
        plans.push(TargetPlan {
            target_name: target_name.to_string(),
            lock_path,
            lock,
            desired,
            resolved,
            plan,
        });
    }
    Ok(plans)
}

fn load_target_locks(
    root: &Path,
    config: &config::Config,
    selection: TargetSelection,
) -> Result<Vec<TargetState>> {
    let mut targets = Vec::new();
    for (target_name, target) in selected_targets(config, selection) {
        let lock_path = target.path.join(".skillctl.lock.json");
        let lock =
            TargetLock::read_or_empty(&lock_path, target_name, &root.join("skills"), &target.path)?;
        targets.push(TargetState {
            target_name: target_name.to_string(),
            lock_path,
            lock,
        });
    }
    Ok(targets)
}

fn selected_targets(
    config: &config::Config,
    selection: TargetSelection,
) -> Vec<(&str, &config::TargetConfig)> {
    config
        .targets
        .iter()
        .filter(|(_, target)| match selection {
            TargetSelection::EnabledOnly => target.enabled,
            TargetSelection::AllConfigured => true,
        })
        .map(|(name, target)| (name.as_str(), target))
        .collect()
}

fn desired_for_target(
    root: &Path,
    config: &config::Config,
    target_name: &str,
    target_root: &Path,
) -> Result<(Vec<DesiredLink>, Vec<ResolvedSkill>)> {
    let mut desired = Vec::new();
    let mut resolved_skills = Vec::new();
    for (skill_id, skill) in &config.skills {
        if !skill.expose.iter().any(|exposed| exposed == target_name) {
            continue;
        }
        let resolved = resolve::resolve_skill(root, skill_id, &skill.path, target_name)?;
        let source_digest = render::resolved_source_digest(&resolved)?;
        let rendered_path = root
            .join("rendered")
            .join(target_name)
            .join(&resolved.target_name);
        let target_path = target_root.join(&resolved.target_name);
        desired.push(DesiredLink {
            skill_id: skill_id.clone(),
            target_name: resolved.target_name.clone(),
            target_path,
            rendered_path,
            source_path: resolved.source_dir.clone(),
            source_digest,
        });
        resolved_skills.push(resolved);
    }
    Ok((desired, resolved_skills))
}

fn update_lock_after_plan(lock: &mut TargetLock, desired: &[DesiredLink], plan: &Plan) {
    let desired_by_target: BTreeMap<_, _> = desired
        .iter()
        .map(|desired| (desired.target_name.clone(), desired))
        .collect();
    for operation in &plan.operations {
        match operation {
            PlanOperation::Link { target_name, .. } => {
                if let Some(desired) = desired_by_target.get(target_name) {
                    lock.managed.insert(
                        target_name.clone(),
                        ManagedEntry {
                            skill_id: desired.skill_id.clone(),
                            target_name: desired.target_name.clone(),
                            target_path: desired.target_path.clone(),
                            rendered_path: desired.rendered_path.clone(),
                            source_path: desired.source_path.clone(),
                            method: "symlink".to_string(),
                            source_digest: desired.source_digest.clone(),
                        },
                    );
                }
            }
            PlanOperation::RemoveStale { target_name, .. } => {
                lock.managed.remove(target_name);
            }
        }
    }
    for desired in desired {
        lock.managed
            .entry(desired.target_name.clone())
            .or_insert_with(|| ManagedEntry {
                skill_id: desired.skill_id.clone(),
                target_name: desired.target_name.clone(),
                target_path: desired.target_path.clone(),
                rendered_path: desired.rendered_path.clone(),
                source_path: desired.source_path.clone(),
                method: "symlink".to_string(),
                source_digest: desired.source_digest.clone(),
            });
    }
}

fn render_plan_for_target(target_name: &str, plan: &Plan) -> String {
    let mut output = String::new();
    for operation in &plan.operations {
        output.push_str(&format!(
            "{} {target_name}/{} -> {}\n",
            operation.label(),
            operation.target_name(),
            operation.target_path().display()
        ));
    }
    for error in &plan.errors {
        output.push_str(&format!("ERROR {target_name}: {error}\n"));
    }
    output
}

fn collect_plan_errors(target_plans: &[TargetPlan]) -> String {
    let errors: Vec<_> = target_plans
        .iter()
        .flat_map(|target_plan| {
            target_plan
                .plan
                .errors
                .iter()
                .map(|error| format!("{}: {error}", target_plan.target_name))
        })
        .collect();
    format_errors(&errors)
}

fn format_errors(errors: &[String]) -> String {
    let mut output = String::new();
    for error in errors {
        output.push_str("ERROR ");
        output.push_str(error);
        output.push('\n');
    }
    output
}

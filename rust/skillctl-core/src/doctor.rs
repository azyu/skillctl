use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetHealthInput {
    pub target: String,
    pub source_root: PathBuf,
    pub target_root: PathBuf,
    pub lock_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorReport {
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub target: String,
    pub path: PathBuf,
    pub message: String,
    pub hint: String,
}

impl DoctorReport {
    pub fn exit_code(&self) -> u8 {
        if self.diagnostics.is_empty() { 0 } else { 1 }
    }

    pub fn render(&self) -> String {
        if self.diagnostics.is_empty() {
            return "skillctl doctor: ok\n".to_string();
        }
        let mut output = String::new();
        for diagnostic in &self.diagnostics {
            output.push_str(&format!(
                "{}: {} at {}\n  hint: {}\n",
                diagnostic.target,
                diagnostic.message,
                diagnostic.path.display(),
                diagnostic.hint
            ));
        }
        output
    }
}

pub fn check(inputs: &[TargetHealthInput]) -> DoctorReport {
    let mut diagnostics = Vec::new();
    for input in inputs {
        if !input.source_root.exists() {
            diagnostics.push(Diagnostic {
                target: input.target.clone(),
                path: input.source_root.clone(),
                message: "missing source root".to_string(),
                hint: "create ~/.skillctl/skills or run skillctl init after Task 8 adds init"
                    .to_string(),
            });
        }
        if !input.target_root.exists() {
            diagnostics.push(Diagnostic {
                target: input.target.clone(),
                path: input.target_root.clone(),
                message: "missing target root".to_string(),
                hint: "create the target directory or disable the target in config.yaml"
                    .to_string(),
            });
        }
    }
    DoctorReport { diagnostics }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_broken_root_lock_and_conflict_states() {
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("missing");
        let report = check(&[TargetHealthInput {
            target: "claude".to_string(),
            source_root: missing.clone(),
            target_root: temp.path().join("target"),
            lock_path: temp.path().join("target/.skillctl.lock.json"),
        }]);
        assert!(!report.diagnostics.is_empty());
        assert!(
            report.diagnostics[0]
                .message
                .contains("missing source root")
        );
        assert_eq!(report.exit_code(), 1);
    }

    #[test]
    fn returns_clean_status_for_healthy_tree() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let target = temp.path().join("target");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::create_dir_all(&target).unwrap();
        let report = check(&[TargetHealthInput {
            target: "claude".to_string(),
            source_root: source,
            target_root: target.clone(),
            lock_path: target.join(".skillctl.lock.json"),
        }]);
        assert!(report.diagnostics.is_empty());
        assert_eq!(report.exit_code(), 0);
    }
}

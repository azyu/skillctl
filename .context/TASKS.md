---
created: 2026-07-05
updated: 2026-07-05
---
# TASKS

## Active Phase

| Phase | Status | Evidence |
|---|---|---|
| Planning and context bootstrap | Complete | `.context/PROJECT.md`, `.context/STEERING.md`, `.context/TASKS.md`, `AGENTS.md`, and `docs/superpowers/plans/2026-07-05-skillctl.md` were written. |
| Rust implementation | In progress | Initial Rust workspace and command orchestration completed; full lock-backed `apply`, `prune`, and `unlink` hardening remains pending. `cargo test --manifest-path rust/Cargo.toml --all` passed 12 tests on 2026-07-05. |

## Completed Work

- [x] Decided tool name: `skillctl`.
- [x] Decided implementation language: Rust.
- [x] Decided source/state root: `~/.skillctl/`.
- [x] Decided `~/.agents/skills` remains the Codex target, not the SOT.
- [x] Reviewed reference repository surfaces for `/Volumes/EXTSSD/code/personal/bitbucket-cli` and `/Volumes/EXTSSD/code/personal/tossinvest-cli`.
- [x] Created implementation plan at `docs/superpowers/plans/2026-07-05-skillctl.md`.
- [x] Bootstrapped Rust workspace and CLI shell under `rust/`.
- [x] Implemented YAML config loading and `~` target path expansion.
- [x] Implemented target variant resolution with common skill fallback.
- [x] Implemented rendered skill tree construction with shared resources.
- [x] Implemented per-target `.skillctl.lock.json` ownership validation and managed entries.
- [x] Implemented deterministic plan operations with unmanaged conflict reporting.
- [x] Implemented apply abort-on-error behavior and filesystem symlink helper.
- [x] Implemented doctor diagnostics and expanded CLI smoke coverage.
- [x] Wired initial `plan`, `doctor`, `list`, `apply`, `prune`, and `unlink` command outputs through core orchestration.

## Pending Observable Work

- [x] Create Rust workspace under `rust/` with `skillctl-core` and `skillctl-cli`.
- [x] Implement config loading from `~/.skillctl/config.yaml`.
- [x] Implement skill package validation and target variant resolution.
- [x] Implement rendered package construction under `~/.skillctl/rendered/`.
- [x] Implement per-target `.skillctl.lock.json` read/write and ownership validation.
- [x] Implement deterministic `plan` behavior.
- [x] Implement mutation-safe `apply` behavior.
- [x] Implement initial `doctor`, `list`, `prune`, and `unlink` command surfaces.
- [x] Add CLI smoke tests and core behavior tests.
- [x] Run focused verification commands and record observed results here.
- [ ] Fully wire lock-backed `apply`, `prune`, and `unlink` mutations before release.

## Verification Results

- 2026-07-05: Task 1 bootstrap verification passed:
  - `cargo fmt --manifest-path rust/Cargo.toml --all -- --check`
  - `cargo test --manifest-path rust/Cargo.toml -p skillctl-cli --test smoke` (1 test passed)
  - `cargo run --manifest-path rust/Cargo.toml -p skillctl-cli --bin skillctl -- --help` (help output includes `plan`, `apply`, and `doctor`)
- 2026-07-05: Task 2 config verification passed:
  - RED: `cargo test --manifest-path rust/Cargo.toml -p skillctl-core config::tests::parses_canonical_root_and_targets` failed because `Config` was undefined.
  - GREEN: `cargo test --manifest-path rust/Cargo.toml -p skillctl-core config::tests::parses_canonical_root_and_targets` passed (1 test passed).
  - `cargo fmt --manifest-path rust/Cargo.toml --all -- --check` passed after formatting.
- 2026-07-05: Task 3 resolver verification passed:
  - RED: `cargo test --manifest-path rust/Cargo.toml -p skillctl-core resolve::tests::resolves_target_variant_before_default` failed because `resolve_skill` was undefined.
  - GREEN: `cargo test --manifest-path rust/Cargo.toml -p skillctl-core resolve::tests::resolves_target_variant_before_default` passed (1 test passed).
  - `cargo fmt --manifest-path rust/Cargo.toml --all -- --check` passed.
- 2026-07-05: Task 4 render verification passed:
  - RED: `cargo test --manifest-path rust/Cargo.toml -p skillctl-core render::tests::builds_rendered_tree_with_shared_resources` failed because `render_skill` was undefined.
  - GREEN: `cargo test --manifest-path rust/Cargo.toml -p skillctl-core render::tests::builds_rendered_tree_with_shared_resources` passed (1 test passed).
  - `cargo fmt --manifest-path rust/Cargo.toml --all -- --check` passed after formatting.
- 2026-07-05: Task 5 lockfile verification passed:
  - RED: `cargo test --manifest-path rust/Cargo.toml -p skillctl-core lockfile::tests` failed because `TargetLock` and `ManagedEntry` were undefined.
  - GREEN: `cargo test --manifest-path rust/Cargo.toml -p skillctl-core lockfile::tests` passed (2 tests passed).
  - `cargo fmt --manifest-path rust/Cargo.toml --all -- --check` passed after formatting.
- 2026-07-05: Task 6 plan verification passed:
  - RED: `cargo test --manifest-path rust/Cargo.toml -p skillctl-core plan::tests::plan_distinguishes_managed_updates_from_unmanaged_conflicts` failed because `DesiredLink` and `build_plan` were undefined.
  - GREEN: `cargo test --manifest-path rust/Cargo.toml -p skillctl-core plan::tests::plan_distinguishes_managed_updates_from_unmanaged_conflicts` passed (1 test passed).
  - `cargo fmt --manifest-path rust/Cargo.toml --all -- --check` passed after formatting.
- 2026-07-05: Task 7 apply verification passed:
  - RED: `cargo test --manifest-path rust/Cargo.toml -p skillctl-core plan::tests::apply_aborts_on_unmanaged_conflict_without_mutation` failed because `apply_plan` was undefined.
  - GREEN: `cargo test --manifest-path rust/Cargo.toml -p skillctl-core plan::tests::apply_aborts_on_unmanaged_conflict_without_mutation` passed (1 test passed).
  - `cargo fmt --manifest-path rust/Cargo.toml --all -- --check` passed.
- 2026-07-05: Task 8 doctor and smoke verification passed:
  - RED: `cargo test --manifest-path rust/Cargo.toml -p skillctl-core doctor::tests` failed because `TargetHealthInput` and `check` were undefined.
  - GREEN: `cargo test --manifest-path rust/Cargo.toml -p skillctl-core doctor::tests` passed (2 tests passed).
  - `cargo test --manifest-path rust/Cargo.toml -p skillctl-cli --test smoke` passed (2 tests passed).
  - `cargo fmt --manifest-path rust/Cargo.toml --all -- --check` passed after formatting.
- 2026-07-05: Task 9 initial orchestration verification passed:
  - RED: `cargo test --manifest-path rust/Cargo.toml -p skillctl-cli --test smoke plan_reads_home_scoped_config_fixture` failed because `plan` printed the placeholder `"No config loaded yet. Task 2 implements planning."`.
  - GREEN: `cargo test --manifest-path rust/Cargo.toml -p skillctl-cli --test smoke plan_reads_home_scoped_config_fixture` passed (1 test passed).
  - `cargo test --manifest-path rust/Cargo.toml --all` passed (12 tests across 4 suites).
  - `cargo fmt --manifest-path rust/Cargo.toml --all -- --check` passed.
  - `cargo run --manifest-path rust/Cargo.toml -p skillctl-cli --bin skillctl -- --help` printed `plan`, `apply`, `doctor`, `list`, `prune`, and `unlink`.

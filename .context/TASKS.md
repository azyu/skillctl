---
created: 2026-07-05
updated: 2026-07-05
---
# TASKS

## Active Phase

| Phase | Status | Evidence |
|---|---|---|
| Planning and context bootstrap | Complete | `.context/PROJECT.md`, `.context/STEERING.md`, `.context/TASKS.md`, `AGENTS.md`, and `docs/superpowers/plans/2026-07-05-skillctl.md` were written. |
| Rust implementation | In progress | Initial Rust workspace and command orchestration completed; release binary copied to `~/.local/bin/skillctl`; full lock-backed `apply`, `prune`, and `unlink` hardening remains pending. `cargo test --manifest-path rust/Cargo.toml --all` passed 12 tests on 2026-07-05. |

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
- [x] Built release binary and copied it to `~/.local/bin/skillctl`.

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
- [ ] Update `.context/PROJECT.md` to reflect implemented Rust workspace and remaining blockers.
- [ ] Replace stale doctor hint that mentions nonexistent `skillctl init`.
- [ ] Add config validation for `version == 1`.
- [ ] Add config validation for unknown `skills.*.expose` targets.
- [ ] Add config validation for allowed policy values.
- [ ] Add config validation preventing skill paths from escaping `~/.skillctl/`.
- [ ] Define and test rendered package inclusion rules.
- [ ] Implement deterministic source/rendered tree digest calculation.
- [ ] Store real `source_digest` values in lockfile entries.
- [ ] Compare lockfile digests when planning managed updates.
- [ ] Add ownership check for managed target symlinks before replacement.
- [ ] Add ownership check for stale managed target removal.
- [ ] Extend `PlanOperation::RemoveStale` with expected rendered path ownership data.
- [ ] Extend `PlanOperation::Link` or planning context with previous managed ownership data.
- [ ] Refactor shared target planning context for `plan`, `apply`, `prune`, and `unlink`.
- [ ] Wire `run_plan` through lockfile loading and `build_plan`.
- [ ] Make `run_plan` report unmanaged conflicts from target directories.
- [ ] Make `run_plan` report stale managed entries.
- [ ] Make `run_plan` distinguish `CREATE`, `UPDATE`, `REMOVE_STALE`, and `ERROR`.
- [ ] Decide and implement non-zero `plan` exit code when plan errors exist.
- [ ] Wire `run_apply` to resolve skills, render packages, build plans, and apply operations.
- [ ] Make `run_apply` abort before mutation when plan errors exist.
- [ ] Make `run_apply` write updated per-target `.skillctl.lock.json` files.
- [ ] Make `run_apply` summarize applied operations.
- [ ] Implement lock-backed `run_prune`.
- [ ] Make `run_prune` remove only lockfile-managed stale symlinks.
- [ ] Make `run_prune` refuse drifted or unmanaged paths.
- [ ] Make `run_prune` update lockfiles after removals.
- [ ] Implement lock-backed `run_unlink`.
- [ ] Make `run_unlink` support optional `--target` filtering.
- [ ] Make `run_unlink` remove only matching lockfile-managed symlinks.
- [ ] Make `run_unlink` update lockfiles after removals.
- [ ] Extend `doctor` to read and validate target lockfiles.
- [ ] Extend `doctor` to report foreign lockfile owners.
- [ ] Extend `doctor` to report missing managed target paths.
- [ ] Extend `doctor` to report non-symlink managed target paths.
- [ ] Extend `doctor` to report managed symlink target mismatches.
- [ ] Extend `doctor` to report missing rendered paths.
- [ ] Extend `doctor` to report unmanaged target conflicts.
- [ ] Add CLI E2E test: `apply` creates rendered directory, target symlink, and lockfile entry.
- [ ] Add CLI E2E test: `apply` aborts before mutation on unmanaged conflict.
- [ ] Add CLI E2E test: `apply` refuses drifted managed target paths.
- [ ] Add CLI E2E test: `prune` removes only lockfile-managed stale symlink.
- [ ] Add CLI E2E test: `prune` refuses unmanaged regular files.
- [ ] Add CLI E2E test: `unlink <skill> --target <target>` removes only one managed entry.
- [ ] Add CLI E2E test: `doctor` reports lockfile owner mismatch.
- [ ] Add CLI E2E test: `doctor` reports broken or drifted managed symlink.
- [ ] Add CLI E2E test: `plan` reports unmanaged conflict.
- [ ] Add CLI E2E test: `list` covers empty and configured skills.
- [ ] Run full release verification after mutation hardening.
- [ ] Rebuild release binary and copy updated `skillctl` to `~/.local/bin/skillctl`.

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
- 2026-07-05: Git and local release install verification passed:
  - `git log --oneline --decorate -5` showed logical commits ending at `2a5695b (HEAD -> main) feat: wire initial skillctl commands`.
  - `cargo build --manifest-path rust/Cargo.toml -p skillctl-cli --bin skillctl --release` succeeded.
  - Copied `rust/target/release/skillctl` to `~/.local/bin/skillctl`.
  - `~/.local/bin/skillctl --help` printed `plan`, `apply`, `doctor`, `list`, `prune`, and `unlink`.
  - `cargo test --manifest-path rust/Cargo.toml --all` passed (12 tests across 4 suites).
  - `git status --short` was clean before recording this context update.

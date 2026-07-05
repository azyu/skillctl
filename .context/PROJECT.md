---
created: 2026-07-05
updated: 2026-07-05
---
# skillctl

`skillctl` is a planned Rust CLI that materializes a canonical Agent Skills library into runtime-specific skill directories for Claude Code, Codex, OMP, and future targets.

## Current State

The project directory has no implemented code yet. This session established the durable direction and wrote the first implementation plan.

- Implementation plan: `docs/superpowers/plans/2026-07-05-skillctl.md`
- Reference repository: `/Volumes/EXTSSD/code/personal/bitbucket-cli`
- Reference repository: `/Volumes/EXTSSD/code/personal/tossinvest-cli`

## Planned Capabilities

- Use `~/.skillctl/` as the tool-owned source and state root.
- Store canonical skill packages under `~/.skillctl/skills/`.
- Use `~/.skillctl/config.yaml` for targets, policies, and skill exposure.
- Render complete target-specific skill packages under `~/.skillctl/rendered/<target>/<skill>/`.
- Symlink rendered packages into target skill directories such as `~/.claude/skills` and `~/.agents/skills`.
- Track tool-owned target entries with per-target `.skillctl.lock.json` files.
- Provide `plan`, `apply`, `doctor`, `list`, `prune`, and `unlink` commands.

## Not Yet Implemented

- Rust workspace scaffold.
- Config parser and schema validation.
- Skill package scanner and target variant resolver.
- Rendered directory builder.
- Lockfile ownership and drift detection.
- Plan/apply/doctor command behavior.
- CLI smoke tests and core behavior tests.

## Planned Tech Stack

- Rust edition 2024.
- Two-crate workspace under `rust/`: `skillctl-core` and `skillctl-cli`.
- `clap` for CLI parsing.
- `serde`, `serde_yaml`, and `serde_json` for config and lockfiles.
- `thiserror` for core errors.
- `assert_cmd` and `tempfile` for tests.

## Verification Commands

No commands pass yet because the Rust workspace does not exist. Planned commands after scaffold:

```bash
cargo fmt --manifest-path rust/Cargo.toml --all -- --check
cargo test --manifest-path rust/Cargo.toml --all
cargo run --manifest-path rust/Cargo.toml -p skillctl-cli --bin skillctl -- --help
```

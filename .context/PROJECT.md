---
created: 2026-07-05
updated: 2026-07-05
---
# skillctl

`skillctl` is a Rust CLI that materializes a canonical Agent Skills library into runtime-specific skill directories for Claude Code, Codex, OMP, and future targets.

## Current State

The Rust workspace is implemented under `rust/` with a two-crate split:

- `rust/skillctl-core`: config loading and validation, skill resolution, rendering, deterministic digests, lockfiles, planning, applying, pruning, unlinking, and doctor diagnostics.
- `rust/skillctl-cli`: clap command parsing, binary entrypoint, process output, and CLI smoke/E2E coverage.

The current release binary has been built and copied to `~/.local/bin/skillctl`.

Durable implementation plan: `docs/superpowers/plans/2026-07-05-skillctl.md`.

## Implemented Capabilities

- Uses `~/.skillctl/` as the tool-owned source and state root.
- Stores canonical skill packages under `~/.skillctl/skills/`.
- Loads YAML config from `~/.skillctl/config.yaml`.
- Validates config version `1`, known `skills.*.expose` targets, v1 policy values, and skill paths that must remain inside `~/.skillctl/`.
- Resolves target variants with common `SKILL.md` fallback.
- Renders selected `SKILL.md` plus package-level `references/` and `scripts/` into `~/.skillctl/rendered/<target>/<skill>/`.
- Computes deterministic SHA-256 digests for rendered/source input trees and stores real `source_digest` values in per-target lockfile entries.
- Symlinks rendered packages into target directories such as `~/.claude/skills` and `~/.agents/skills`.
- Tracks managed target entries in `.skillctl.lock.json` inside each target skill directory.
- Reports deterministic `plan` operations: `CREATE`, `UPDATE`, `REMOVE_STALE`, and `ERROR`.
- Exits non-zero when `plan` contains plan errors.
- Aborts `apply` before mutation when plan errors exist.
- Applies lock-backed create/update/remove operations and writes updated per-target lockfiles.
- Implements lock-backed `prune` and `unlink`, including `unlink <skill> --target <target>` filtering.
- Extends `doctor` to report foreign lockfile owners, missing managed paths, non-symlink managed paths, symlink target mismatches, missing rendered paths, and unmanaged target conflicts.
- Provides CLI E2E coverage for `apply`, `prune`, `unlink`, `doctor`, `plan`, and `list` behavior.

## Plan Error Policy

- `skillctl plan` exits with code `1` when plan errors exist. It still prints `ERROR` rows to make the blocking paths visible to humans and CI logs.

## Planned Tech Stack

- Rust edition 2024.
- Two-crate workspace under `rust/`: `skillctl-core` and `skillctl-cli`.
- `clap` for CLI parsing.
- `serde`, `serde_yaml`, and `serde_json` for config and lockfiles.
- `sha2` for deterministic digests.
- `thiserror` for core errors.
- `assert_cmd`, `predicates`, and `tempfile` for tests.

## Verification Commands

Latest observed verification on 2026-07-05:

```bash
cargo fmt --manifest-path rust/Cargo.toml --all -- --check
cargo test --manifest-path rust/Cargo.toml --all
cargo run --manifest-path rust/Cargo.toml -p skillctl-cli --bin skillctl -- --help
cargo build --manifest-path rust/Cargo.toml -p skillctl-cli --bin skillctl --release
~/.local/bin/skillctl --help
```

Observed results:

- `cargo fmt --manifest-path rust/Cargo.toml --all -- --check` passed.
- `cargo test --manifest-path rust/Cargo.toml --all` passed: 36 tests across 4 suites.
- `cargo run --manifest-path rust/Cargo.toml -p skillctl-cli --bin skillctl -- --help` printed `plan`, `apply`, `doctor`, `list`, `prune`, and `unlink`.
- `cargo build --manifest-path rust/Cargo.toml -p skillctl-cli --bin skillctl --release` succeeded.
- `~/.local/bin/skillctl --help` printed `plan`, `apply`, `doctor`, `list`, `prune`, and `unlink`.
- Installed binary conflict smoke test passed: with a temporary `HOME` containing an unmanaged target conflict, `/Users/azyu/.local/bin/skillctl plan` exited `1` and printed `ERROR ... unmanaged conflict`.

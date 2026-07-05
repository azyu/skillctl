# AGENTS.md

## Project

`skillctl` is a planned Rust CLI that materializes a canonical Agent Skills library from `~/.skillctl/` into runtime-specific skill directories such as Claude Code, Codex, and OMP.

## Required Context Intake

Before starting any task, read these files in order:

1. `.context/PROJECT.md` — current project summary and active state.
2. `.context/STEERING.md` — active priorities, constraints, and decision log.
3. `.context/TASKS.md` — current status board.

The `.context` directory is the lightweight coordination layer for future sessions and agents. Keep it current when task status changes, but do not duplicate README, full specs, or implementation plans.

## Durable Documents

- Implementation plan: `docs/superpowers/plans/2026-07-05-skillctl.md`
- Reference repository: `/Volumes/EXTSSD/code/personal/bitbucket-cli`
- Reference repository: `/Volumes/EXTSSD/code/personal/tossinvest-cli`

## Architecture Direction

Use a two-crate Rust workspace:

```text
rust/
├── skillctl-core/   # config, skill validation, rendering, plan/apply/doctor, lockfiles
└── skillctl-cli/    # clap parser, command dispatch, process output, binary behavior
```

Follow the reference repositories for useful CLI discipline only:

- `bitbucket-cli`: Rust workspace rooted at `rust/`, CLI crate plus core crate, CLI smoke tests.
- `tossinvest-cli`: context intake style, Rust 2024, explicit command/runtime split.

Do not copy domain concepts from either reference repository.

## Implementation Rules

- Keep `~/.skillctl/` as the only canonical source/state root.
- Treat `~/.agents/skills` as a Codex target, not the source of truth.
- Treat `~/.claude/skills` as a Claude target, not the source of truth.
- Use YAML config only for v1: `~/.skillctl/config.yaml`.
- Use per-target `.skillctl.lock.json` lockfiles.
- Default to symlink materialization.
- Abort on unmanaged conflicts before mutating files.
- Remove only target paths recorded in the lockfile and still matching expected ownership.
- Keep remote install, marketplaces, GUI, YAML/TOML dual support, and patch-style overlays out of v1.

## Verification

Before claiming behavior is complete, run focused verification for the changed area and record the observed result in `.context/TASKS.md`.

Planned commands after Rust scaffold exists:

```bash
cargo fmt --manifest-path rust/Cargo.toml --all -- --check
cargo test --manifest-path rust/Cargo.toml --all
cargo run --manifest-path rust/Cargo.toml -p skillctl-cli --bin skillctl -- --help
```

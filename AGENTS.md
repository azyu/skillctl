# AGENTS.md

## Project

`skillctl` is a Rust CLI that materializes a canonical Agent Skills library from `~/.skillctl/` into runtime-specific skill directories such as Claude Code, Codex, and OMP.

It owns source and state under `~/.skillctl/`, renders target-specific complete skill trees under `~/.skillctl/rendered/`, and symlinks those rendered trees into runtime target directories.

## Project Structure

Current repository state:

```text
README.md                         # user-facing overview, quick start, command guide
AGENTS.md                         # agent/contributor workflow and constraints
.context/
  PROJECT.md                      # current project state and verification summary
  STEERING.md                     # durable constraints and decisions
  TASKS.md                        # task board and observed verification results
docs/superpowers/plans/
  2026-07-05-skillctl.md          # implementation plan/history
rust/
  Cargo.toml                      # Rust workspace manifest
  skillctl-core/                  # config, validation, rendering, planning, applying, lockfiles, doctor
  skillctl-cli/                   # clap parser, binary entrypoint, CLI smoke/E2E tests
```

## Required Context Intake

Before starting any task, read these files in order:

1. `.context/PROJECT.md` — current project summary and active state.
2. `.context/STEERING.md` — active priorities, constraints, and decision log.
3. `.context/TASKS.md` — current status board.

The `.context` directory is the lightweight coordination layer for future sessions and agents. Keep it current when task status changes, but do not duplicate README, full specs, or implementation plans.

## Durable Documents

- User README: `README.md`
- Implementation plan/history: `docs/superpowers/plans/2026-07-05-skillctl.md`
- Reference repository: `/Volumes/EXTSSD/code/personal/bitbucket-cli`
- Reference repository: `/Volumes/EXTSSD/code/personal/tossinvest-cli`

Document roles:

- `README.md` is for users: install, quick start, commands, safety model.
- `AGENTS.md` is for coding agents and contributors working in this repository.
- `.context/PROJECT.md` is the compact current-state summary.
- `.context/STEERING.md` is the durable constraint and decision log.
- `.context/TASKS.md` is the task/verification ledger.
- `docs/superpowers/plans/2026-07-05-skillctl.md` is the historical implementation plan, not a replacement for current context intake.

## Architecture Direction

Use a two-crate Rust workspace:

```text
rust/
├── skillctl-core/   # config, validation, rendering, plan/apply/prune/unlink/doctor, lockfiles
└── skillctl-cli/    # clap parser, command dispatch, process output, binary behavior
```

Follow the reference repositories for useful CLI discipline only:

- `bitbucket-cli`: Rust workspace rooted at `rust/`, CLI crate plus core crate, CLI smoke tests, agent coordination notes.
- `tossinvest-cli`: context intake style, Rust 2024, explicit command/runtime split, verification discipline.

Do not copy domain concepts from either reference repository.

## Implementation Rules

- Keep `~/.skillctl/` as the only canonical source/state root.
- Treat `~/.skillctl/skills/` as the canonical skill source.
- Treat `~/.skillctl/rendered/` as generated output, not source of truth.
- Treat `~/.agents/skills` as a Codex target, not the source of truth.
- Treat `~/.claude/skills` as a Claude target, not the source of truth.
- Use YAML config only for v1: `~/.skillctl/config.yaml`.
- Use per-target `.skillctl.lock.json` lockfiles inside target skill directories.
- Default to symlink materialization.
- Abort on unmanaged conflicts before mutating files.
- Replace managed targets only when the existing path is missing or still points to the lockfile's expected rendered path.
- Remove only target paths recorded in the lockfile and still matching expected ownership.
- Keep remote install, marketplaces, GUI, YAML/TOML dual support, copy mode, and patch-style overlays out of v1 unless explicitly approved.

## Command Surface

Current user-facing commands are:

- `skillctl list`
- `skillctl plan`
- `skillctl apply`
- `skillctl doctor`
- `skillctl prune`
- `skillctl unlink <skill>`
- `skillctl unlink <skill> --target <target>`

Do not document or implement extra commands such as `init`, remote install, marketplace, or completion unless the task explicitly changes scope.

`skillctl plan` prints deterministic operation rows and exits with code `1` when any `ERROR` row exists.

## Code Standards

### Do

- Keep changes directly tied to the current task.
- Prefer the smallest explicit Rust implementation that satisfies requirements.
- Keep `skillctl-core` responsible for filesystem state, config parsing, validation, rendering, planning, applying, lockfiles, and diagnostics.
- Keep `skillctl-cli` responsible for clap parsing, process exit codes, and human-readable output.
- Exercise core behavior through public core functions and CLI behavior through the compiled binary.
- For bug fixes, reproduce the behavior with a failing test before implementing the fix.
- Prefer focused crate/package tests before the full workspace when iterating.
- Update `.context/TASKS.md` when a task starts, completes, or verification results change.
- Update `.context/STEERING.md` only for durable decisions that should affect future sessions.
- Update `.context/PROJECT.md` when project state changes materially.
- Keep README, context files, and command behavior aligned when user-facing behavior changes.

### Don't

- Do not silently assume requirements when multiple interpretations exist; surface the decision.
- Do not copy Bitbucket, Toss, brokerage, auth, HTTP, API pagination, or release concepts from reference repositories.
- Do not introduce abstractions before a clear second use case exists.
- Do not change unrelated files or formatting.
- Do not claim completion without fresh verification evidence.
- Do not leave aliases, stale docs, or placeholders for removed behavior.
- Do not commit secrets, credentials, tokens, private skill payloads, or generated local state from `~/.skillctl/`, `~/.claude/skills`, or `~/.agents/skills`.

## Secrets and Local State

- Treat canonical skill packages as potentially private user data.
- Keep `~/.skillctl/config.yaml`, rendered trees, target lockfiles, and runtime target directories out of the repository unless a test fixture explicitly needs a minimal synthetic sample.
- Never hardcode personal paths, tokens, credentials, or private skill contents in source code, README examples, or tests.

## Verification

Before claiming behavior is complete, run focused verification for the changed area and record the observed result in `.context/TASKS.md`.

Use the smallest meaningful check first. Current common checks:

```bash
cargo fmt --manifest-path rust/Cargo.toml --all -- --check
cargo test --manifest-path rust/Cargo.toml --all
cargo run --manifest-path rust/Cargo.toml -p skillctl-cli --bin skillctl -- --help
```

When rebuilding the installed binary:

```bash
cargo build --manifest-path rust/Cargo.toml -p skillctl-cli --bin skillctl --release
cp rust/target/release/skillctl ~/.local/bin/skillctl
~/.local/bin/skillctl --help
```

For plan-error behavior, verify an unmanaged-conflict fixture exits non-zero and prints an `ERROR` row.

Pure docs/context updates require read-back verification of the changed files. Behavior changes require focused Cargo verification plus any relevant CLI smoke scenario.

If `AGENTS.md` changes, re-read it before final response to verify internal consistency.

## Updating `.context`

- `.context/TASKS.md`: update when task status or verification evidence changes.
- `.context/STEERING.md`: update when a durable constraint, product decision, or architecture direction changes.
- `.context/PROJECT.md`: update when the implemented project state or verification summary changes materially.

Keep `.context` compact. Link to durable docs instead of copying large specs, plans, or README content.

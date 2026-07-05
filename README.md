# skillctl

> A small, filesystem-safe Agent Skills materialization CLI built in Rust.

`skillctl` keeps `~/.skillctl/` as the canonical source of Agent Skills and materializes complete runtime-specific skill directories for tools such as Claude Code and Codex.

## Features

- Canonical skill source under `~/.skillctl/skills/`
- Target-specific rendered trees under `~/.skillctl/rendered/<target>/<skill>/`
- Symlink materialization into runtime target directories such as `~/.claude/skills` and `~/.agents/skills`
- YAML-only v1 config at `~/.skillctl/config.yaml`
- Target variant resolution with common `SKILL.md` fallback
- Package-level and target-variant resource directory rendering for `references/`, `scripts/`, `agents/`, `assets/`, and `examples/`
- Per-target `.skillctl.lock.json` ownership tracking
- Deterministic `plan` output with `CREATE`, `UPDATE`, `REMOVE_STALE`, and `ERROR`
- Safe `apply`, `prune`, and `unlink` behavior that refuses desired-path conflicts and managed-path drift before mutation
- `doctor` diagnostics for lockfile ownership, missing paths, drifted symlinks, missing rendered trees, and unmanaged target conflicts

## Status

The Rust workspace is implemented under `rust/`:

```text
rust/
├── skillctl-core/   # config, validation, rendering, planning, applying, lockfiles, doctor
└── skillctl-cli/    # clap parser, command dispatch, process output
```

The current local release binary is installed at:

```text
~/.local/bin/skillctl
```

## Installation

### From source

Requires a Rust toolchain with Rust 2024 edition support.

```bash
cargo build --manifest-path rust/Cargo.toml -p skillctl-cli --bin skillctl --release
cp rust/target/release/skillctl ~/.local/bin/skillctl
```

Verify the installed binary:

```bash
skillctl --help
```

Expected commands:

```text
plan
apply
doctor
list
prune
unlink
version
```

## Quick Start

### 1. Create the canonical source tree

```bash
mkdir -p ~/.skillctl/skills/example-skill
cat > ~/.skillctl/skills/example-skill/SKILL.md <<'EOF'
---
name: example-skill
description: Example skill managed by skillctl.
---

Use this skill when you need an example.
EOF
```

Optional shared resources can live beside `SKILL.md`:

```text
~/.skillctl/skills/example-skill/
├── SKILL.md
├── references/
└── scripts/
```

Target-specific variants can override `SKILL.md` and target-specific resource directories:

```text
~/.skillctl/skills/example-skill/
├── SKILL.md
├── references/
└── variants/
    └── claude/
        ├── SKILL.md
        └── scripts/
```

Current v1 rendering rule: `skillctl` uses the selected target variant's `SKILL.md` when present, falls back to the common `SKILL.md`, copies package-level resource directories, then overlays the selected variant's resource directories. Supported resource directory names are `references/`, `scripts/`, `agents/`, `assets/`, and `examples/`.

### 2. Write config

Create `~/.skillctl/config.yaml`:

```yaml
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
policies: {}
skills:
  example-skill:
    path: skills/example-skill
    expose: [claude, codex]
```

Config validation enforces:

- `version: 1`
- every `skills.*.expose` target exists in `targets`
- v1 policy values only
- skill paths stay inside `~/.skillctl/`

### 3. Inspect the plan

```bash
skillctl plan
```

Plan output is deterministic and uses these labels:

| Label | Meaning |
|-------|---------|
| `CREATE` | target symlink does not exist yet |
| `UPDATE` | managed target needs a new rendered path or source digest |
| `REMOVE_STALE` | lockfile contains a managed entry no longer desired by config |
| `ERROR` | unmanaged conflict or managed-path drift blocks mutation |

`skillctl plan` exits with code `1` when any `ERROR` row exists.

### 4. Apply safely

```bash
skillctl apply
```

`apply` resolves skills, computes digests, builds the plan, aborts before mutation if desired target paths conflict or managed paths drift, renders packages under `~/.skillctl/rendered/`, creates or updates target symlinks, and writes each target's `.skillctl.lock.json`.

### 5. Check health

```bash
skillctl doctor
```

`doctor` reports:

- missing source or target roots
- foreign or invalid lockfiles
- missing managed target paths
- managed paths that are not symlinks
- managed symlink target mismatches
- missing rendered paths
- unmanaged target conflicts

## Command Overview

| Command | Purpose |
|---------|---------|
| `skillctl list` | list configured skill IDs |
| `skillctl plan` | print deterministic planned operations and blocking errors |
| `skillctl apply` | render desired skills, materialize target symlinks, update lockfiles |
| `skillctl doctor` | inspect source roots, target roots, lockfiles, symlinks, and conflicts |
| `skillctl prune` | remove stale lockfile-managed symlinks only |
| `skillctl version` | show CLI version, commit, and build timestamp |
| `skillctl unlink <skill>` | remove lockfile-managed target symlinks for one skill |
| `skillctl unlink <skill> --target <target>` | remove one skill from one configured target |

## Safety Model

`skillctl` separates source, rendered output, and runtime targets:

```text
~/.skillctl/skills/             # canonical source
~/.skillctl/rendered/           # generated target-specific skill trees
~/.claude/skills/               # Claude Code target
~/.agents/skills/               # Codex target
<target>/.skillctl.lock.json    # per-target ownership lockfile
```

Mutation commands follow these rules:

- desired target path conflicts fail before mutation; unrelated unmanaged target entries are left untouched by `plan` and `apply`
- managed target replacement requires the existing path to be missing or still symlink to the lockfile's expected rendered path
- stale removal only removes paths recorded in the lockfile and still matching expected ownership
- drifted regular files, foreign symlinks, and foreign lockfiles are reported instead of overwritten

## Development

Run focused verification from the repository root:

```bash
cargo fmt --manifest-path rust/Cargo.toml --all -- --check
cargo test --manifest-path rust/Cargo.toml --all
cargo run --manifest-path rust/Cargo.toml -p skillctl-cli --bin skillctl -- --help
cargo run --manifest-path rust/Cargo.toml -p skillctl-cli --bin skillctl -- --version
```

Build and install the release binary locally:

```bash
cargo build --manifest-path rust/Cargo.toml -p skillctl-cli --bin skillctl --release
cp rust/target/release/skillctl ~/.local/bin/skillctl
~/.local/bin/skillctl --help
~/.local/bin/skillctl --version
```

## Project Docs

- [Implementation plan](docs/superpowers/plans/2026-07-05-skillctl.md)
- [Project context](.context/PROJECT.md)
- [Steering notes](.context/STEERING.md)
- [Task board](.context/TASKS.md)

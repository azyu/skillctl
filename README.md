# skillctl

[![CI](https://github.com/azyu/skillctl/actions/workflows/ci.yml/badge.svg)](https://github.com/azyu/skillctl/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/azyu/skillctl)](https://github.com/azyu/skillctl/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

[English](README.md) | [한국어](README.ko-kr.MD)

> A small, filesystem-safe Agent Skills materialization CLI built in Rust.

`skillctl` keeps `~/.skillctl/` as the canonical source of Agent Skills and materializes complete runtime-specific skill directories for tools such as Claude Code, Codex, and Pi.

## Features

- Canonical skill source under `~/.skillctl/skills/`
- Manual Git-backed skill sources synced by `skillctl update` into `~/.skillctl/repos/<source_id>/`
- Target-specific rendered trees under `~/.skillctl/rendered/<target>/<skill>/`
- Enabled default symlink targets for Claude Code (`~/.claude/skills`), Codex (`~/.agents/skills`), and Pi (`~/.pi/agent/skills`)
- YAML-only v1 config at `~/.skillctl/config.yaml`
- Target variant resolution with common `SKILL.md` fallback
- Package-level and target-variant resource directory rendering for `references/`, `scripts/`, `agents/`, `assets/`, and `examples/`
- Per-target `.skillctl.lock.json` ownership tracking
- Deterministic `plan` output with `CREATE`, `UPDATE`, `REMOVE_STALE`, and `ERROR`
- Safe `apply`, `prune`, and `unlink` behavior that refuses desired-path conflicts and managed-path drift before mutation
- `doctor` diagnostics for lockfile ownership, missing paths, drifted symlinks, missing rendered trees, and unmanaged target conflicts

## Installation

### Homebrew

```bash
brew install azyu/tap/skillctl
```

### Prebuilt binaries

Download the latest archive from [GitHub Releases](https://github.com/azyu/skillctl/releases/latest).

| Platform | Asset |
|----------|-------|
| Linux amd64 | `skillctl_0.x.y_linux_amd64.tar.gz` |
| Linux arm64 | `skillctl_0.x.y_linux_arm64.tar.gz` |
| macOS arm64 | `skillctl_0.x.y_macos_arm64.tar.gz` |

### From source

Requires a Rust toolchain with Rust 2024 edition support.

```bash
make install
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
    └── pi/
        ├── SKILL.md
        └── scripts/
```

Current v1 rendering rule: `skillctl` uses the selected target variant's `SKILL.md` when present, falls back to the common `SKILL.md`, copies package-level resource directories, then overlays the selected variant's resource directories. Supported resource directory names are `references/`, `scripts/`, `agents/`, `assets/`, and `examples/`.

For target exactly `pi`, the selected common or `variants/pi/SKILL.md` is copied unchanged and must contain a nonblank YAML string `description`. This validation is Pi-specific: description-less skills remain valid for Claude Code, Codex, and arbitrary custom targets.

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
  pi:
    path: ~/.pi/agent/skills
    method: symlink
    enabled: true
policies: {}
skills:
  example-skill:
    path: skills/example-skill
    expose: [claude, codex, pi]
```

An existing `~/.skillctl/config.yaml` remains authoritative and is not merged with built-in defaults. Add the `pi` target and `pi` exposure explicitly when upgrading an existing configuration.

Local skills stay under `skills:`. Git-backed sources are declared separately:

```yaml
version: 1
targets:
  claude:
    path: ~/.claude/skills
    method: symlink
    enabled: true
sources:
  shared:
    type: git
    repo: https://github.com/you/agent-skills.git
    ref: main
    path: skills
skills:
  recap:
    source: shared
    path: recap
    expose: [claude]
```

`skillctl update` records source state in `~/.skillctl/source-lock.json` and keeps the checkout cache under `~/.skillctl/repos/<source_id>/`.

### 3. Sync Git sources

```bash
skillctl update
```

`skillctl update` clones or fetches configured Git sources into `~/.skillctl/repos/<source_id>/`. Those checkouts are tool-owned caches and may be clean-reset during update. `skillctl plan` and `skillctl apply` do not fetch from remotes; they work from the already-synced source state. Target updates are based on the rendered skill input digest, not commit hash alone.

### 4. Inspect the plan

```bash
skillctl plan
```

Plan output is deterministic and uses these labels:

| Label | Meaning |
|-------|---------|
| `CREATE` | target symlink does not exist yet |
| `UPDATE` | managed target needs a new rendered path or rendered input digest |
| `REMOVE_STALE` | lockfile contains a managed entry no longer desired by config |
| `ERROR` | unmanaged conflict or managed-path drift blocks mutation |

`skillctl plan` exits with code `1` when any `ERROR` row exists.

### 5. Apply safely

```bash
skillctl apply
```

`skillctl apply` resolves skills, computes digests, builds the plan, and does not fetch from remotes. It aborts before mutation if desired target paths conflict or managed paths drift, renders packages under `~/.skillctl/rendered/`, creates or updates target symlinks, and writes each target's `.skillctl.lock.json`.

### 6. Check health

```bash
skillctl doctor
```

`doctor` reports source roots, target roots, lockfiles, managed paths, rendered paths, symlink drift, and unmanaged target conflicts.

## Command Overview

| Command | Purpose |
|---------|---------|
| `skillctl list` | list configured skill IDs |
| `skillctl update` | sync configured Git sources into `~/.skillctl/repos/<source_id>/` |
| `skillctl plan` | print deterministic planned operations and blocking errors |
| `skillctl apply` | render desired skills, materialize target symlinks, update lockfiles |
| `skillctl doctor` | inspect source roots, target roots, lockfiles, symlinks, and conflicts |
| `skillctl prune` | remove stale lockfile-managed symlinks only |
| `skillctl version` | show CLI version, commit, and build timestamp |
| `skillctl unlink <skill>` | remove lockfile-managed target symlinks for one skill |
| `skillctl unlink <skill> --target <target>` | remove one skill from one configured target |

## Configuration and Safety

Config validation enforces:

- `version: 1`
- every `skills.*.expose` target exists in `targets`
- v1 policy values only
- skill paths stay inside `~/.skillctl/`

When `plan` or `apply` resolves target exactly `pi`, the selected `SKILL.md` must have a nonblank YAML string `description`.

Git-backed source state is stored in `~/.skillctl/source-lock.json`, and tool-owned Git checkouts live under `~/.skillctl/repos/<source_id>/`.
`skillctl update` may clean-reset those tool-owned checkouts.
`skillctl plan` and `skillctl apply` do not fetch from remotes.
Managed target updates are based on rendered skill input digests, not commit hashes alone.

`skillctl` separates source, Git checkout cache, rendered output, and runtime targets:

```text
~/.skillctl/skills/             # canonical source
~/.skillctl/repos/              # tool-owned Git checkout cache
~/.skillctl/source-lock.json    # Git source state
~/.skillctl/rendered/           # generated target-specific skill trees
~/.claude/skills/               # Claude Code target
~/.agents/skills/               # Codex target
~/.pi/agent/skills/             # Pi target
<target>/.skillctl.lock.json    # per-target ownership lockfile
```

Pi can discover skills from more than one root. Exposing the same skill through `~/.pi/agent/skills` and another Pi-discovered root can create duplicate-name resolution; `skillctl` does not control Pi's discovery precedence.

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

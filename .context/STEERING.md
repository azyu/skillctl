---
created: 2026-07-05
updated: 2026-07-05
---
# STEERING

## Current Priority

Implement `skillctl` as a Rust CLI with deterministic planning and safe materialization of Agent Skills from `~/.skillctl/` into runtime-specific target directories.

## Execution Mode

Follow the implementation plan at `docs/superpowers/plans/2026-07-05-skillctl.md` task-by-task. Prefer subagent-driven implementation with review between tasks when multiple agents are available.

## Non-Negotiable Constraints

- Binary name: `skillctl`.
- Implementation language: Rust.
- Workspace layout follows the referenced two-crate style: `rust/skillctl-core` and `rust/skillctl-cli`.
- Tool-owned root: `~/.skillctl/`.
- Canonical source root: `~/.skillctl/skills/`.
- Render root: `~/.skillctl/rendered/`.
- `~/.agents/skills` is a Codex target, not the source of truth.
- `~/.claude/skills` is a Claude target, not the source of truth.
- Config format for v1: YAML only, at `~/.skillctl/config.yaml`.
- Lockfile name: `.skillctl.lock.json` inside each target skill directory.
- Default materialization method: symlink.
- Unmanaged target conflicts must fail before mutation.
- Tool must only remove target paths recorded in its lockfile and still matching expected ownership.
- Remote install, marketplaces, GUI, YAML/TOML dual support, and patch-style overlays are out of v1 scope.

## Target Seams and Interfaces

- `skillctl-core` owns filesystem state, config parsing, validation, rendering, planning, applying, lockfiles, and diagnostics.
- `skillctl-cli` owns clap argument parsing, process exit codes, and human-readable output.
- Tests should exercise core behavior through public core functions and CLI behavior through the compiled binary.

## Decisions Log

| Date | Decision | Rationale |
|---|---|---|
| 2026-07-05 | Name the tool `skillctl`. | The name matches the intended `plan/apply/doctor` control-plane style and avoids overloaded `asm` naming. |
| 2026-07-05 | Use Rust. | The tool is a filesystem state manager; correctness, atomic operations, and single-binary distribution matter. |
| 2026-07-05 | Use `~/.skillctl/` as the tool root. | It clearly marks tool-owned source, rendered output, and state without overloading `~/.agents/skills`. |
| 2026-07-05 | Keep target directories separate from source. | Prevents active runtime discovery paths from also being the canonical source tree. |
| 2026-07-05 | Use rendered packages between source and targets. | Runtime variants can share references/scripts while targets see complete skill directories. |
| 2026-07-05 | Start with YAML only. | Skill metadata already uses YAML frontmatter; dual YAML/TOML support is unnecessary v1 complexity. |
| 2026-07-05 | Publish release archives through GitHub Actions and update `azyu/homebrew-tap` with `HOMEBREW_TAP_TOKEN`. | Homebrew install support should be backed by release assets and SHA256-backed formula updates rather than README-only claims. |

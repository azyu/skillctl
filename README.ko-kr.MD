# skillctl

[English](README.md) | [한국어](README.ko-kr.MD)

> Rust로 작성된, 파일시스템 안전성을 우선하는 작은 Agent Skills materialization CLI입니다.

`skillctl`은 `~/.skillctl/`을 Agent Skills의 canonical source로 유지하고 Claude Code, Codex 같은 런타임별 완성된 skill 디렉터리를 materialize합니다.

## 특징

- canonical skill source를 `~/.skillctl/skills/` 아래에 보관
- target별 rendered tree를 `~/.skillctl/rendered/<target>/<skill>/` 아래에 생성
- `~/.claude/skills`, `~/.agents/skills` 같은 런타임 target 디렉터리에 symlink로 materialize
- v1 설정은 `~/.skillctl/config.yaml`의 YAML만 지원
- target variant가 있으면 우선 사용하고, 없으면 공통 `SKILL.md`로 fallback
- package-level 및 target-variant resource 디렉터리 `references/`, `scripts/`, `agents/`, `assets/`, `examples/` 렌더링
- target별 `.skillctl.lock.json` ownership 추적
- `CREATE`, `UPDATE`, `REMOVE_STALE`, `ERROR` 라벨을 쓰는 deterministic `plan` 출력
- desired-path conflict와 managed-path drift가 있으면 mutation 전에 거부하는 안전한 `apply`, `prune`, `unlink`
- lockfile ownership, 누락된 path, drifted symlink, 누락된 rendered tree, unmanaged target conflict를 진단하는 `doctor`

## 설치

### Homebrew

```bash
brew install azyu/tap/skillctl
```

### 미리 빌드된 바이너리

[GitHub Releases](https://github.com/azyu/skillctl/releases/latest)에서 최신 아카이브를 다운로드하세요.

| 플랫폼 | 에셋 |
|--------|------|
| Linux amd64 | `skillctl_0.x.y_linux_amd64.tar.gz` |
| Linux arm64 | `skillctl_0.x.y_linux_arm64.tar.gz` |
| macOS arm64 | `skillctl_0.x.y_macos_arm64.tar.gz` |

### 소스에서 빌드

Rust 2024 edition을 지원하는 Rust toolchain이 필요합니다.

```bash
make install
```

설치된 바이너리를 확인합니다:

```bash
skillctl --help
```

예상 명령:

```text
plan
apply
doctor
list
prune
unlink
version
```

## 빠른 시작

### 1. canonical source tree 만들기

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

선택적 shared resource는 `SKILL.md` 옆에 둘 수 있습니다:

```text
~/.skillctl/skills/example-skill/
├── SKILL.md
├── references/
└── scripts/
```

target-specific variant는 `SKILL.md`와 target-specific resource 디렉터리를 override할 수 있습니다:

```text
~/.skillctl/skills/example-skill/
├── SKILL.md
├── references/
└── variants/
    └── claude/
        ├── SKILL.md
        └── scripts/
```

현재 v1 렌더링 규칙: `skillctl`은 선택된 target variant의 `SKILL.md`가 있으면 그것을 사용하고, 없으면 공통 `SKILL.md`로 fallback합니다. 그다음 package-level resource 디렉터리를 복사하고 선택된 variant의 resource 디렉터리를 overlay합니다. 지원되는 resource 디렉터리 이름은 `references/`, `scripts/`, `agents/`, `assets/`, `examples/`입니다.

### 2. 설정 작성

`~/.skillctl/config.yaml`을 만듭니다:

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

### 3. plan 확인

```bash
skillctl plan
```

plan 출력은 deterministic하며 다음 라벨을 사용합니다:

| 라벨 | 의미 |
|------|------|
| `CREATE` | target symlink가 아직 없음 |
| `UPDATE` | managed target에 새 rendered path 또는 source digest가 필요함 |
| `REMOVE_STALE` | lockfile에 더 이상 config가 원하지 않는 managed entry가 있음 |
| `ERROR` | unmanaged conflict 또는 managed-path drift가 mutation을 막음 |

`skillctl plan`은 `ERROR` row가 하나라도 있으면 exit code `1`로 종료합니다.

### 4. 안전하게 적용

```bash
skillctl apply
```

`apply`는 skill을 resolve하고 digest를 계산한 뒤 plan을 만듭니다. desired target path conflict나 managed path drift가 있으면 mutation 전에 중단합니다. 통과하면 package를 `~/.skillctl/rendered/` 아래에 렌더링하고 target symlink를 만들거나 갱신한 뒤 각 target의 `.skillctl.lock.json`을 씁니다.

### 5. 상태 진단

```bash
skillctl doctor
```

`doctor`는 source root, target root, lockfile, managed path, rendered path, symlink drift, unmanaged target conflict를 보고합니다.

## 명령 개요

| 명령 | 용도 |
|------|------|
| `skillctl list` | 설정된 skill ID 목록 출력 |
| `skillctl plan` | deterministic planned operation과 blocking error 출력 |
| `skillctl apply` | 원하는 skill을 렌더링하고 target symlink와 lockfile 갱신 |
| `skillctl doctor` | source root, target root, lockfile, symlink, conflict 진단 |
| `skillctl prune` | stale lockfile-managed symlink만 제거 |
| `skillctl version` | CLI version, commit, build timestamp 출력 |
| `skillctl unlink <skill>` | 한 skill의 lockfile-managed target symlink 제거 |
| `skillctl unlink <skill> --target <target>` | 한 configured target에서 한 skill만 제거 |

## 설정과 안전 모델

설정 검증은 다음을 강제합니다:

- `version: 1`
- 모든 `skills.*.expose` target이 `targets`에 존재
- v1 policy 값만 허용
- skill path가 `~/.skillctl/` 안에 머무름

`skillctl`은 source, rendered output, runtime target을 분리합니다:

```text
~/.skillctl/skills/             # canonical source
~/.skillctl/rendered/           # generated target-specific skill trees
~/.claude/skills/               # Claude Code target
~/.agents/skills/               # Codex target
<target>/.skillctl.lock.json    # per-target ownership lockfile
```

mutation 명령은 다음 규칙을 따릅니다:

- desired target path conflict는 mutation 전에 실패합니다. 관련 없는 unmanaged target entry는 `plan`과 `apply`가 건드리지 않습니다.
- managed target replacement는 기존 path가 없거나 lockfile의 expected rendered path를 가리키는 symlink일 때만 허용됩니다.
- stale removal은 lockfile에 기록되어 있고 expected ownership과 일치하는 path만 제거합니다.
- drifted regular file, foreign symlink, foreign lockfile은 덮어쓰지 않고 보고합니다.

## 개발

저장소 root에서 focused verification을 실행합니다:

```bash
cargo fmt --manifest-path rust/Cargo.toml --all -- --check
cargo test --manifest-path rust/Cargo.toml --all
cargo run --manifest-path rust/Cargo.toml -p skillctl-cli --bin skillctl -- --help
cargo run --manifest-path rust/Cargo.toml -p skillctl-cli --bin skillctl -- --version
```

release binary를 로컬에 빌드하고 설치합니다:

```bash
cargo build --manifest-path rust/Cargo.toml -p skillctl-cli --bin skillctl --release
cp rust/target/release/skillctl ~/.local/bin/skillctl
~/.local/bin/skillctl --help
~/.local/bin/skillctl --version
```

## 프로젝트 문서

- [Implementation plan](docs/superpowers/plans/2026-07-05-skillctl.md)
- [Project context](.context/PROJECT.md)
- [Steering notes](.context/STEERING.md)
- [Task board](.context/TASKS.md)

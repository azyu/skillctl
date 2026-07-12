# skillctl

[![CI](https://github.com/azyu/skillctl/actions/workflows/ci.yml/badge.svg)](https://github.com/azyu/skillctl/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/azyu/skillctl)](https://github.com/azyu/skillctl/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

[English](README.md) | [한국어](README.ko-kr.MD)

> Rust로 작성된, 파일시스템 안전성을 우선하는 작은 Agent Skills 배포 CLI입니다.

`skillctl`은 `~/.skillctl/`을 Agent Skills의 원본 저장소로 사용하고, Claude Code, Codex, Pi 같은 실행 환경이 바로 읽을 수 있는 완성된 스킬 디렉터리를 만들어 연결합니다.

## 특징

- 원본 스킬은 `~/.skillctl/skills/` 아래에 보관
- `skillctl update`로 수동 Git-backed 스킬 소스를 `~/.skillctl/repos/<source_id>/` 아래에 동기화
- 실행 환경별 결과물은 `~/.skillctl/rendered/<target>/<skill>/` 아래에 생성
- Claude Code(`~/.claude/skills`), Codex(`~/.agents/skills`), Pi(`~/.pi/agent/skills`)를 기본으로 활성화하고 심볼릭 링크로 연결
- v1 설정 파일은 `~/.skillctl/config.yaml`의 YAML만 지원
- 실행 환경별 변형 파일이 있으면 우선 사용하고, 없으면 공통 `SKILL.md`를 사용
- 공통 리소스와 실행 환경별 리소스 디렉터리 `references/`, `scripts/`, `agents/`, `assets/`, `examples/`를 함께 렌더링
- 실행 환경별 `.skillctl.lock.json` 파일로 소유권 추적
- `CREATE`, `UPDATE`, `REMOVE_STALE`, `ERROR` 라벨을 쓰는 재현 가능한 `plan` 출력
- 대상 경로 충돌이나 관리 대상 경로 이탈이 있으면 변경 전에 중단하는 안전한 `apply`, `prune`, `unlink`
- lockfile 소유권, 누락된 경로, 어긋난 심볼릭 링크, 누락된 렌더링 결과, 관리되지 않는 대상 경로 충돌을 진단하는 `doctor`

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

### 1. 원본 스킬 트리 만들기

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

선택 사항인 공통 리소스는 `SKILL.md` 옆에 둘 수 있습니다:

```text
~/.skillctl/skills/example-skill/
├── SKILL.md
├── references/
└── scripts/
```

실행 환경별 변형은 `SKILL.md`와 리소스 디렉터리를 덮어쓸 수 있습니다:

```text
~/.skillctl/skills/example-skill/
├── SKILL.md
├── references/
└── variants/
    └── pi/
        ├── SKILL.md
        └── scripts/
```

현재 v1 렌더링 규칙: `skillctl`은 선택한 실행 환경의 변형 `SKILL.md`가 있으면 그것을 사용하고, 없으면 공통 `SKILL.md`를 사용합니다. 그다음 공통 리소스 디렉터리를 복사하고, 선택한 실행 환경의 리소스 디렉터리를 그 위에 덮어씁니다. 지원하는 리소스 디렉터리 이름은 `references/`, `scripts/`, `agents/`, `assets/`, `examples/`입니다.

대상이 정확히 `pi`이면 선택된 공통 파일 또는 `variants/pi/SKILL.md`를 변경 없이 복사하며, YAML 문자열 `description`이 비어 있지 않아야 합니다. 이 검증은 Pi에만 적용되므로 `description`이 없는 스킬도 Claude Code, Codex, 임의의 사용자 정의 대상에서는 계속 사용할 수 있습니다.

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

기존 `~/.skillctl/config.yaml`은 그대로 우선 적용되며 내장 기본값과 병합되지 않습니다. 기존 설정을 업그레이드할 때는 `pi` 대상과 `pi` 노출을 명시적으로 추가해야 합니다.

로컬 스킬은 `skills:` 아래에 두고, Git-backed 소스는 별도로 선언합니다:

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

`skillctl update`는 `~/.skillctl/source-lock.json`에 소스 상태를 기록하고, 체크아웃 캐시는 `~/.skillctl/repos/<source_id>/` 아래에 둡니다.

### 3. Git 소스 동기화

```bash
skillctl update
```

`skillctl update`는 설정된 Git 소스를 `~/.skillctl/repos/<source_id>/`로 복제하거나 가져옵니다. 이 체크아웃은 `skillctl`이 소유하는 캐시이며 update 중 clean reset될 수 있습니다. `skillctl plan`과 `skillctl apply`는 원격을 조회하지 않고, 이미 동기화된 소스 상태만 사용합니다. 대상 갱신 기준은 commit hash 자체가 아니라 렌더된 입력 digest입니다.

### 4. 변경 계획 확인

```bash
skillctl plan
```

`plan` 출력은 항상 같은 입력에 대해 같은 순서로 나오며 다음 라벨을 사용합니다:

| 라벨 | 의미 |
|------|------|
| `CREATE` | 대상 심볼릭 링크가 아직 없음 |
| `UPDATE` | 관리 중인 대상에 새 렌더링 경로 또는 렌더된 입력 digest 반영이 필요함 |
| `REMOVE_STALE` | lockfile에는 있지만 현재 설정에서는 더 이상 원하지 않는 관리 항목이 있음 |
| `ERROR` | 관리되지 않는 충돌 또는 관리 대상 경로 이탈 때문에 변경할 수 없음 |

`skillctl plan`은 `ERROR` 행이 하나라도 있으면 exit code `1`로 종료합니다.

### 5. 안전하게 적용

```bash
skillctl apply
```

`skillctl apply`는 스킬을 해석하고 digest를 계산한 뒤 변경 계획을 만들며 원격을 조회하지 않습니다. 원하는 대상 경로에 충돌이 있거나 관리 대상 경로가 이탈했으면 변경 전에 중단합니다. 문제가 없으면 `~/.skillctl/rendered/` 아래에 패키지를 렌더링하고, 대상 심볼릭 링크를 만들거나 갱신한 뒤 각 대상의 `.skillctl.lock.json`을 씁니다.

### 6. 상태 진단

```bash
skillctl doctor
```

`doctor`는 원본 루트, 대상 루트, lockfile, 관리 대상 경로, 렌더링 결과, 심볼릭 링크 이탈, 관리되지 않는 대상 경로 충돌을 보고합니다.

## 명령 개요

| 명령 | 용도 |
|------|------|
| `skillctl list` | 설정된 스킬 ID 목록 출력 |
| `skillctl update` | Git 소스를 `~/.skillctl/repos/<source_id>/` 아래로 동기화 |
| `skillctl plan` | 재현 가능한 변경 계획과 차단 오류 출력 |
| `skillctl apply` | 원하는 스킬을 렌더링하고 대상 심볼릭 링크와 lockfile 갱신 |
| `skillctl doctor` | 원본 루트, 대상 루트, lockfile, 심볼릭 링크, 충돌 진단 |
| `skillctl prune` | 오래된 lockfile 관리 심볼릭 링크만 제거 |
| `skillctl version` | CLI version, commit, build timestamp 출력 |
| `skillctl unlink <skill>` | 한 스킬의 lockfile 관리 대상 심볼릭 링크 제거 |
| `skillctl unlink <skill> --target <target>` | 설정된 대상 하나에서 스킬 하나만 제거 |

## 설정과 안전 모델

설정 검증은 다음을 강제합니다:

- `version: 1`
- 모든 `skills.*.expose` 대상이 `targets`에 존재
- v1 policy 값만 허용
- 스킬 경로가 `~/.skillctl/` 안에 머무름

`plan` 또는 `apply`가 정확히 `pi`인 대상을 해석할 때, 선택된 `SKILL.md`에는 비어 있지 않은 YAML 문자열 `description`이 있어야 합니다.

Git-backed 소스 상태는 `~/.skillctl/source-lock.json`에 저장되고, `skillctl`이 소유하는 Git 체크아웃은 `~/.skillctl/repos/<source_id>/` 아래에 둡니다.
`skillctl update`는 그 체크아웃을 clean reset할 수 있습니다.
`skillctl plan`과 `skillctl apply`는 원격을 조회하지 않습니다.
대상 갱신 기준은 commit hash 자체가 아니라 렌더된 입력 digest입니다.

`skillctl`은 원본, Git 체크아웃 캐시, 렌더링 결과, 실행 환경의 대상 디렉터리를 분리합니다:

```text
~/.skillctl/skills/             # 원본 스킬
~/.skillctl/repos/              # Git 체크아웃 캐시
~/.skillctl/source-lock.json    # Git 소스 상태
~/.skillctl/rendered/           # 실행 환경별로 생성된 스킬 트리
~/.claude/skills/               # Claude Code 대상 디렉터리
~/.agents/skills/               # Codex 대상 디렉터리
~/.pi/agent/skills/             # Pi 대상 디렉터리
<target>/.skillctl.lock.json    # 대상별 소유권 lockfile
```

Pi는 둘 이상의 루트에서 스킬을 찾을 수 있습니다. 같은 스킬을 `~/.pi/agent/skills`와 Pi가 검색하는 다른 루트에 함께 노출하면 이름이 중복되어 해석 문제가 생길 수 있으며, `skillctl`은 Pi의 검색 우선순위를 제어하지 않습니다.

변경 명령은 다음 규칙을 따릅니다:

- 원하는 대상 경로에 충돌이 있으면 변경 전에 실패합니다. 관련 없는 관리 외 대상 항목은 `plan`과 `apply`가 건드리지 않습니다.
- 관리 대상 교체는 기존 경로가 없거나 lockfile의 예상 렌더링 경로를 가리키는 심볼릭 링크일 때만 허용됩니다.
- 오래된 항목 제거는 lockfile에 기록되어 있고 예상 소유권과 일치하는 경로만 제거합니다.
- 어긋난 일반 파일, 외부 심볼릭 링크, 외부 lockfile은 덮어쓰지 않고 보고합니다.

## 개발

저장소 루트에서 집중 검증을 실행합니다:

```bash
cargo fmt --manifest-path rust/Cargo.toml --all -- --check
cargo test --manifest-path rust/Cargo.toml --all
cargo run --manifest-path rust/Cargo.toml -p skillctl-cli --bin skillctl -- --help
cargo run --manifest-path rust/Cargo.toml -p skillctl-cli --bin skillctl -- --version
```

릴리즈 바이너리를 로컬에 빌드하고 설치합니다:

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

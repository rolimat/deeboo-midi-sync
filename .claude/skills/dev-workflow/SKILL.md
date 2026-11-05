---
name: dev-workflow
description: Use when setting up the environment, running just commands, working with Docker, lefthook, CI or releases, committing, or deciding what runs on macOS versus in the container.
---

# Dev workflow

Spec §9: [docs/superpowers/specs/2026-11-05-deeboo-midi-sync-design.md](../../../docs/superpowers/specs/2026-11-05-deeboo-midi-sync-design.md#9-entorno-de-desarrollo-y-calidad).

## Where things run
- Docker (image `deeboo-dev`, Rust 1.99.1, Node, `just`, cargo-deny, hadolint): fmt, lint, tests. Recipes re-exec through `docker compose run --rm dev just <recipe>` when `DEEBOO_IN_CONTAINER` is unset, so run them from the host as usual.
- macOS host only: `just dev`, `just dmg`, `just jitter` (need CoreMIDI and macOS bundles). They exit with an error elsewhere.

## Recipes
| Recipe | Does |
|---|---|
| `just fmt` / `fmt-check` | `cargo fmt`; UI Prettier when `ui/package.json` exists |
| `just lint` | clippy `-D warnings`, `cargo deny`, hadolint, `scripts/check-agent-docs.sh`, UI eslint + typecheck |
| `just test` | `cargo test --workspace`; Vitest when `ui/package.json` exists |
| `just precommit` (`check`) | fmt-check + lint + test |
| `just dev` / `just dmg` | macOS only: `npm --prefix ui run tauri dev` / `tauri build -- --bundles dmg` (run `npm ci` first if `ui/node_modules` is missing) |
| `just ui-dist` | builds `ui/dist`; `lint` and `test` run it first because `tauri::generate_context!` needs it |
| `just ship` | push and wait for the CI run of this exact commit |

UI steps are guarded by `[ -f ui/package.json ]`, so they appear once the UI exists.

## Rules
- Hook: lefthook pre-commit runs `just precommit`. Never `--no-verify`; fix the cause.
- Conventional Commits (`feat(scope):`, `fix:`, `docs:`, `chore:`, ...).
- Branch `feat/v1`, never `main`. Every task ends with commit, then `just ship`; done only when CI is green. If red: fix in a new commit and ship again.
- Convention, command or interface change: update the matching skill in the same commit.

## Details
- Image, compose, volumes, macOS steps: [references/docker-and-macos.md](references/docker-and-macos.md) (read when env or Docker issues arise).
- CI jobs, tags, DMG notes: [references/ci-and-release.md](references/ci-and-release.md) (read when touching workflows or releasing).

## Checklist
- [ ] `just precommit` green before commit.
- [ ] Commit message is Conventional.
- [ ] `just ship` ended green.

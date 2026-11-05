# Deeboo MIDI Sync: agent guide

## Project
macOS menu bar app (Tauri 2 + Rust + midir) that relays MIDI clock from a master to a slave with
Play/Pause and hold-to-nudge. Pure logic lives in `crates/clock-core`. Open a skill (below) before
working in its area; do not re-read the whole spec.
Spec: `docs/superpowers/specs/2026-11-05-deeboo-midi-sync-design.md` (Spanish).
Plan: `docs/superpowers/plans/2026-11-05-deeboo-midi-sync.md`.

## Hard rules
- App text is English; never name third-party software (Traktor, Ableton...).
- Ports default `Deeboo In` (virtual destination) / `Deeboo Out` (virtual source).
- 24 PPQN; `0xF8` clock, `0xFA` Start, `0xFB` Continue, `0xFC` Stop, `0xF2` SPP.
- No clock for 500 ms = no signal. Deeboo starts paused.
- Only Deeboo's Play starts the slave; incoming Start/Continue/SPP are never forwarded.
- Incoming Stop or clock loss while playing: send one `Stop`, pause.
- Nudge default 4 %, range 1-10 %. BPM has 1 decimal, `—` without signal.
- Popover: icon-only buttons; port rows are dot + `In · name` / `Out · name`.
- No Dock icon (`Accessory` + `LSUIElement`). No signing or notarization.
- Plain CSS custom properties and plain TS: no CSS or UI framework.
- One monotonic µs clock feeds `clock-core`; never mix in midir timestamps.
- Everything except real MIDI, the DMG and the jitter probe runs in Docker.
- Work on `feat/v1`, never `main`. Never `--no-verify`.

## Commands
Docker (re-exec themselves via `docker compose run`): `just fmt`, `fmt-check`, `lint`, `test`,
`precommit` (= `check`).
macOS host only: `just dev`, `just dmg`, `just jitter`.
`just ship`: push and wait for the CI run of this commit.

## Layout
- `crates/clock-core/` pure engine: parser, BPM, transport, nudge (tests in `tests/`)
- `src-tauri/` app: midir bridge, settings, commands, tray
- `ui/` TypeScript + Vite popover, Vitest tests next to sources
- `tools/jitter-probe/` macOS jitter measurement binary
- `docker/`, `compose.yaml`, `justfile`, `lefthook.yml`, `.github/` dev environment and CI
- `.claude/skills/` skills; `scripts/check-agent-docs.sh` enforces budgets

## Skills index
Claude Code loads these from `.claude/skills/`. Other agents: open the `SKILL.md` when "when" matches.
| Skill | When |
|---|---|
| `midi-clock-domain` | clock bytes, transport, BPM, nudge, beat counter |
| `rust-code` | Rust in `crates/` or `src-tauri/` |
| `writing-tests` | any feature or bug fix, any language |
| `ui-code` | anything in `ui/` |
| `dev-workflow` | setup, commands, hooks, CI, releasing |

## Done means
- `just precommit` is green and tests were written first (red, then green).
- Commit follows Conventional Commits; then `just ship` and CI is green.
- Maintenance: any change to a convention, command or interface updates the matching skill in the
  same commit.

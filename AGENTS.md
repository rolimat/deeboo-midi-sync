# Useful commands

```sh
just precommit   # fmt-check + lint + tests, inside Docker (the pre-commit hook runs this)
just fmt         # format Rust + UI
just test        # Rust + Vitest tests (Docker)
just lint        # clippy -D warnings, cargo-deny, hadolint, eslint, tsc, agent-docs budget
just dev         # macOS host: run the app with real MIDI (hot reload)
just dmg         # macOS host: build the .dmg (use CI=true in non-GUI sessions)
just jitter --seconds 60 --connect "Deeboo Clock Sync Out"   # macOS host: measure clock jitter
```

# TDD (mandatory)

- Every behaviour change starts with a failing test: write it, run it, see it fail (red),
  then write the minimum code to pass (green), then refactor.
- No production code without a test that exercised it first. Bug fixes start with a test
  that reproduces the bug.
- Where tests go and how (simulated time, mocked UI api): `writing-tests` skill.

# Architecture

- macOS menu bar app: Tauri 2 + Rust + midir; UI in TypeScript + Vite, plain CSS (no frameworks).
- `crates/clock-core/`: pure, deterministic clock engine (parser, BPM, transport, nudge). No I/O, no real time.
- `src-tauri/`: midir virtual ports bridge, timer thread, settings, Tauri commands, tray.
- `ui/`: popover and settings views. `tools/jitter-probe/`: jitter measurement binary.
- Everything except real MIDI, the DMG and the jitter probe runs in Docker.

# Golden rules

- App text is English and never names third-party software.
- Ports: `Deeboo Clock Sync In` (virtual destination) / `Deeboo Clock Sync Out` (virtual source).
- Only Deeboo's Play starts the slave; incoming Start/Continue/SPP are never forwarded.
  Incoming Stop or 500 ms without clock while playing → one `Stop`, pause.
- One monotonic µs clock feeds `clock-core`; never mix in midir timestamps.
- Never `--no-verify`. Never write DOM under the pointer on every state tick.

# Documentation

- Spec (Spanish): `docs/superpowers/specs/2026-11-05-deeboo-midi-sync-design.md`
- Detailed conventions live in skills. Use this map and read only what your task needs
  (Claude Code loads them automatically; other agents open the `SKILL.md` first):

```
.claude/skills/
├── midi-clock-domain/      # clock bytes, transport, BPM, nudge, beat counter
│   └── references/transport-and-nudge.md
├── rust-engineer/          # Rust in crates/, src-tauri/, tools/ (adapted from Jeffallan, MIT)
│   └── references/{ownership, traits, error-handling, testing, deeboo-clock-core, deeboo-tauri-app}.md
├── writing-tests/          # TDD, simulated time, Vitest with mocked api
│   └── references/{rust-simulated-time.md, ui-vitest.md}
├── ui-code/                # anything in ui/: views, styles, copy
│   └── references/styles-and-copy.md
└── dev-workflow/           # Docker vs macOS, hooks, CI, releases
    └── references/{docker-and-macos.md, ci-and-release.md}
```

# Plans

- New feature or behaviour change: brainstorm first, then write the spec in
  `docs/superpowers/specs/` and the plan in `docs/superpowers/plans/YYYY-MM-DD-<topic>.md`.
- Specs are committed. Plans are never committed (`docs/superpowers/plans/` is git-ignored).
- **Plan-only prompts** ("crea un plan para…", "create a plan for…", "diseña…"): only write the
  plan document. Do not change code, run the app or tag anything.
- **Implement** only when explicitly asked ("implementa el plan", "ejecuta el plan").

# Git, CI and releases

- Work on a feature branch and open a PR to `main`. Push straight to `main` only when the user says so.
- After each commit: push, then watch CI without blocking. The macOS job runs only when
  `src-tauri/`, `ui/`, `tools/`, `Cargo.lock` or workflows change; PRs always run it.
- Release: bump the version in `src-tauri/tauri.conf.json`, `ui/package.json` and `Cargo.toml`, merge to `main`,
  then `git tag -a vX.Y.Z -m "..." && git push origin vX.Y.Z`. Ask before pushing any tag.

# Done means

- Tests written first (red, then green) and `just precommit` green.
- Conventional Commits.
- Any change to a convention, command or interface updates its skill in the same commit.

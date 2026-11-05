# Deeboo MIDI Sync

## What it is
A macOS menu bar app that relays MIDI clock from a master to a slave, with Play/Pause and a
hold-to-nudge "vinyl" push to line up phase by ear. It exposes two virtual ports, `Deeboo In` and
`Deeboo Out`, and shows the incoming BPM in the menu bar.

## Install
TODO(task-11): download the `.dmg` from Releases and the unsigned-app steps.

## Usage
1. Send MIDI clock from your master to `Deeboo In`; the menu bar shows the BPM and a green dot.
2. Point your slave's clock input at `Deeboo Out`.
3. Press Play to start the slave. Hold `◀◀` or `▶▶` to nudge it slower or faster; release to return to 1:1.
4. Ports, nudge strength and launch at login are in Settings (⚙).

## Development
Everything except real MIDI and the DMG runs in Docker. Requires Docker, `just`, `lefthook`.
- `just precommit`: format check, lint and tests (also the git hook).
- `just dev`, `just dmg`: macOS host only.
- `just ship`: push and wait for CI.

Agent guidance: `AGENTS.md` and `.claude/skills/`. Design: `docs/superpowers/specs/`.

## Measuring jitter
TODO(task-12): how to run `just jitter` and read the report.

## License
GPL-3.0.

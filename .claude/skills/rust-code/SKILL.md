---
name: rust-code
description: Use when writing or reviewing Rust in crates/clock-core, src-tauri or tools/jitter-probe: purity of clock-core, error handling, clippy pedantic, module layout, Tauri commands and the MIDI callback.
---

# Rust code

## Rules
- `clock-core` is pure: no I/O, no system clock, no threads, no `midir`/`tauri`. Time comes in as `now_us: u64` arguments.
- No `unwrap`/`expect` outside tests. Errors are logged (`log::error!`/`warn!`) or returned, never panic, especially on port creation and in MIDI callbacks.
- Clippy is pedantic at warn (workspace lints) and CI uses `-D warnings`. Each crate sets `lints.workspace = true`. Fix the lint; allow only with a one-line reason.
- One responsibility per module (`bpm.rs`, `engine.rs`, `bridge.rs`, `settings.rs`, `commands.rs`, `tray.rs`, `clock.rs`).
- Dependencies must pass `cargo deny` (GPL-3.0 compatible licenses).
- Tests first: see the `writing-tests` skill.

## clock-core API
`Engine::{new, on_midi, on_command, on_tick, next_deadline_us, state}`; bytes out are
`Output { send }`. Details and how to add a state field: [references/clock-core.md](references/clock-core.md) (read when changing `clock-core`).

## App (src-tauri)
- Commands are snake_case; DTOs are camelCase via serde; state event is `deeboo://state`.
- Forwarding `0xF8` happens inside the input callback, with the engine lock held only for the engine call and the write; no UI calls.
- Every timestamp comes from the one monotonic clock (`MonoClock`), not midir's.
- Commands, lock scope and time base: [references/tauri-app.md](references/tauri-app.md) (read when changing `src-tauri`).

## Checklist
- [ ] `clock-core` imports nothing impure.
- [ ] No `unwrap`/`expect`/indexing panics on external input.
- [ ] `just precommit` clean (clippy, fmt, deny).
- [ ] Interface change: update the reference and the plan-linked skill in the same commit.

---
name: rust-engineer
description: Use when writing, reviewing or debugging Rust in crates/clock-core, src-tauri or tools/jitter-probe: ownership, traits, error handling, testing, clippy, plus this project's clock-core and Tauri bridge rules.
license: MIT
metadata:
  author: https://github.com/Jeffallan
  source: https://github.com/Jeffallan/claude-skills/tree/078a25c/skills/rust-engineer
  adapted-for: deeboo-midi-sync (budget trim, project overrides, async removed)
  related-skills: writing-tests, midi-clock-domain
---

# Rust Engineer

Senior Rust engineer: idiomatic, memory-safe, zero-cost abstractions, ownership-driven design.

## Project overrides (win over the generic text below)
- **TDD always**: failing test first, then code (see `writing-tests`).
- Edition 2024, clippy pedantic via workspace lints, CI runs `-D warnings`. Allow a lint only with a one-line reason.
- No `unwrap`/`expect` outside tests. Log (`log::error!`/`warn!`) or return errors; never panic in MIDI callbacks or port setup.
- No async/tokio: the app uses `std::thread` + `Mutex`. Do not add an async runtime.
- `clock-core` is pure: no I/O, no system clock, no threads, no `midir`/`tauri`. API and how to extend it: [references/deeboo-clock-core.md](references/deeboo-clock-core.md).
- `src-tauri`: one monotonic µs clock, lock order engine→out, send under the engine lock, commands snake_case / DTOs camelCase / event `deeboo://state`: [references/deeboo-tauri-app.md](references/deeboo-tauri-app.md).
- Dependencies must pass `cargo deny` (GPL-3.0 compatible).
- Docs: one-line `///` on public items; doctests only where an example clarifies.

## Core workflow
1. **Analyze ownership**: lifetimes and borrowing; explicit lifetimes only where inference fails.
2. **Design traits**: only when there are two real implementations (no single-impl traits).
3. **Implement safely**: no `unsafe`; if unavoidable, document its invariants.
4. **Handle errors**: `Result`/`Option` with `?`; `thiserror` for library error enums.
5. **Validate**: `just precommit` (fmt-check, clippy, deny, tests).

## Reference guide (load only when relevant)
| Topic | Reference | Load when |
|---|---|---|
| Ownership | [references/ownership.md](references/ownership.md) | lifetimes, borrowing, smart pointers |
| Traits | [references/traits.md](references/traits.md) | trait design, generics, derive |
| Errors | [references/error-handling.md](references/error-handling.md) | Result/Option, custom errors |
| Testing | [references/testing.md](references/testing.md) | unit/integration tests, fixtures |
| clock-core | [references/deeboo-clock-core.md](references/deeboo-clock-core.md) | changing the engine |
| Tauri app | [references/deeboo-tauri-app.md](references/deeboo-tauri-app.md) | changing src-tauri |

## Must not
- Clone or allocate to dodge the borrow checker; prefer `&str`/`&[u8]`.
- Ignore clippy, skip error handling, or leave `unsafe` undocumented.
- Mix blocking work into the MIDI callback beyond the engine call and the write.

## Checklist
- [ ] Test written first and seen failing.
- [ ] `clock-core` imports nothing impure.
- [ ] No `unwrap`/`expect`/indexing panics on external input.
- [ ] `just precommit` clean.
- [ ] Interface change: update the matching reference in the same commit.

Adapted from [Jeffallan/claude-skills](https://github.com/Jeffallan/claude-skills) (MIT).

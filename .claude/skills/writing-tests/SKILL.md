---
name: writing-tests
description: Use when adding a feature or fixing a bug in any language: TDD loop, where each kind of test belongs, test naming, simulated time for clock-core, Vitest with a mocked api for the UI.
---

# Writing tests

## Loop
1. Write the failing test; run `just test`; see it fail for the right reason.
2. Implement the minimum; run `just test`; see it pass.
3. Refactor, then `just precommit`.
Bug fix: first a test that reproduces it.

## Where tests go
| Kind of logic | Where | How |
|---|---|---|
| Clock, transport, BPM, nudge | `crates/clock-core/tests/*.rs` | `Engine` with simulated time, no real clock |
| App glue (settings, DTO, tray frame) | unit tests in the module | pure helpers, temp dirs |
| UI views | `ui/src/*.test.ts` | Vitest + jsdom, mocked `api` |

Real MIDI is never used in automated tests; real-MIDI checks are the manual acceptance tests in
spec §10 (macOS host).

## Naming
`snake_case_behaviour` that states the outcome, e.g. `clock_loss_while_playing_sends_single_stop`,
`stops_nudge_on_pointerleave`. One behaviour per test.

## Review-focus cases (each has a named test; keep them green)
- Packed or mixed bytes: every `0xF8` counts, nothing else does.
- Tempo change within one beat; zero interval gives no NaN.
- Clock lost mid-nudge: one `Stop`, no leaked pulse.
- Nudge button released outside, or window blur: nudge stops.

## Details
- Rust helpers and timing: [references/rust-simulated-time.md](references/rust-simulated-time.md) (read when writing clock-core tests).
- UI mocking and events: [references/ui-vitest.md](references/ui-vitest.md) (read when writing UI tests).

## Checklist
- [ ] Test failed before the code existed.
- [ ] Deterministic: no sleeps, no wall clock, no real ports.
- [ ] `just precommit` green.

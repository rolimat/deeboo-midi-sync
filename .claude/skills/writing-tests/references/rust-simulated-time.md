# Rust simulated time

Tests drive `Engine` with explicit `now_us`; nothing reads a clock.

## Helpers (`crates/clock-core/tests/common/mod.rs`)
- `clock_pulses(e: &mut Engine, start_us: u64, n: u64, interval_us: u64) -> Vec<u8>` feeds `n` x `[0xF8]` spaced by `interval_us` and returns all output bytes concatenated.
- `run_with_ticks(e, start, n, interval) -> u64` feeds `n` input pulses and returns the count of output `0xF8`. Before each next input pulse it fires `on_tick` at every deadline strictly before that pulse time; after the last input it treats the next pulse time as `last + interval`.

## Numbers
- 125 BPM = exactly 20 000 µs per pulse; 100 BPM = 25 000 µs.
- Signal timeout 500 000 µs: assert `next_deadline_us() == Some(last + 500_000)` before loss, then `on_tick(last + 500_001)` yields `[0xFC]` and a later tick is empty.
- 4 % nudge: one extra or dropped pulse per 25 inputs (100 in -> 104 / 96 out); the extra pulse is due at `t + 10_000` after the 25th input.
- BPM assertions use a tolerance: `(bpm - 125.0).abs() < 0.05`.

## Example
```rust
let mut e = Engine::new(4.0);
clock_pulses(&mut e, 0, 48, 20_000);
let s = e.state(47 * 20_000);
assert!(s.in_ok);
assert_eq!(s.input_beats, 2);
```
In tests, `unwrap` is allowed.

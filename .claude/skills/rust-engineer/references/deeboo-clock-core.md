# clock-core API shape

Spec §7.1. Inputs are `(timestamp, bytes)` and commands; outputs are bytes to send, a deadline
for a pulse to schedule, and state.

```rust
Engine::new(nudge_strength_pct: f64)
on_midi(now_us, bytes: &[u8]) -> Output       // scan every byte of the packet
on_command(now_us, Command) -> Output         // Play, Pause, NudgeStart(Direction), NudgeStop, SetNudgeStrength
on_tick(now_us) -> Output                     // due extra pulse, signal-loss Stop
next_deadline_us() -> Option<u64>             // earliest of extra pulse and signal-loss deadline
state(now_us) -> State { bpm, beat, playing, in_ok, input_beats }
```
`Output { send: Vec<u8> }` holds bytes in order. `BpmMeter` is `pub(crate)` in `bpm.rs`.
Signal timeout constant: `SIGNAL_TIMEOUT_US` = 500 000.

## Adding a state field
1. Write the failing test in `crates/clock-core/tests/` using simulated time.
2. Add the field to `State` (and `Engine` internals); compute it in `state()`.
3. Reset it wherever Pause, incoming Stop and clock loss reset state.
4. Thread it through `StateDto` in `src-tauri/src/commands.rs` (camelCase) and `ui/src/api.ts` `AppState`.
5. Update `midi-clock-domain` and this file in the same commit.

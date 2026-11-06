# Tauri app conventions

## Commands and events
- JS-visible command names: `play`, `pause`, `nudge_start(direction: "push" | "slow")`, `nudge_stop`, `get_settings`, `save_settings`, `quit`.
- State event: `deeboo://state` at about 30 Hz, payload `StateDto` with camelCase fields: `bpm`, `beat`, `playing`, `inOk`, `outOk`, `inputBeats`.
- `inOk = state.in_ok && in_port_ok`; `outOk = state.playing && out_port_ok`.
- `Settings` is camelCase JSON in `app_config_dir()/settings.json`; `validate` trims names (empty or > 63 chars is an error) and rejects strength outside 1.0-10.0; `load` runs `validate` and falls back to defaults (with `log::warn!`) on an invalid file, and `Engine::new` clamps strength to 1.0-10.0 (non-finite gives 4.0); `save` writes a temp file then renames.
- Popover hiding (`Focused(false)`) sends `NudgeStop`.

## Lock scope in the MIDI callback
Shared `Arc<Mutex<Engine>>` and `Arc<Mutex<Option<MidiOutputConnection>>>`. The input callback calls
`on_midi(clock.now_us(), msg)` and writes `Output.send` inside the callback, with no UI or channel
hop. `command()` and the timer thread do the same, all through `Shared::run`, which writes to Out
while still holding the engine lock so bytes leave in engine order. Lock order is always
engine → out; `rename_ports` never takes the engine lock. A failed port creation logs `log::error!` and
clears that port's ok flag; it never panics.

## Time base
One monotonic clock (`Instant` since app start, in µs, `MonoClock`) feeds `clock-core` for every
callback and timer event. midir timestamps are not mixed in: two clocks break deadline and
signal-loss arithmetic. Only the jitter probe uses midir/CoreMIDI timestamps, on purpose.

## Timer thread
Sleeps until `min(next_deadline_us, now + 5 ms)`, then `on_tick(now)` and sends the result.

# Tauri app conventions

## Commands and events
- JS-visible command names: `play`, `pause`, `nudge_start(direction: "push" | "slow")`, `nudge_stop`, `get_settings`, `save_settings`, `quit`.
- State event: `deeboo://state` at about 30 Hz, payload `StateDto` with camelCase fields: `bpm`, `beat`, `playing`, `inOk`, `outOk`, `inputBeats`.
- `inOk = state.in_ok && in_port_ok`; `outOk = state.playing && out_port_ok`.
- `Settings` is camelCase JSON in `app_config_dir()/settings.json`; `validate` trims names (empty or > 63 chars is an error) and rejects strength outside 1.0-10.0; `load` runs `validate` and falls back to defaults (with `log::warn!`) on an invalid file, and `Engine::new` clamps strength to 1.0-10.0 (non-finite gives 4.0); `save` writes a temp file then renames.
- Popover hiding (`Focused(false)`) sends `NudgeStop`.

## MIDI platform layer and shared state
`midi.rs` wraps `coremidi`: `MidiClient` (one per app), `MidiIn` (virtual destination), `MidiOut` (virtual
source); dropping a port removes it. Off macOS a stub makes `MidiClient::new` return `Err`, so Docker
compiles everything and the bridge reports both ports down. `coremidi_e2e.rs` (macOS only) tests the
bridge against the real CoreMIDI server; run it with `just check-macos`.

Shared state: `Arc<Mutex<Engine>>` and `Mutex<Option<MidiOut>>`. The input callback gets each packet as
`(stamp ticks, bytes)` and calls `Shared::on_packet`: a non-zero stamp becomes µs and
`e.on_midi(event_time_us(stamp, now), bytes)` runs through `Shared::run`, which maps
`Output.send` to `(us_to_ticks(at_us), byte)` and writes them with one `MidiOut::send` while still
holding the engine lock, so bytes leave in engine order. `command()` and the timer thread use `run` too.
Lock order is always engine → out; `rename_ports` never takes the engine lock. A failed client or port
logs `log::error!` and clears that port's ok flag; it never panics. A failed send logs `log::warn!`.

## Time base
One clock, `HostClock`: CoreMIDI host time (`mach_absolute_time`) in µs, feeding `clock-core` for
every callback and timer event. CoreMIDI packet stamps are the same clock, so they are used
directly and re-emitted on Out (see `event_time_us`: stamps in the future or older than the signal timeout fall back to
now). `Timebase` converts ticks to µs rounding down and µs to ticks rounding up, so
`us_to_ticks(ticks_to_us(t)) <= t` (a stamp never lands after now). Off macOS the ticks are
nanoseconds since an `Instant`.

## Timer thread
Sleeps until `min(next_deadline_us, now + 5 ms)`, then `on_tick(now)` and sends the result.

# Styles and copy

## Tokens (`:root` in `ui/src/styles.css`, from the approved mockup)
`--bg:#1c1d24` `--card:#262833` `--border:#33364a` `--text:#eee` `--dim:#8a8da0`
`--beat-off:#3a3d50` `--beat-on:#ff7a3d` `--beat-one:#ff3d3d` `--green:#2fb36b` `--green-dark:#1f6b43` `--red:#e5484d`.
Radius 12px on cards, 16px on the panel. Popover width 340px, dark, rounded cards (spec §6).

## Component classes
- Beat cell: base cell, `on` (lit), `one` (the red first beat).
- Port dot: `.dot` + `ok` (green) or `err` (red). Also `.bpm-row` (number + `BPM` label, baseline-aligned), `.bpm`, `.beats`, `.ctl`, `.btn` (`.play`, `.quit`), `.field`, `.error`, `.back`, `.icon-btn`.
- Play button: dark green (`--green-dark`, light icon) when paused; `.playing` adds bright `--green` with ⏸.
- Nudge button `.active` while held: `--beat-on` background with glow, dark icon (added with `nudgeStart`, removed in the stop path).
- Panel `#app` has no min-height: `fit-window.ts` observes it (`ResizeObserver`) and calls `setSize(340, height)`; `tauri.conf.json` height (330) is only the first-frame guess.
- Cards: BPM card and ports card.
- Class names introduced later are added here by the task that adds them.

## Strings (English only)
- Header: `Deeboo MIDI Sync`, with a ⚙ button (aria `Settings`).
- BPM card: number with 1 decimal or `—`; label `BPM`; buttons `◀◀` (aria `Slow down`), `▶`/`⏸` (aria `Play`/`Pause`), `▶▶` (aria `Push`).
- Ports card: `In · <inputName>`, `Out · <outputName>`.
- Settings: `‹ Settings`, `MIDI PORTS`, `Input name`, `Output name`, `NUDGE`, `Strength while held` (number 1-10, step 0.5, `%`), `Launch at login`, `Quit Deeboo`.
- Validation errors from `saveSettings` are shown inline under the form, e.g. `Input name cannot be empty`.
- Defaults: `Deeboo Clock Sync In`, `Deeboo Clock Sync Out`.

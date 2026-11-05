# UI tests (Vitest + jsdom)

- Mock the whole `api` module so no Tauri code runs: `vi.mock("./api")`, with `play`, `pause`, `nudgeStart`, `nudgeStop`, `getSettings`, `saveSettings`, `quit` as `vi.fn()` and `onState` resolving to an unsubscribe function. Mock `@tauri-apps/api` the same way for `main.ts`.
- Mount into a fresh `<div id="app">` per test; call the returned `update(state)` with a full `AppState` (`bpm`, `beat`, `playing`, `inOk`, `outOk`, `inputBeats`).
- Query by `aria-label` (`Slow down`, `Play`/`Pause`, `Push`) and by class (`on`, `one`, `ok`, `err`), not by position.
- Hold-to-nudge: dispatch `new Event("pointerdown")` / `"pointerup"` / `"pointerleave"` on the button, and `window.dispatchEvent(new Event("blur"))`; assert `nudgeStart("push"|"slow")` then `nudgeStop`.
- Disabled while paused: assert the `disabled` property of the nudge buttons.
- Errors from `saveSettings`: `mockRejectedValue("Input name cannot be empty")`, await, assert the text is visible.
- Clean up listeners between tests (unmount, or `vi.restoreAllMocks()`).

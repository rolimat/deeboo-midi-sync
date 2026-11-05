---
name: ui-code
description: Use when editing anything in ui/: TypeScript views, api.ts, styles.css, index.html, or any user-visible text in the popover and settings.
---

# UI code

## Rules
- TypeScript `strict: true`; no `any`. ESLint, Prettier and `tsc` are gates in `just lint` / `fmt-check`.
- No frameworks: plain TS modules and DOM APIs. No CSS framework: plain CSS with custom properties (tokens) in `ui/src/styles.css`; no hard-coded colours or radii outside the token block.
- All text is English. Never name third-party software (Traktor, Ableton...).
- Icon-only buttons (`◀◀`, `▶`/`⏸`, `▶▶`) need an `aria-label` (`Slow down`, `Play`/`Pause`, `Push`).
- Port rows are a green/red dot (`ok`/`err`) plus `In · <name>` / `Out · <name>`; no status text.
- BPM has 1 decimal, `—` when `null`. Beats 1-4: cell 1 red (`one`), 2-4 orange, all off when paused.
- The UI only talks to Rust through `ui/src/api.ts`; views never import `@tauri-apps/api` directly.
- Nudge buttons act while held: start on pointerdown; stop on pointerup, pointerleave and window blur. Disabled while paused.
- Tests are Vitest + jsdom with `api` mocked; see the `writing-tests` skill.

## Structure
`api.ts` typed wrappers; `view-main.ts` (`mountMain`), `view-settings.ts` (`mountSettings`); `main.ts` bootstraps and swaps views (`dom.ts` has the `el` helper); settings save on each field `change`, serialized; `styles.css` tokens + components.
Tokens, component classes and every UI string: [references/styles-and-copy.md](references/styles-and-copy.md) (read before changing styles or copy).

## Checklist
- [ ] New text is in English and listed in the reference.
- [ ] New colour or size is a token.
- [ ] Interactive control has an accessible name and a test.
- [ ] A new command or state field is added to `api.ts` types and to the Rust DTO together.

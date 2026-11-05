# Deeboo MIDI Sync Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A macOS menu bar app that relays MIDI clock from a virtual `Deeboo In` port to a virtual `Deeboo Out` port, with Play/Pause, hold-to-nudge (vinyl style), BPM and 1–4 beat display, shipped as an unsigned `.dmg`, built in a Dockerised dev environment with linters, git hooks and CI.

**Architecture:** All timing logic lives in a pure, deterministic Rust crate `clock-core` (bytes + timestamps in → bytes out + state), tested in Docker with simulated time. A Tauri 2 app (`src-tauri`) wires `clock-core` to `midir` virtual ports, a timer thread, the tray icon and a borderless popover window. The UI (`ui/`) is TypeScript + Vite with plain CSS, talking to Rust through Tauri commands and one state event.

**Tech Stack:** Rust (stable, pinned), Tauri 2, midir, tauri-plugin-positioner, tauri-plugin-autostart, TypeScript, Vite, Vitest, ESLint, Prettier, Docker + docker compose, just, lefthook, cargo-deny, hadolint, GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-11-05-deeboo-midi-sync-design.md` (Spanish). Read it before starting any task.

## Global Constraints

- All user-visible app text in **English**. No app text names Traktor, Ableton or any specific software.
- Default port names: `Deeboo In` (virtual destination) and `Deeboo Out` (virtual source).
- MIDI clock: 24 PPQN, `0xF8` clock, `0xFA` Start, `0xFB` Continue, `0xFC` Stop, `0xF2` SPP.
- No clock for **500 ms** = "no signal".
- Nudge strength default **4 %**, allowed range **1–10 %**.
- BPM is shown with 1 decimal place. With no signal it is shown as `—`.
- Only Deeboo's Play starts the slave. Incoming `Start`/`Continue`/`SPP` are **never** forwarded. Incoming `Stop` while playing → forward `Stop` and pause. Clock loss while playing → send `Stop` and pause. Deeboo starts **paused**.
- Popover buttons are icon-only: `◀◀`, `▶`/`⏸`, `▶▶`. Port rows show only a green/red dot and `In · <name>` / `Out · <name>`.
- Menu bar shows `[vinyl icon][dot] 124.0`. The dot is green with input signal and red without. It blinks once per **input** beat (every 24 received pulses).
- No Dock icon (`ActivationPolicy::Accessory` + `LSUIElement`).
- No code signing or notarization. The `.dmg` is built by `tauri build`.
- Plain CSS with custom properties. No CSS framework, no UI framework.
- Everything except running the app with real MIDI, building the DMG and the jitter probe runs inside Docker.
- The lefthook **pre-commit** hook runs format check, lint **and tests**.
- Repo license is GPL-3.0. `cargo-deny` must accept only licenses compatible with it.
- Agent guidance uses progressive disclosure: `AGENTS.md` ≤ 80 lines, each `SKILL.md` ≤ 100 lines, details in `references/`. `CLAUDE.md` imports `AGENTS.md`. Every task keeps the skills in sync with what it changes (Task 2).
- **CI from phase 0:** Task 1 creates the CI. All work happens on branch `feat/v1`, never directly on `main`. **Every task ends with commit → `just ship`.** `just ship` pushes and waits for the GitHub Actions run of that exact commit. A task is done only when CI is green. If CI is red, fix it in a new commit and run `just ship` again before starting the next task.
- **Time base (plan decision):** the app feeds `clock-core` timestamps from one monotonic clock (`Instant` since app start, in µs) for every MIDI callback and timer event. midir's own timestamps are not mixed in, because two clocks would break the arithmetic for deadlines and signal loss. The jitter probe (Task 12) uses midir/CoreMIDI timestamps on purpose, because it measures what the receiver actually sees.

## Review Focus

- **Clock bytes packed together or mixed with other messages** (`[0xF8, 0xF8]`, `[0x90, 60, 100, 0xF8]`, `0xF8` inside SysEx): every `0xF8` must count, and nothing else may count. Test: Task 3, `counts_every_clock_byte_in_mixed_packet`.
- **Master changes tempo abruptly** (125 → 100 BPM): the BPM shown converges within one beat, and nudge timing uses the new interval. Test: Task 3, `bpm_follows_tempo_change_within_one_beat`.
- **Clock lost while a nudge is held**: Deeboo pauses, sends exactly one `Stop`, and no inserted pulse leaks out afterwards. Test: Task 5, `signal_loss_during_push_cancels_pending_pulse`.
- **Nudge button released outside the button, or popover hides mid-nudge**: the nudge must stop. Without that the slave drifts forever. Tests: Task 9, `stops_nudge_on_pointerleave` and `stops_nudge_on_window_blur`.
- **Duplicate or zero-interval timestamps** (two `0xF8` in one packet, same µs): no division by zero, no NaN BPM. Test: Task 3, `zero_interval_does_not_produce_nan`.

---

## File Structure

```
.devcontainer/devcontainer.json      # opens repo in the dev image
docker/Dockerfile                    # dev/CI image
compose.yaml                         # `dev` service, repo mounted at /work
rust-toolchain.toml                  # pinned toolchain + rustfmt, clippy
Cargo.toml                           # workspace: crates/clock-core, src-tauri, tools/jitter-probe
deny.toml                            # cargo-deny config
justfile                             # all dev commands
lefthook.yml                         # pre-commit: just precommit
.hadolint.yaml
crates/clock-core/src/lib.rs         # pub API re-exports, constants
crates/clock-core/src/bpm.rs         # BpmMeter
crates/clock-core/src/engine.rs      # Engine: parser, transport, beat counter, nudge
crates/clock-core/tests/*.rs         # behaviour tests with simulated time
src-tauri/Cargo.toml, build.rs, tauri.conf.json, Info.plist, capabilities/default.json
src-tauri/icons/                     # app icons + tray frames
src-tauri/src/main.rs                # entry → lib::run()
src-tauri/src/lib.rs                 # Tauri builder, plugins, setup, window/tray wiring
src-tauri/src/clock.rs               # monotonic µs clock
src-tauri/src/bridge.rs              # midir virtual ports + Engine + timer thread
src-tauri/src/settings.rs            # Settings load/save/validate
src-tauri/src/commands.rs            # Tauri commands + StateDto
src-tauri/src/tray.rs                # tray frame selection (pure) + tray updater
ui/package.json, vite.config.ts, tsconfig.json, eslint.config.js, .prettierrc
ui/index.html
ui/src/main.ts                       # bootstraps views, subscribes to state
ui/src/api.ts                        # typed wrappers over invoke/listen
ui/src/view-main.ts                  # main view render + hold-to-nudge
ui/src/view-settings.ts              # settings form
ui/src/styles.css                    # tokens + components
ui/src/*.test.ts                     # Vitest (jsdom)
tools/jitter-probe/src/{lib.rs,main.rs}
.github/workflows/ci.yml             # Task 1 (phase 0), extended by Task 6
.github/workflows/release.yml        # Task 11, tag v*
.github/actions/macos-build/         # shared macOS build steps (Task 11)
.github/dependabot.yml               # Task 11
README.md                            # user + dev docs
AGENTS.md                            # level 1 agent guide (source of truth, ≤ 80 lines)
CLAUDE.md                            # `@AGENTS.md` import for Claude Code
.claude/skills/<skill>/SKILL.md      # level 2: rules + checklist (≤ 100 lines)
.claude/skills/<skill>/references/   # level 3: detail, loaded on demand
scripts/check-agent-docs.sh          # enforces the token budget in `just lint`
```

---

### Task 1: Dockerised dev environment, workspace and quality gates

**Files:**
- Create: `docker/Dockerfile`, `compose.yaml`, `.devcontainer/devcontainer.json`, `rust-toolchain.toml`, `Cargo.toml`, `deny.toml`, `justfile`, `lefthook.yml`, `.hadolint.yaml`, `.dockerignore`
- Create: `crates/clock-core/Cargo.toml`, `crates/clock-core/src/lib.rs`
- Create: `.github/workflows/ci.yml`
- Modify: `.gitignore` (add `target/`, `ui/node_modules/`, `ui/dist/`)

**Interfaces:**
- Produces: `just fmt | fmt-check | lint | test | precommit | check | dev | dmg | jitter`. Recipes that need Docker re-run themselves through `docker compose run --rm dev just <recipe>` when `DEEBOO_IN_CONTAINER` is unset. `dev`, `dmg` and `jitter` run on the macOS host and fail with a clear message on any other OS.
- Produces: `just ship`. It runs `git push -u origin HEAD`, then polls `gh run list --commit $(git rev-parse HEAD) --json databaseId` for up to 60 s until the run exists, then runs `gh run watch --exit-status <id>`. It exits non-zero when CI fails.
- Produces `ci.yml`, which later tasks extend and never replace:
  - It runs on `push` (all branches) and on `pull_request` to `main`, with `concurrency` set to cancel superseded runs on the same ref.
  - Job `linux` builds `docker/Dockerfile` with `docker/build-push-action` and the GHA cache, then runs `docker compose run --rm dev just precommit`. This makes CI identical to the local hook.
  - Job `macos` (`macos-latest`) installs the toolchain from `rust-toolchain.toml` and `just`, then runs `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace`. Task 6 adds the UI and the DMG build to this job.
  - Pin every action by full commit SHA.
- Produces: crate `clock-core` (lib name `clock_core`) with `pub const PPQN: u64 = 24;` and `pub const SIGNAL_TIMEOUT_US: u64 = 500_000;`.

- [ ] **Step 1: Write `rust-toolchain.toml`.** Pin the exact latest stable version at implementation time (e.g. `channel = "1.NN.0"`) with components `rustfmt` and `clippy`.
- [ ] **Step 2: Write `docker/Dockerfile`.** Base `rust:<same version>-bookworm`. Install the apt packages `build-essential pkg-config libssl-dev libwebkit2gtk-4.1-dev libxdo-dev libayatana-appindicator3-dev librsvg2-dev libasound2-dev curl`. Install Node 26 (NodeSource). Install pinned `just`, `cargo-deny`, `lefthook` and `hadolint` binaries. Set `ENV DEEBOO_IN_CONTAINER=1` and `WORKDIR /work`. Pin package versions where hadolint asks for it.
- [ ] **Step 3: Write `compose.yaml`.** Service `dev` built from `docker/Dockerfile`. Mount the repo at `/work`. Use named volumes for `/usr/local/cargo/registry`, `/work/target` and `/work/ui/node_modules`, so Linux build artefacts never mix with macOS host builds. Also write `.devcontainer/devcontainer.json` pointing at that compose service.
- [ ] **Step 4: Write the workspace `Cargo.toml`.** Members are `crates/clock-core` only for now; later tasks add theirs. Set `resolver = "3"` and `[workspace.lints.clippy] pedantic = { level = "warn", priority = -1 }`. Each crate sets `lints.workspace = true`.
- [ ] **Step 5: Write the `clock-core` skeleton** with the two constants above and this test in `crates/clock-core/src/lib.rs`:

```rust
#[test]
fn constants_match_spec() {
    assert_eq!(PPQN, 24);
    assert_eq!(SIGNAL_TIMEOUT_US, 500_000);
}
```

- [ ] **Step 6: Write `justfile`.**
  - `fmt-check` = `cargo fmt --all --check`, plus `npm --prefix ui run format:check` once `ui/` exists. Skip with `[ -d ui ]`.
  - `lint` = `cargo clippy --workspace --all-targets -- -D warnings`, `cargo deny check`, `hadolint docker/Dockerfile`, plus the ui lint and typecheck when `ui/` exists.
  - `test` = `cargo test --workspace`, plus `npm --prefix ui test -- --run` when `ui/` exists.
  - `precommit` = `fmt-check lint test`.
  - `check` = `precommit`, run in Docker.
- [ ] **Step 7: Write `deny.toml`.** Allow licenses `MIT`, `Apache-2.0`, `Apache-2.0 WITH LLVM-exception`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `Unicode-3.0`, `Zlib`, `MPL-2.0`, `GPL-3.0`, `CC0-1.0`. Deny advisories at `vulnerability`. Add more licenses only when a real dependency needs one and it is GPL-3.0 compatible. Note each addition in the commit message.
- [ ] **Step 8: Write `lefthook.yml`.** One `pre-commit` command, `just precommit`.
- [ ] **Step 9: Verify.**
  - Run `docker compose build dev && just precommit` on the host. Expected: rustfmt, clippy, deny and hadolint are clean, and `constants_match_spec ... ok`.
  - Then run `lefthook install` and make an empty test commit. Expected: the hook runs `just precommit`.
- [ ] **Step 10: Write `ci.yml`.** Validate the workflows with `docker run --rm -v "$PWD:/repo" -w /repo rhysd/actionlint:<pinned tag>`. Expected: no output.
- [ ] **Step 11: Prerequisite (user).** `gh auth status` must show a logged-in account with push access to `rolimat/deeboo-midi-sync`. If it doesn't, stop and ask the user to run `! gh auth login`.
- [ ] **Step 12: Commit** with `chore: dockerised dev environment, quality gates and CI` on a new branch `feat/v1`, then run `just ship`. Expected: the `linux` and `macos` jobs are both green on GitHub.

---

### Task 2: Agent guidance (AGENTS.md / CLAUDE.md), project skills and README skeleton

The goal is to give any coding agent (Claude Code, Codex, …) the project rules while spending as few tokens as possible. This uses **progressive disclosure** in three levels:

1. **Level 1, always loaded:** `AGENTS.md` and `CLAUDE.md`. They are tiny: what the project is, the hard rules, the commands, and a map that says which skill covers what.
2. **Level 2, loaded when the task matches:** each skill's `SKILL.md`. Its frontmatter `description` is the only part that sits permanently in context. The body is short, and holds the rules and a checklist.
3. **Level 3, loaded on demand:** `references/*.md` inside each skill. These are detailed patterns and examples, read only when the SKILL.md body points to them for the case at hand.

The guidance must **not duplicate the spec**. Skills link to spec sections (`docs/superpowers/specs/…#5-comportamiento`) and to code, instead of restating them.

**Files:**
- Create: `AGENTS.md`. This is the single source of truth.
- Create: `CLAUDE.md`. Its whole content is `@AGENTS.md` plus one line saying Claude Code skills live in `.claude/skills/`.
- Create: `.claude/skills/midi-clock-domain/SKILL.md` + `references/transport-and-nudge.md`
- Create: `.claude/skills/rust-code/SKILL.md` + `references/clock-core.md`, `references/tauri-app.md`
- Create: `.claude/skills/writing-tests/SKILL.md` + `references/rust-simulated-time.md`, `references/ui-vitest.md`
- Create: `.claude/skills/ui-code/SKILL.md` + `references/styles-and-copy.md`
- Create: `.claude/skills/dev-workflow/SKILL.md` + `references/docker-and-macos.md`, `references/ci-and-release.md`
- Create: `scripts/check-agent-docs.sh`
- Create: `README.md`. Overwrite the 2-line stub with sections `What it is`, `Install`, `Usage`, `Development`, `Measuring jitter`. The sections Task 11 and Task 12 own get a one-line `TODO(task-N)` placeholder, which those tasks replace.
- Modify: `justfile`, so that `lint` also runs `scripts/check-agent-docs.sh`.

**Interfaces:**
- `AGENTS.md` sections, ≤ 80 lines total:
  1. **Project:** one paragraph, plus links to the spec and the plan.
  2. **Hard rules:** this plan's Global Constraints condensed into one line each.
  3. **Commands:** the `just` recipes, and which ones run in Docker and which on macOS.
  4. **Layout:** the directory map, one line per directory.
  5. **Skills index:** a table of skill → when to use it. Agents without skill support are told to open the `SKILL.md` when the "when" column matches.
  6. **Done means:** `just precommit` is green, the tests were written first, and the commit follows Conventional Commits.
- Skill frontmatter: `name` (kebab-case, equal to the directory name) and a `description` of ≤ 250 chars that starts with "Use when …" and names concrete triggers.
- Skill contents:

| Skill | Triggers (description) | Body (≤ 100 lines) | references/ |
|---|---|---|---|
| `midi-clock-domain` | touching clock bytes, transport, BPM, nudge, beat counter | byte table, PPQN, the transport rules as 6 bullets linking spec §5 | full nudge algorithm and edge cases (signal loss, pending pulse) |
| `rust-code` | writing or reviewing Rust in `crates/` or `src-tauri/` | `clock-core` stays pure (no I/O, no clock, no threads); no `unwrap`/`expect` outside tests; errors are logged, never panic; clippy pedantic; one responsibility per module | `clock-core.md`: API shape and how to add a state field. `tauri-app.md`: commands/events naming (`deeboo://state`, camelCase DTOs), lock scope in the MIDI callback, the time base rule |
| `writing-tests` | adding features or fixing bugs, any language | TDD loop; what to test where (logic → clock-core with simulated time; app → pure helpers; UI → Vitest + jsdom with mocked `api`); test naming `snake_case_behaviour`; no real MIDI in automated tests | `rust-simulated-time.md`: the `clock_pulses` / `run_with_ticks` helpers, 125 BPM = 20 000 µs. `ui-vitest.md`: mocking `api.ts`, pointer/blur events |
| `ui-code` | editing `ui/` | TS strict, no frameworks, plain CSS with tokens, English copy, icon-only buttons need `aria-label`, never name third-party software | `styles-and-copy.md`: the token list, component classes, every UI string |
| `dev-workflow` | env setup, running commands, hooks, CI, releasing | Docker vs macOS host, `just` recipes, hook = `just precommit`, Conventional Commits, never `--no-verify` | `docker-and-macos.md`, `ci-and-release.md` (tagging `vX.Y.Z`, unsigned DMG notes) |

- `scripts/check-agent-docs.sh` enforces the token budget and exits non-zero when:
  - `AGENTS.md` is > 80 lines;
  - any `SKILL.md` is > 100 lines;
  - any `SKILL.md` lacks `name:` or `description:` frontmatter;
  - a `name` differs from its directory;
  - a `description` is > 250 chars;
  - a file under `references/` is not linked from its `SKILL.md`;
  - `CLAUDE.md` does not contain `@AGENTS.md`.

- [ ] **Step 1: Write a failing check.** Create `scripts/check-agent-docs.sh` with the rules above and run `just lint`. Expected: FAIL, because `AGENTS.md` is missing.
- [ ] **Step 2: Write `AGENTS.md`, `CLAUDE.md` and the 5 skills** with their references, following the table. Every rule must be traceable to the spec, to this plan's Global Constraints, or to a decision in a task. Do not invent new conventions.
- [ ] **Step 3: Write the `README.md` skeleton.**
- [ ] **Step 4: Run** `just precommit`. Expected: PASS, and `check-agent-docs.sh` prints `agent docs OK`.
- [ ] **Step 5: Check by hand that disclosure works.**
  - Expected: `wc -l AGENTS.md .claude/skills/*/SKILL.md` stays within budget.
  - Expected: in a fresh Claude Code session in the repo, `/skills` (or the skill listing) shows the 5 project skills.
- [ ] **Step 6: Commit** with `docs: AGENTS.md/CLAUDE.md, project skills with progressive disclosure, README skeleton`, then run `just ship`. Expected: CI is green.
, then run `just ship`. Expected: CI is green.

**Maintenance rule** (written into `AGENTS.md`): any task that changes a convention, command or interface updates the matching skill **in the same commit**. Tasks 3–12 must follow it. In particular, Task 11 and Task 12 replace their README placeholders, and Task 11 checks that `dev-workflow/references/ci-and-release.md` matches the real workflows.

---

### Task 3: clock-core: clock parsing, BPM and input signal

**Files:**
- Create: `crates/clock-core/src/bpm.rs`, `crates/clock-core/src/engine.rs`, `crates/clock-core/tests/input.rs`, `crates/clock-core/tests/common/mod.rs`
- Modify: `crates/clock-core/src/lib.rs`

**Interfaces:**
- Produces (exported from `clock_core`):

```rust
pub enum Direction { Push, Slow }
pub enum Command { Play, Pause, NudgeStart(Direction), NudgeStop, SetNudgeStrength(f64) }
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Output { pub send: Vec<u8> }          // bytes to write to Out, in order
#[derive(Debug, Clone, PartialEq)]
pub struct State {
    pub bpm: Option<f64>,      // None without signal or with fewer than 2 pulses
    pub beat: Option<u8>,      // 1..=4 while playing, None while paused
    pub playing: bool,
    pub in_ok: bool,           // a 0xF8 arrived within SIGNAL_TIMEOUT_US
    pub input_beats: u64,      // received 0xF8 count / 24 (drives menu-bar blink)
}
pub struct Engine;
impl Engine {
    pub fn new(nudge_strength_pct: f64) -> Self;
    pub fn on_midi(&mut self, now_us: u64, bytes: &[u8]) -> Output;
    pub fn on_command(&mut self, now_us: u64, cmd: Command) -> Output;
    pub fn on_tick(&mut self, now_us: u64) -> Output;
    pub fn next_deadline_us(&self) -> Option<u64>;
    pub fn state(&self, now_us: u64) -> State;
}
```

  `on_command` and `on_tick` are stubs returning `Output::default()` in this task. Tasks 3 and 4 fill them in.
- `bpm.rs`: `pub(crate) struct BpmMeter` with `fn record(&mut self, now_us: u64)`, `fn bpm(&self) -> Option<f64>` and `fn interval_us(&self) -> Option<u64>`. It keeps a ring of the last 24 intervals and ignores intervals ≤ 0. `bpm = 60e6 / (24 * mean_interval_us)`.
- `tests/common/mod.rs`: `pub fn clock_pulses(e: &mut Engine, start_us: u64, n: u64, interval_us: u64) -> Vec<u8>` feeds `n` × `[0xF8]` and returns all output bytes concatenated. At 125 BPM the interval is exactly **20 000 µs**.

- [ ] **Step 1: Write failing tests** in `tests/input.rs`:

```rust
#[test] fn bpm_from_steady_clock() {            // 48 pulses @ 20_000 µs
    let mut e = Engine::new(4.0); clock_pulses(&mut e, 0, 48, 20_000);
    let s = e.state(47 * 20_000);
    assert!((s.bpm.unwrap() - 125.0).abs() < 0.05); assert!(s.in_ok); assert_eq!(s.input_beats, 2);
}
#[test] fn counts_every_clock_byte_in_mixed_packet() {
    let mut e = Engine::new(4.0);
    e.on_midi(0, &[0xF8, 0xF8]); e.on_midi(20_000, &[0x90, 60, 100, 0xF8]);
    e.on_midi(40_000, &[0xF0, 0x7E, 0xF8, 0x01, 0xF7]); // realtime inside SysEx
    for i in 0..20 { e.on_midi(60_000 + i * 20_000, &[0xF8]); }
    assert_eq!(e.state(440_000).input_beats, 1);       // 4 + 20 = 24 pulses
}
#[test] fn no_signal_after_500ms() {
    let mut e = Engine::new(4.0); clock_pulses(&mut e, 0, 24, 20_000);
    let s = e.state(23 * 20_000 + 500_001); assert!(!s.in_ok); assert_eq!(s.bpm, None);
}
#[test] fn bpm_follows_tempo_change_within_one_beat() {   // 125 → 100 BPM (25_000 µs)
    let mut e = Engine::new(4.0); clock_pulses(&mut e, 0, 48, 20_000);
    clock_pulses(&mut e, 48 * 20_000, 25, 25_000);
    assert!((e.state(48 * 20_000 + 24 * 25_000).bpm.unwrap() - 100.0).abs() < 0.05);
}
#[test] fn zero_interval_does_not_produce_nan() {
    let mut e = Engine::new(4.0); e.on_midi(1_000, &[0xF8, 0xF8]);
    assert!(e.state(1_000).bpm.map_or(true, |b| b.is_finite()));
}
#[test] fn paused_engine_forwards_nothing() {
    let mut e = Engine::new(4.0); assert!(clock_pulses(&mut e, 0, 24, 20_000).is_empty());
}
```

- [ ] **Step 2: Run** `just test`. Expected: compile errors or FAIL, because `Engine` is not defined.
- [ ] **Step 3: Implement** `BpmMeter` and `Engine::{new, on_midi, state}` (plus stubs). The parser scans **every byte** of `bytes`. It acts on `0xF8`, `0xFA`, `0xFB` and `0xFC` and ignores everything else. In this task only `0xF8` matters.
- [ ] **Step 4: Run** `just test`. Expected: all 6 tests PASS, and clippy is clean.
- [ ] **Step 5: Commit** with `feat(clock-core): clock parsing, BPM meter and input signal state`, then run `just ship`. Expected: CI is green.
, then run `just ship`. Expected: CI is green.

---

### Task 4: clock-core: transport and beat counter

**Files:**
- Modify: `crates/clock-core/src/engine.rs`
- Create: `crates/clock-core/tests/transport.rs`

**Interfaces:**
- Consumes: Task 3 API.
- Produces: `on_command(Play | Pause)`, transport handling inside `on_midi`, signal-loss handling in `on_tick`, and `next_deadline_us()` returning `last_clock_us + SIGNAL_TIMEOUT_US` while playing. Rules (spec §5.2):
  - `Play` while paused → output `[0xFA]` and `playing = true`. The output pulse counter resets to 0. While playing, each forwarded `0xF8` is output and increments the counter. `beat = (out_pulses / 24) % 4 + 1`, so it is `Some(1)` immediately after Play.
  - `Pause` while playing → `[0xFC]` and `playing = false`. `Play` while playing and `Pause` while paused are no-ops (empty output).
  - Incoming `0xFC` while playing → forward `[0xFC]` and pause. Incoming `0xFA`, `0xFB` and `0xF2` (and its 2 data bytes) → never forwarded and no state change.
  - `on_tick(now)` while playing and `now - last_clock > SIGNAL_TIMEOUT_US` → `[0xFC]` and pause, exactly once. This also applies when no clock was received after Play: measure from the Play time.

- [ ] **Step 1: Write failing tests** in `tests/transport.rs`:
  - `play_sends_start_then_forwards_clock`: Play at t=0 → `send == [0xFA]`. Then 3 pulses → the output is `[0xF8; 3]` and `state.beat == Some(1)`.
  - `beat_counter_wraps_every_four_beats`: after Play and 24×5 pulses → `beat == Some(2)`, because 120 pulses = 5 beats and `5 % 4 + 1 = 2`.
  - `pause_sends_stop_and_halts_forwarding`: Pause → `[0xFC]`, the next pulses output nothing, and `beat == None`.
  - `repeated_play_or_pause_is_noop`.
  - `incoming_stop_forwards_and_pauses`: `on_midi(_, &[0xFC]) == Output { send: vec![0xFC] }` and `!playing`.
  - `incoming_start_continue_spp_never_forwarded`: while playing, `on_midi` with `[0xFA]`, `[0xFB]` and `[0xF2, 0x10, 0x00]` each returns an empty `send`, and `playing` stays true.
  - `clock_loss_while_playing_sends_single_stop`: Play, pulses until t=T, then `on_tick(T + 500_001) == [0xFC]` and `on_tick(T + 600_000)` is empty. Assert `next_deadline_us() == Some(T + 500_000)` before the loss.
  - `clock_returning_does_not_resume`: after loss, new pulses → empty output and `!playing`.
- [ ] **Step 2: Run** `just test`. Expected: the new tests FAIL.
- [ ] **Step 3: Implement** the transport rules above in `engine.rs`.
- [ ] **Step 4: Run** `just test`. Expected: PASS.
- [ ] **Step 5: Commit** with `feat(clock-core): play/pause transport, beat counter and clock-loss stop`, then run `just ship`. Expected: CI is green.
, then run `just ship`. Expected: CI is green.

---

### Task 5: clock-core: hold-to-nudge

**Files:**
- Modify: `crates/clock-core/src/engine.rs`
- Create: `crates/clock-core/tests/nudge.rs`

**Interfaces:**
- Consumes: Tasks 3–4.
- Produces: `Command::{NudgeStart(Direction), NudgeStop, SetNudgeStrength(f64)}`. `next_deadline_us()` now returns the earlier of the pending extra pulse and the signal-loss deadline. `on_tick` emits a due extra pulse.

Algorithm (spec §5.3). It only applies while playing; the nudge commands are ignored while paused.

```text
on each incoming 0xF8 while playing and nudging:
    acc += strength_pct / 100
    if acc >= 1.0:
        acc -= 1.0
        Push: forward this pulse; pending_extra = now + interval_us/2   (interval from BpmMeter;
              if unknown, skip insertion)
        Slow: drop this pulse (not forwarded, not counted)
on_tick(now): if pending_extra <= now → send [0xF8], count it, clear pending_extra
NudgeStop: acc = 0 (an already pending extra pulse still fires)
Pause / incoming Stop / clock loss: clear pending_extra and acc, nudge = None
SetNudgeStrength(p): reject p outside 1.0..=10.0 (ignored, keep previous)
```

- [ ] **Step 1: Write failing tests** in `tests/nudge.rs`. Use 4 % = one extra or dropped pulse every 25 input pulses, at 125 BPM (20 000 µs). Drive `on_tick` at `next_deadline_us()` between pulses with a helper `run_with_ticks(e, start, n, interval) -> u64` that returns the count of output `0xF8`.
  - `push_adds_one_pulse_per_25_inputs`: Play, NudgeStart(Push), 100 input pulses → 104 output pulses.
  - `extra_pulse_lands_mid_interval`: after the 25th input pulse at time t, `next_deadline_us() == Some(t + 10_000)`.
  - `slow_drops_one_pulse_per_25_inputs`: 100 inputs → 96 outputs.
  - `release_returns_to_one_to_one_and_keeps_offset`: Push for 50 inputs, NudgeStop, then 100 more inputs → total outputs 152, and from the release on, outputs == inputs.
  - `nudge_ignored_while_paused`: no Play, NudgeStart(Push), 50 pulses → 0 outputs and no deadline other than `None`.
  - `signal_loss_during_push_cancels_pending_pulse`: a pending extra exists, then `on_tick` at loss time → output is exactly `[0xFC]`, and later ticks are empty.
  - `strength_outside_range_is_ignored`: `SetNudgeStrength(0.5)` and `SetNudgeStrength(11.0)` leave 4 % behaviour unchanged. `SetNudgeStrength(10.0)` → 100 inputs give 110 outputs.
- [ ] **Step 2: Run** `just test`. Expected: FAIL.
- [ ] **Step 3: Implement** the algorithm above.
- [ ] **Step 4: Run** `just test`. Expected: all clock-core tests PASS.
- [ ] **Step 5: Commit** with `feat(clock-core): hold-to-nudge by pulse insertion and dropping`, then run `just ship`. Expected: CI is green.
, then run `just ship`. Expected: CI is green.

---

### Task 6: Tauri + UI scaffold: tray app, popover window, no Dock icon

**Files:**
- Create: `ui/package.json`, `ui/vite.config.ts`, `ui/tsconfig.json`, `ui/eslint.config.js`, `ui/.prettierrc`, `ui/index.html`, `ui/src/main.ts`, `ui/src/styles.css`, `ui/src/smoke.test.ts`
- Create: `src-tauri/Cargo.toml`, `src-tauri/build.rs`, `src-tauri/tauri.conf.json`, `src-tauri/Info.plist`, `src-tauri/capabilities/default.json`, `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`, `src-tauri/icons/*`
- Modify: `.github/workflows/ci.yml`, `Cargo.toml` (add member), `justfile` (`dev`, `dmg` recipes; `check` builds `ui/dist` before cargo so `generate_context!` compiles in Docker)

**Interfaces:**
- Produces: `pub fn run()` in `src-tauri/src/lib.rs`. Window label `"popover"`. Tray id `"main"`.
- `tauri.conf.json`:
  - `productName: "Deeboo MIDI Sync"`, `identifier: "com.deeboo.midisync"`, `build.frontendDist: "../ui/dist"`, `devUrl: "http://localhost:5173"`, `bundle.targets: ["app", "dmg"]`, `app.macOSPrivateApi: true` (needed for the transparent window).
  - One window `popover` with width 340, `visible: false`, `decorations: false`, `transparent: true`, `resizable: false`, `alwaysOnTop: true`, `skipTaskbar: true`.
- `Info.plist`: `LSUIElement = true`.
- `ui/package.json` scripts: `dev`, `build` (`tsc --noEmit && vite build`), `lint` (eslint), `typecheck`, `format`, `format:check`, `test` (vitest, jsdom), `tauri`. Dev dependencies: `@tauri-apps/cli@^2`. Dependencies: `@tauri-apps/api@^2`, `@tauri-apps/plugin-autostart@^2`. TypeScript uses `strict: true`.

- [ ] **Step 1: Write the failing UI smoke test** in `ui/src/smoke.test.ts`: importing `main.ts` into a jsdom document with `<div id="app">` renders an element with text `Deeboo MIDI Sync`. Mock `@tauri-apps/api` with `vi.mock`.
- [ ] **Step 2: Run** `just test`. Expected: the UI test FAILS.
- [ ] **Step 3: Implement the UI scaffold.** `main.ts` renders the header `Deeboo MIDI Sync`. `styles.css` defines the tokens from the approved mockup (`.superpowers/brainstorm/37723-1793895149/content/layout-v3.html`): `--bg:#1c1d24`, `--card:#262833`, `--border:#33364a`, `--text:#eee`, `--dim:#8a8da0`, `--beat-off:#3a3d50`, `--beat-on:#ff7a3d`, `--beat-one:#ff3d3d`, `--green:#2fb36b`, `--red:#e5484d`, plus a radius of 12px on cards and 16px on the panel.
- [ ] **Step 4: Implement `src-tauri`.**
  - Dependencies: `tauri` with feature `tray-icon` and `macos-private-api`, `tauri-plugin-positioner` with feature `tray-icon`, and `tauri-plugin-autostart`.
  - In `setup`, call `app.set_activation_policy(tauri::ActivationPolicy::Accessory)` (macOS cfg). Build the tray with `TrayIconBuilder::with_id("main")`, using a placeholder icon and title `—`.
  - On a left click: `tauri_plugin_positioner::on_tray_event`, then toggle the `popover` window with `move_window(Position::TrayBottomCenter)`, `show`, `set_focus`.
  - Hide the popover on `WindowEvent::Focused(false)`.
- [ ] **Step 5: Run** `just precommit` (Docker). Expected: PASS, including `cargo clippy` on `src-tauri` for Linux.
- [ ] **Step 6: Manual check on macOS** with `just dev`. Expected: an icon appears in the menu bar with no Dock icon. Clicking it shows the dark popover under the icon, and clicking elsewhere hides it.
- [ ] **Step 7: Extend CI.**
  - In `ci.yml` job `macos`: install Node 26, run `npm ci --prefix ui`, then `npm --prefix ui run tauri build -- --bundles dmg`, and upload the `.dmg` as an artifact named `deeboo-dmg`.
  - Re-run actionlint. Expected: no output.
- [ ] **Step 8: Commit** with `feat(app): tauri tray app with popover window and ui scaffold`, then run `just ship`. Expected: CI is green, and the run has a `deeboo-dmg` artifact.

---

### Task 7: MIDI bridge: virtual ports, engine wiring, timer thread

**Files:**
- Create: `src-tauri/src/clock.rs`, `src-tauri/src/bridge.rs`
- Modify: `src-tauri/Cargo.toml` (add `midir`, `clock-core = { path = "../crates/clock-core" }`, `log`, `env_logger`), `src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: the `clock_core::{Engine, Command, Output, State}` API.
- Produces:

```rust
// clock.rs
pub struct MonoClock;                       // wraps Instant captured at startup
impl MonoClock { pub fn new() -> Self; pub fn now_us(&self) -> u64; }
// bridge.rs
pub struct Bridge;                          // cheap Clone (Arc inside)
impl Bridge {
    pub fn start(in_name: &str, out_name: &str, nudge_pct: f64) -> Bridge;   // never panics on port errors
    pub fn command(&self, cmd: clock_core::Command);
    pub fn snapshot(&self) -> BridgeSnapshot;
    pub fn rename_ports(&self, in_name: &str, out_name: &str);
}
#[derive(Clone, Debug, PartialEq)]
pub struct BridgeSnapshot { pub state: clock_core::State, pub in_port_ok: bool, pub out_port_ok: bool }
```

- Wiring:
  - Shared `Arc<Mutex<Engine>>` plus `Arc<Mutex<Option<MidiOutputConnection>>>`.
  - The input is `MidiInput::new("Deeboo MIDI Sync")` with `.ignore(Ignore::None)` and `VirtualInput::create_virtual(in_name, cb, ())`. The callback locks the engine, calls `on_midi(clock.now_us(), msg)` and writes `Output.send` **inside the callback**.
  - The output is `VirtualOutput::create_virtual(out_name)`.
  - The timer thread loops: deadline = `next_deadline_us()`. Sleep until `min(deadline, now + 5 ms)`, then call `on_tick(now)` and send.
  - `command()` calls `on_command` and sends the result.
  - A failed port creation is logged with `log::error!`, and that port's flag is set false.
  - `rename_ports` drops the old connections and recreates them.
- Gate the `VirtualInput`/`VirtualOutput` usage with `#[cfg(unix)]`. midir supports both CoreMIDI and ALSA, so it compiles in Docker.

- [ ] **Step 1: Write a failing unit test** in `clock.rs`: `now_us_is_monotonic`. Two calls 2 ms apart (`thread::sleep`) differ by ≥ 2000 µs.
- [ ] **Step 2: Run** `just test`. Expected: FAIL. Then implement `MonoClock` and run again. Expected: PASS.
- [ ] **Step 3: Implement `Bridge`** as specified. There is no automated port test, because Docker has no MIDI sequencer. The logic under it is already covered by clock-core.
- [ ] **Step 4:** In `lib.rs` `setup`, start the `Bridge` with the default names and strength 4.0, and `app.manage(bridge)`.
- [ ] **Step 5: Run** `just precommit`. Expected: PASS.
- [ ] **Step 6: Manual check on macOS** with `just dev` and a clock master pointed at `Deeboo In` and a slave listening on `Deeboo Out`. Expected:
  - Both ports appear in other apps.
  - With `RUST_LOG=debug`, the app logs BPM once per second (temporary debug log; remove before commit).
  - Pressing nothing sends nothing to the slave.
- [ ] **Step 7: Commit** with `feat(app): midir virtual ports bridged through clock-core`, then run `just ship`. Expected: CI is green.
, then run `just ship`. Expected: CI is green.

---

### Task 8: Settings, commands, state event, autostart

**Files:**
- Create: `src-tauri/src/settings.rs`, `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`, `src-tauri/capabilities/default.json`

**Interfaces:**
- Consumes: `Bridge` (Task 7).
- Produces:

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)] #[serde(rename_all = "camelCase")]
pub struct Settings { pub input_name: String, pub output_name: String,
                      pub nudge_strength_pct: f64, pub launch_at_login: bool }
impl Default for Settings { /* "Deeboo In", "Deeboo Out", 4.0, false */ }
pub fn validate(s: Settings) -> Result<Settings, String>; // trims names; empty or > 63 chars → Err; pct outside 1.0..=10.0 → Err
pub fn load(path: &Path) -> Settings;                     // missing/corrupt → Default (log::warn on corrupt)
pub fn save(path: &Path, s: &Settings) -> std::io::Result<()>;  // write temp file + rename
```

- The settings file is `app.path().app_config_dir()?/settings.json`.
- Tauri commands, as the JS-visible names: `play`, `pause`, `nudge_start(direction: "push" | "slow")`, `nudge_stop`, `get_settings -> Settings`, `save_settings(settings) -> Result<Settings, String>`, `quit`.
  - `save_settings` validates and persists. It calls `bridge.rename_ports` only when the names changed, sends `SetNudgeStrength`, and enables or disables autostart through `tauri_plugin_autostart::ManagerExt` (`MacosLauncher::LaunchAgent`).
- State event `"deeboo://state"`, emitted at about 30 Hz by a thread in `lib.rs`. Payload `StateDto { bpm: Option<f64>, beat: Option<u8>, playing: bool, inOk: bool, outOk: bool, inputBeats: u64 }`. `inOk = state.in_ok && in_port_ok` and `outOk = state.playing && out_port_ok`.
- The popover hiding (`Focused(false)`) calls `bridge.command(NudgeStop)`.

- [ ] **Step 1: Write failing tests** in `settings.rs`:
  - `defaults_match_spec`.
  - `validate_trims_and_rejects_empty_names`: `"  X  "` → `"X"` and `"   "` → Err.
  - `validate_rejects_strength_out_of_range`: 0.9 and 10.1 → Err, 1.0 and 10.0 → Ok.
  - `load_missing_file_gives_defaults`.
  - `load_corrupt_file_gives_defaults`.
  - `save_then_load_roundtrips` (use `std::env::temp_dir()` with a unique subdir).
  - In `commands.rs`: `state_dto_out_ok_requires_playing_and_port`.
- [ ] **Step 2: Run** `just test`. Expected: FAIL.
- [ ] **Step 3: Implement** `settings.rs`, `commands.rs` and the emitter thread, then register the commands and the autostart plugin. On startup, load the settings and start the `Bridge` with them.
- [ ] **Step 4: Run** `just precommit`. Expected: PASS.
- [ ] **Step 5: Commit** with `feat(app): settings persistence, commands, state event and launch at login`, then run `just ship`. Expected: CI is green.
, then run `just ship`. Expected: CI is green.

---

### Task 9: UI: main view and settings view

**Files:**
- Create: `ui/src/api.ts`, `ui/src/view-main.ts`, `ui/src/view-settings.ts`, `ui/src/view-main.test.ts`, `ui/src/view-settings.test.ts`
- Modify: `ui/src/main.ts`, `ui/src/styles.css`, `ui/index.html`; delete `ui/src/smoke.test.ts`, which is replaced.

**Interfaces:**
- Consumes: the Task 8 commands and event.
- Produces (`api.ts`):

```ts
export type Direction = "push" | "slow";
export interface AppState { bpm: number | null; beat: number | null; playing: boolean; inOk: boolean; outOk: boolean; inputBeats: number }
export interface Settings { inputName: string; outputName: string; nudgeStrengthPct: number; launchAtLogin: boolean }
export const api: { play(): Promise<void>; pause(): Promise<void>; nudgeStart(d: Direction): Promise<void>;
  nudgeStop(): Promise<void>; getSettings(): Promise<Settings>; saveSettings(s: Settings): Promise<Settings>;
  quit(): Promise<void>; onState(cb: (s: AppState) => void): Promise<() => void> };
```

- `view-main.ts`: `export function mountMain(root: HTMLElement, settings: Settings): { update(s: AppState): void; setSettings(s: Settings): void }`
- `view-settings.ts`: `export function mountSettings(root: HTMLElement, initial: Settings, onBack: () => void): void`
- Layout follows the spec §6.2/§6.3 and the approved mockup v3, with every string in English:
  - Header `Deeboo MIDI Sync` with a ⚙ button.
  - BPM card: the big number with 1 decimal or `—`, the label `BPM`, 4 beat cells, and the icon-only buttons `◀◀` and `▶`/`⏸` and `▶▶`, each with an `aria-label` of `Slow down`, `Play`/`Pause` or `Push`.
  - Ports card: `In · <inputName>` and `Out · <outputName>` with a dot classed `ok` or `err`.
  - Settings view: `‹ Settings`, then `MIDI PORTS` with *Input name* and *Output name*, `NUDGE` with *Strength while held* (number input, 1–10, step 0.5, `%`), *Launch at login* toggle, and a *Quit Deeboo* button. Validation errors from `saveSettings` are shown inline under the form.

- [ ] **Step 1: Write failing tests** (Vitest + jsdom, mocking `api`):
  - `renders_bpm_with_one_decimal`: `update({bpm:124, ...})` → text `124.0`. `bpm:null` → `—`.
  - `lights_beats_up_to_current`: `beat:3` → cells 1–3 have class `on`, cell 1 also has `one`, and cell 4 has neither. `beat:null` → none lit.
  - `play_button_toggles_icon_and_command`: `playing:false` shows `▶` and a click calls `api.play`. `playing:true` shows `⏸` and a click calls `api.pause`.
  - `nudge_buttons_disabled_while_paused`.
  - `hold_push_starts_and_release_stops`: `pointerdown` on `▶▶` → `nudgeStart("push")`. `pointerup` → `nudgeStop`.
  - `stops_nudge_on_pointerleave`.
  - `stops_nudge_on_window_blur`: `window` `blur` during a hold → `nudgeStop`.
  - `port_dots_reflect_state` (`inOk`/`outOk` → `ok`/`err`) and `port_rows_show_names_only`, which checks that no "receiving" or "sending" text appears.
  - `settings_saves_and_shows_validation_error`: the mocked `saveSettings` rejects with `"Input name cannot be empty"` → that text is visible.
- [ ] **Step 2: Run** `just test`. Expected: FAIL.
- [ ] **Step 3: Implement** `api.ts`, both views and the CSS, then wire `main.ts`: load the settings, mount the main view, subscribe to state, and let ⚙ swap views.
- [ ] **Step 4: Run** `just precommit`. Expected: PASS.
- [ ] **Step 5: Manual check on macOS** with `just dev` and a master and slave connected. Run acceptance tests 2–6 and 8 from spec §10. Expected: all of them behave as described.
- [ ] **Step 6: Commit** with `feat(ui): main and settings views with hold-to-nudge`, then run `just ship`. Expected: CI is green.
, then run `just ship`. Expected: CI is green.

---

### Task 10: Menu bar icon: vinyl, signal dot, beat blink, BPM title

**Files:**
- Create: `src-tauri/src/tray.rs`, the tray frame PNGs in `src-tauri/icons/tray/` (`vinyl-green-on.png`, `vinyl-green-dim.png`, `vinyl-red.png`, each at @1x 22 px tall and @2x), and the source SVG `src-tauri/icons/tray/vinyl.svg`
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: `StateDto` / `BridgeSnapshot`.
- Produces:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum TrayFrame { GreenOn, GreenDim, Red }
pub fn tray_frame(in_ok: bool, input_beats: u64, last_beat_change_us: u64, now_us: u64) -> TrayFrame; // GreenOn for 120 ms after a beat change
pub fn tray_title(bpm: Option<f64>) -> String;   // Some(124.04) → "124.0", None → "—"
```

- The vinyl glyph comes from mockup option A (`.superpowers/.../menubar-icon.html`): a disc, inner ring, a filled hub and a grip mark. The dot sits to the right of the disc inside the same image.
- **Plan decision:** frames are non-template images. The disc is drawn in a mid-grey (`#8e8e93`), which stays legible on both light and dark menu bars, and the dot keeps its colour. The builder may refine this (spec §11).
- The ~30 Hz emitter thread from Task 8 also updates the tray. It calls `set_icon` only when the frame changes and `set_title` only when the title changes. The title uses a figure-space-padded format so the width does not jump.

- [ ] **Step 1: Write failing tests** in `tray.rs`:
  - `red_without_signal`.
  - `green_on_right_after_beat_then_dim`: at +0 ms → `GreenOn`, at +119 ms → `GreenOn`, at +121 ms → `GreenDim`.
  - `title_formats_one_decimal_and_dash`: 124.04 → `"124.0"`, 99.96 → `"100.0"`, None → `"—"`.
- [ ] **Step 2: Run** `just test`. Expected: FAIL. Implement it, then run again. Expected: PASS.
- [ ] **Step 3:** Produce the PNG frames from `vinyl.svg` (any one-off tool; commit the PNGs, not the tool) and wire up the updater.
- [ ] **Step 4: Manual check on macOS.** Expected: the menu bar shows the vinyl, a dot blinking at the master's tempo and the BPM. The dot turns red within 1 s of stopping the master. Check with both light and dark menu bars.
- [ ] **Step 5: Commit** with `feat(app): menu bar vinyl icon with beat blink and bpm`, then run `just ship`. Expected: CI is green.
, then run `just ship`. Expected: CI is green.

---

### Task 11: Release pipeline and README

**Files:**
- Create: `.github/workflows/release.yml`, `.github/dependabot.yml`
- Modify: `README.md`, `.claude/skills/dev-workflow/references/ci-and-release.md`, `justfile` (the `dmg` recipe outputs to `target/release/bundle/dmg/`)

**Interfaces:**
- `release.yml` runs on tag `v*`. It reuses the macOS build steps from `ci.yml` and attaches the `.dmg` to a GitHub Release (`softprops/action-gh-release`, pinned by SHA). Factor the shared steps into a composite action at `.github/actions/macos-build/action.yml`, used by both `ci.yml` and `release.yml`.
- `dependabot.yml`: weekly updates for `cargo` (/), `npm` (/ui), `github-actions` (/) and `docker` (/docker).
- README sections:
  - What it is.
  - Install (unsigned):
    1. Open the DMG and drag the app to Applications.
    2. Open it once. When macOS blocks it, go to *System Settings → Privacy & Security → Open Anyway*.
    3. If macOS says the app "is damaged", run `xattr -dr com.apple.quarantine "/Applications/Deeboo MIDI Sync.app"`.
  - Usage: point the master's clock output at `Deeboo In` and the slave's clock input at `Deeboo Out`, then press Play on beat 1 of the phrase and nudge with ◀◀/▶▶.
  - Development: Docker, `just` recipes, hooks (`lefthook install`), macOS-only recipes, `just ship`.

- [ ] **Step 1:** Write the composite action, `release.yml` and `dependabot.yml`, then switch `ci.yml` to the composite action. Run actionlint. Expected: no output.
- [ ] **Step 2:** Run `just dmg` locally. Expected: `target/release/bundle/dmg/Deeboo MIDI Sync_<version>_aarch64.dmg` exists.
- [ ] **Step 3:** Replace the `TODO(task-11)` placeholders in `README.md` (Install, Usage, Development) and update `.claude/skills/dev-workflow/references/ci-and-release.md` to match the real workflows.
- [ ] **Step 4: Commit** with `ci: release on tag, shared macos build action, dependabot; docs: README install and usage`, then run `just ship`. Expected: CI is green. The release workflow is first exercised when the user tags `v0.1.0`; ask before pushing any tag.

---

### Task 12: Jitter probe

**Files:**
- Create: `tools/jitter-probe/Cargo.toml`, `tools/jitter-probe/src/lib.rs`, `tools/jitter-probe/src/main.rs`
- Modify: `Cargo.toml` (add member), `justfile` (`jitter` recipe, macOS host), `README.md` (a "Measuring jitter" section)

**Interfaces:**
- `lib.rs`:

```rust
pub struct Stats { pub count: usize, pub mean_us: f64, pub stddev_us: f64, pub p99_abs_dev_us: f64 }
pub fn summarize(intervals_us: &[f64]) -> Option<Stats>;   // None if fewer than 2 intervals
```

- `main.rs` usage: `jitter-probe [--seconds N=30] [--port-name "Deeboo Probe"]`.
  - It creates a virtual input with that name, records the timestamp of each `0xF8` from midir's callback (CoreMIDI time), and computes the intervals.
  - After N seconds it prints `count, mean, stddev, p99 |dev|` in µs plus the implied BPM.
  - Procedure (README): run once with master → probe directly and once with master → Deeboo → probe, both idle and while holding ▶▶. Added jitter = stddev(via Deeboo) − stddev(direct). The target is < 1 ms.

- [ ] **Step 1: Write failing tests** in `lib.rs`:
  - `constant_intervals_have_zero_jitter`: `[20000.0; 100]` → mean 20000 and stddev 0.
  - `known_jitter`: alternating 19000 and 21000 → mean 20000 and stddev 1000.
  - `too_few_intervals_is_none`.
- [ ] **Step 2: Run** `just test`. Expected: FAIL. Implement it, then run again. Expected: PASS.
- [ ] **Step 3: Implement** `main.rs` and the `jitter` recipe.
- [ ] **Step 4: Manual run on macOS** with the procedure above. Record the four numbers in the PR description. If the added jitter is ≥ 1 ms, report it to the user. Do **not** switch to raw CoreMIDI inside this plan, because the user prefers to avoid that.
- [ ] **Step 5: Commit** with `feat(tools): jitter probe to measure added clock jitter`, then run `just ship`. Expected: CI is green.
, then run `just ship`. Expected: CI is green.

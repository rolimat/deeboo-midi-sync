import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { api, type AppState, type Settings } from "./api";
import { mountMain } from "./view-main";

vi.mock("./api", () => ({
  api: {
    play: vi.fn(() => Promise.resolve()),
    pause: vi.fn(() => Promise.resolve()),
    nudgeStart: vi.fn(() => Promise.resolve()),
    nudgeStop: vi.fn(() => Promise.resolve()),
  },
}));

const settings: Settings = {
  inputName: "Deeboo Clock Sync In",
  outputName: "Deeboo Clock Sync Out",
  nudgeStrengthPct: 4,
  launchAtLogin: false,
};
const base: AppState = {
  bpm: 124,
  beat: null,
  playing: false,
  inOk: true,
  outOk: false,
  inputBeats: 0,
};

let root: HTMLElement;
let view: ReturnType<typeof mountMain>;

beforeEach(() => {
  vi.clearAllMocks();
  document.body.innerHTML = '<div id="app"></div>';
  root = document.getElementById("app") as HTMLElement;
  view = mountMain(root, settings, vi.fn());
  view.update(base);
});
afterEach(() => {
  window.dispatchEvent(new Event("blur"));
});

const btn = (label: string) => root.querySelector(`[aria-label="${label}"]`) as HTMLButtonElement;
const cells = () => Array.from(root.querySelectorAll(".beat"));

it("renders_bpm_with_one_decimal", () => {
  expect(root.querySelector(".bpm")?.textContent).toBe("124.0");
  view.update({ ...base, bpm: null });
  expect(root.querySelector(".bpm")?.textContent).toBe("—");
});

it("lights_only_current_beat", () => {
  view.update({ ...base, beat: 1 });
  expect(cells().map((x) => x.classList.contains("on"))).toEqual([true, false, false, false]);
  expect(cells().map((x) => x.classList.contains("one"))).toEqual([true, false, false, false]);
  view.update({ ...base, beat: 3 });
  expect(cells().map((x) => x.classList.contains("on"))).toEqual([false, false, true, false]);
  expect(cells().some((x) => x.classList.contains("one"))).toBe(false);
  view.update({ ...base, beat: null });
  expect(cells().some((x) => x.classList.contains("on") || x.classList.contains("one"))).toBe(
    false,
  );
});

it("play_button_toggles_icon_and_command", () => {
  expect(btn("Play").textContent).toBe("▶");
  btn("Play").click();
  expect(api.play).toHaveBeenCalledTimes(1);
  view.update({ ...base, playing: true });
  expect(btn("Pause").textContent).toBe("⏸");
  btn("Pause").click();
  expect(api.pause).toHaveBeenCalledTimes(1);
});

it("nudge_buttons_disabled_while_paused", () => {
  expect(btn("Slow down").disabled).toBe(true);
  expect(btn("Push").disabled).toBe(true);
  view.update({ ...base, playing: true });
  expect(btn("Slow down").disabled).toBe(false);
  expect(btn("Push").disabled).toBe(false);
});

it("hold_push_starts_and_release_stops", () => {
  view.update({ ...base, playing: true });
  btn("Push").dispatchEvent(new Event("pointerdown"));
  expect(api.nudgeStart).toHaveBeenCalledWith("push");
  btn("Push").dispatchEvent(new Event("pointerup"));
  expect(api.nudgeStop).toHaveBeenCalledTimes(1);
});

it("stops_nudge_on_pointerleave", () => {
  view.update({ ...base, playing: true });
  btn("Slow down").dispatchEvent(new Event("pointerdown"));
  expect(api.nudgeStart).toHaveBeenCalledWith("slow");
  btn("Slow down").dispatchEvent(new Event("pointerleave"));
  expect(api.nudgeStop).toHaveBeenCalledTimes(1);
});

it("stops_nudge_on_window_blur", () => {
  view.update({ ...base, playing: true });
  btn("Push").dispatchEvent(new Event("pointerdown"));
  window.dispatchEvent(new Event("blur"));
  expect(api.nudgeStop).toHaveBeenCalledTimes(1);
});

it("does_not_stop_when_no_nudge_active", () => {
  btn("Push").dispatchEvent(new Event("pointerleave"));
  window.dispatchEvent(new Event("blur"));
  expect(api.nudgeStop).not.toHaveBeenCalled();
});

it("port_dots_reflect_state", () => {
  const dots = root.querySelectorAll(".dot");
  expect(dots[0]?.classList.contains("ok")).toBe(true);
  expect(dots[1]?.classList.contains("err")).toBe(true);
  view.update({ ...base, inOk: false, outOk: true });
  expect(dots[0]?.classList.contains("err")).toBe(true);
  expect(dots[1]?.classList.contains("ok")).toBe(true);
});

it("port_rows_show_names_only", () => {
  const text = root.textContent ?? "";
  expect(text).toContain("In · Deeboo Clock Sync In");
  expect(text).toContain("Out · Deeboo Clock Sync Out");
  expect(text.toLowerCase()).not.toContain("receiving");
  expect(text.toLowerCase()).not.toContain("sending");
  view.setSettings({ ...settings, inputName: "X" });
  expect(root.textContent).toContain("In · X");
});

it("nudge_button_lit_while_held", () => {
  view.update({ ...base, playing: true });
  const push = btn("Push");
  push.dispatchEvent(new Event("pointerdown"));
  expect(push.classList.contains("active")).toBe(true);
  push.dispatchEvent(new Event("pointerup"));
  expect(push.classList.contains("active")).toBe(false);
});

it("nudge_button_unlit_on_pointerleave", () => {
  view.update({ ...base, playing: true });
  const slow = btn("Slow down");
  slow.dispatchEvent(new Event("pointerdown"));
  expect(slow.classList.contains("active")).toBe(true);
  slow.dispatchEvent(new Event("pointerleave"));
  expect(slow.classList.contains("active")).toBe(false);
});

it("play_button_dark_green_when_paused", () => {
  const play = root.querySelector(".play") as HTMLElement;
  expect(play.classList.contains("playing")).toBe(false);
  view.update({ ...base, playing: true });
  expect(play.classList.contains("playing")).toBe(true);
  view.update({ ...base, playing: false });
  expect(play.classList.contains("playing")).toBe(false);
});

it("play_glyph_node_not_replaced_on_unchanged_state", () => {
  const glyph = btn("Play").querySelector(".glyph") as HTMLElement;
  const text = glyph.firstChild;
  view.update({ ...base });
  view.update({ ...base });
  expect(btn("Play").querySelector(".glyph")).toBe(glyph);
  expect(glyph.firstChild).toBe(text);
  glyph.dispatchEvent(new MouseEvent("click", { bubbles: true }));
  expect(api.play).toHaveBeenCalledTimes(1);
});

it("nudge_pointerdown_on_glyph_starts_nudge", () => {
  view.update({ ...base, playing: true });
  const glyph = btn("Push").querySelector(".glyph") as HTMLElement;
  glyph.dispatchEvent(new Event("pointerdown", { bubbles: true }));
  expect(api.nudgeStart).toHaveBeenCalledWith("push");
});

import { beforeEach, expect, it, vi } from "vitest";

vi.mock("./api", () => ({
  api: {
    getSettings: vi.fn(() =>
      Promise.resolve({
        inputName: "In",
        outputName: "Out",
        nudgeStrengthPct: 4,
        launchAtLogin: false,
      }),
    ),
    onState: vi.fn(() => Promise.resolve(() => {})),
  },
}));

vi.mock("./fit-window", () => ({ fitWindowToPanel: vi.fn() }));

beforeEach(() => {
  vi.resetModules();
  document.body.innerHTML = '<div id="app"></div>';
});

it("renders_header_and_swaps_to_settings", async () => {
  await import("./main");
  await new Promise((r) => setTimeout(r, 0));
  const app = document.getElementById("app") as HTMLElement;
  expect(app.textContent).toContain("Deeboo MIDI Sync");
  (app.querySelector('[aria-label="Settings"]') as HTMLButtonElement).click();
  expect(app.textContent).toContain("MIDI PORTS");
  (app.querySelector(".back") as HTMLButtonElement).click();
  await new Promise((r) => setTimeout(r, 0));
  expect(app.textContent).toContain("BPM");
});

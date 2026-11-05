import { beforeEach, expect, it, vi } from "vitest";

const setSize = vi.fn(() => Promise.resolve());
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => ({ setSize }) }));
vi.mock("@tauri-apps/api/dpi", () => ({
  LogicalSize: class {
    constructor(
      public width: number,
      public height: number,
    ) {}
  },
}));

let fire: () => void;
class FakeObserver {
  constructor(cb: () => void) {
    fire = cb;
  }
  observe = vi.fn();
  disconnect = vi.fn();
}

let panel: HTMLElement;
let height: number;

beforeEach(() => {
  vi.clearAllMocks();
  vi.stubGlobal("ResizeObserver", FakeObserver);
  panel = document.createElement("div");
  height = 311.2;
  panel.getBoundingClientRect = () => ({ height }) as DOMRect;
});

it("fit_window_sets_measured_height_rounded_up", async () => {
  const { fitWindowToPanel } = await import("./fit-window");
  fitWindowToPanel(panel);
  fire();
  expect(setSize).toHaveBeenCalledTimes(1);
  expect(setSize).toHaveBeenCalledWith(expect.objectContaining({ width: 340, height: 312 }));
});

it("fit_window_skips_unchanged_height", async () => {
  const { fitWindowToPanel } = await import("./fit-window");
  fitWindowToPanel(panel);
  fire();
  fire();
  expect(setSize).toHaveBeenCalledTimes(1);
  height = 400;
  fire();
  expect(setSize).toHaveBeenCalledTimes(2);
});

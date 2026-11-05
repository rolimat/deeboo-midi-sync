import { beforeEach, expect, it, vi } from "vitest";
import { api, type Settings } from "./api";
import { mountSettings } from "./view-settings";

vi.mock("./api", () => ({
  api: { saveSettings: vi.fn(), quit: vi.fn(() => Promise.resolve()) },
}));

const initial: Settings = {
  inputName: "Deeboo Clock Sync In",
  outputName: "Deeboo Clock Sync Out",
  nudgeStrengthPct: 4,
  launchAtLogin: false,
};
let root: HTMLElement;
const onBack = vi.fn();
const flush = () => new Promise((r) => setTimeout(r, 0));

beforeEach(() => {
  vi.clearAllMocks();
  document.body.innerHTML = '<div id="app"></div>';
  root = document.getElementById("app") as HTMLElement;
  mountSettings(root, initial, onBack);
});

const input = (label: string) => root.querySelector(`[aria-label="${label}"]`) as HTMLInputElement;

it("settings_saves_and_shows_validation_error", async () => {
  vi.mocked(api.saveSettings).mockRejectedValue("Input name cannot be empty");
  input("Input name").value = "";
  input("Input name").dispatchEvent(new Event("change"));
  await flush();
  expect(root.querySelector(".error")?.textContent).toBe("Input name cannot be empty");
});

it("settings_saves_changed_values", async () => {
  vi.mocked(api.saveSettings).mockImplementation((s) => Promise.resolve(s));
  input("Strength while held").value = "6.5";
  input("Strength while held").dispatchEvent(new Event("change"));
  input("Launch at login").click();
  await flush();
  expect(api.saveSettings).toHaveBeenLastCalledWith({
    ...initial,
    nudgeStrengthPct: 6.5,
    launchAtLogin: true,
  });
  expect(root.querySelector(".error")?.textContent ?? "").toBe("");
});

it("settings_back_and_quit", () => {
  (root.querySelector(".back") as HTMLButtonElement).click();
  expect(onBack).toHaveBeenCalled();
  (root.querySelector(".quit") as HTMLButtonElement).click();
  expect(api.quit).toHaveBeenCalled();
});

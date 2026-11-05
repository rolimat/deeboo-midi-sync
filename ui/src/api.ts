import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type Direction = "push" | "slow";

export interface AppState {
  bpm: number | null;
  beat: number | null;
  playing: boolean;
  inOk: boolean;
  outOk: boolean;
  inputBeats: number;
}

export interface Settings {
  inputName: string;
  outputName: string;
  nudgeStrengthPct: number;
  launchAtLogin: boolean;
}

export const api = {
  play: (): Promise<void> => invoke("play"),
  pause: (): Promise<void> => invoke("pause"),
  nudgeStart: (direction: Direction): Promise<void> => invoke("nudge_start", { direction }),
  nudgeStop: (): Promise<void> => invoke("nudge_stop"),
  getSettings: (): Promise<Settings> => invoke("get_settings"),
  saveSettings: (settings: Settings): Promise<Settings> => invoke("save_settings", { settings }),
  quit: (): Promise<void> => invoke("quit"),
  onState: (cb: (s: AppState) => void): Promise<() => void> =>
    listen<AppState>("deeboo://state", (e) => cb(e.payload)),
};

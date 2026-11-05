import { LogicalSize } from "@tauri-apps/api/dpi";
import { getCurrentWindow } from "@tauri-apps/api/window";

const WIDTH = 340;

/** Keeps the native window height equal to the panel height. */
export function fitWindowToPanel(panel: HTMLElement): void {
  let last = 0;
  const observer = new ResizeObserver(() => {
    const height = Math.ceil(panel.getBoundingClientRect().height);
    if (height === last || height === 0) return;
    last = height;
    void getCurrentWindow().setSize(new LogicalSize(WIDTH, height));
  });
  observer.observe(panel);
}

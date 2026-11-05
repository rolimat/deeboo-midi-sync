import { api, type AppState, type Direction, type Settings } from "./api";
import { el } from "./dom";

export function mountMain(
  root: HTMLElement,
  settings: Settings,
  onOpenSettings: () => void = () => {},
): { update(s: AppState): void; setSettings(s: Settings): void } {
  const header = el("div", "header");
  header.append(el("span", "title", "Deeboo MIDI Sync"));
  const gear = el("button", "icon-btn", "⚙");
  gear.setAttribute("aria-label", "Settings");
  gear.addEventListener("click", onOpenSettings);
  header.append(gear);

  const bpm = el("div", "bpm", "—");
  const beats = [0, 1, 2, 3].map(() => el("div", "beat"));
  const beatRow = el("div", "beats");
  beatRow.append(...beats);

  let active = false;
  const stopNudge = (): void => {
    if (!active) return;
    active = false;
    void api.nudgeStop();
  };
  const nudgeBtn = (icon: string, label: string, dir: Direction): HTMLButtonElement => {
    const b = el("button", "btn", icon);
    b.setAttribute("aria-label", label);
    b.addEventListener("pointerdown", () => {
      if (b.disabled) return;
      active = true;
      void api.nudgeStart(dir);
    });
    b.addEventListener("pointerup", stopNudge);
    b.addEventListener("pointerleave", stopNudge);
    b.addEventListener("pointercancel", stopNudge);
    return b;
  };
  window.addEventListener("blur", stopNudge);

  const slow = nudgeBtn("◀◀", "Slow down", "slow");
  const push = nudgeBtn("▶▶", "Push", "push");
  const play = el("button", "btn play", "▶");
  let playing = false;
  play.setAttribute("aria-label", "Play");
  play.addEventListener("click", () => void (playing ? api.pause() : api.play()));
  const ctl = el("div", "ctl");
  ctl.append(slow, play, push);

  const bpmCard = el("div", "card");
  bpmCard.append(bpm, el("div", "dim", "BPM"), beatRow, ctl);

  const inDot = el("span", "dot err");
  const outDot = el("span", "dot err");
  const inName = el("span");
  const outName = el("span");
  const inRow = el("div", "row");
  inRow.append(inDot, inName);
  const outRow = el("div", "row");
  outRow.append(outDot, outName);
  const portsCard = el("div", "card");
  portsCard.append(inRow, outRow);

  const setSettings = (s: Settings): void => {
    inName.textContent = `In · ${s.inputName}`;
    outName.textContent = `Out · ${s.outputName}`;
  };
  setSettings(settings);

  root.replaceChildren(header, bpmCard, portsCard);

  return {
    setSettings,
    update(s) {
      bpm.textContent = s.bpm === null ? "—" : s.bpm.toFixed(1);
      beats.forEach((c, i) => {
        const lit = s.beat !== null && i < s.beat;
        c.classList.toggle("on", lit);
        c.classList.toggle("one", lit && i === 0);
      });
      playing = s.playing;
      play.textContent = playing ? "⏸" : "▶";
      play.setAttribute("aria-label", playing ? "Pause" : "Play");
      slow.disabled = !playing;
      push.disabled = !playing;
      if (!playing) stopNudge();
      inDot.className = `dot ${s.inOk ? "ok" : "err"}`;
      outDot.className = `dot ${s.outOk ? "ok" : "err"}`;
    },
  };
}

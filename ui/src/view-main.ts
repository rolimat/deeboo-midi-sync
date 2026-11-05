import { api, type AppState, type Direction, type Settings } from "./api";
import { el, glyphButton } from "./dom";

const setDot = (d: HTMLElement, ok: boolean): void => {
  const cls = `dot ${ok ? "ok" : "err"}`;
  if (d.className !== cls) d.className = cls;
};

export function mountMain(
  root: HTMLElement,
  settings: Settings,
  onOpenSettings: () => void = () => {},
): { update(s: AppState): void; setSettings(s: Settings): void } {
  const header = el("div", "header");
  header.append(el("span", "title", "Deeboo MIDI Sync"));
  const gear = glyphButton("icon-btn", "⚙");
  gear.setAttribute("aria-label", "Settings");
  gear.addEventListener("click", onOpenSettings);
  header.append(gear);

  const bpm = el("div", "bpm", "—");
  const beats = [0, 1, 2, 3].map(() => el("div", "beat"));
  const beatRow = el("div", "beats");
  beatRow.append(...beats);

  let active = false;
  const held = new Set<HTMLElement>();
  const stopNudge = (): void => {
    if (!active) return;
    active = false;
    held.forEach((b) => b.classList.remove("active"));
    held.clear();
    void api.nudgeStop();
  };
  const nudgeBtn = (icon: string, label: string, dir: Direction): HTMLButtonElement => {
    const b = glyphButton("btn", icon);
    b.setAttribute("aria-label", label);
    b.addEventListener("pointerdown", () => {
      if (b.disabled) return;
      active = true;
      b.classList.add("active");
      held.add(b);
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
  const play = glyphButton("btn play", "▶");
  const playGlyph = play.firstElementChild as HTMLElement;
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
      const bpmText = s.bpm === null ? "—" : s.bpm.toFixed(1);
      if (bpm.textContent !== bpmText) bpm.textContent = bpmText;
      beats.forEach((c, i) => {
        const lit = s.beat !== null && i < s.beat;
        c.classList.toggle("on", lit);
        c.classList.toggle("one", lit && i === 0);
      });
      if (playing !== s.playing) {
        playing = s.playing;
        play.classList.toggle("playing", playing);
        playGlyph.textContent = playing ? "⏸" : "▶";
        play.setAttribute("aria-label", playing ? "Pause" : "Play");
      }
      slow.disabled = !playing;
      push.disabled = !playing;
      if (!playing) stopNudge();
      setDot(inDot, s.inOk);
      setDot(outDot, s.outOk);
    },
  };
}

import { api, type Settings } from "./api";
import { el, glyphButton } from "./dom";

export function mountSettings(root: HTMLElement, initial: Settings, onBack: () => void): void {
  let current = { ...initial };
  const error = el("div", "error");

  let queue: Promise<void> = Promise.resolve();
  const save = (patch: Partial<Settings>): Promise<void> => {
    queue = queue.then(async () => {
      try {
        current = await api.saveSettings({ ...current, ...patch });
        error.textContent = "";
      } catch (e) {
        error.textContent = String(e);
      }
    });
    return queue;
  };

  const back = glyphButton("back", "‹ Settings");
  const header = el("div", "header");
  header.append(back);
  back.addEventListener("click", onBack);

  const field = (label: string, input: HTMLInputElement): HTMLElement => {
    input.setAttribute("aria-label", label);
    const row = el("label", "row");
    row.append(el("span", "", label), input);
    return row;
  };
  const text = (value: string, key: "inputName" | "outputName"): HTMLInputElement => {
    const i = el("input", "field");
    i.type = "text";
    i.value = value;
    i.addEventListener("change", () => void save({ [key]: i.value }));
    return i;
  };

  const strength = el("input", "field num");
  strength.type = "number";
  strength.min = "1";
  strength.max = "10";
  strength.step = "0.5";
  strength.value = String(initial.nudgeStrengthPct);
  strength.addEventListener(
    "change",
    () => void save({ nudgeStrengthPct: Number(strength.value) }),
  );
  const strengthRow = field("Strength while held", strength);
  strengthRow.append(el("span", "dim", "%"));

  const login = el("input");
  login.type = "checkbox";
  login.checked = initial.launchAtLogin;
  login.addEventListener("change", () => void save({ launchAtLogin: login.checked }));

  const portsCard = el("div", "card");
  portsCard.append(
    el("div", "dim", "MIDI PORTS"),
    field("Input name", text(initial.inputName, "inputName")),
    field("Output name", text(initial.outputName, "outputName")),
  );
  const nudgeCard = el("div", "card");
  nudgeCard.append(el("div", "dim", "NUDGE"), strengthRow);
  const loginCard = el("div", "card");
  loginCard.append(field("Launch at login", login));

  const quit = el("button", "btn quit", "Quit Deeboo");
  quit.addEventListener("click", () => void api.quit());

  root.replaceChildren(header, portsCard, nudgeCard, loginCard, error, quit);
}

import "./styles.css";

function render(root: HTMLElement): void {
  const header = document.createElement("h1");
  header.className = "header";
  header.textContent = "Deeboo MIDI Sync";
  root.replaceChildren(header);
}

const root = document.getElementById("app");
if (root) render(root);

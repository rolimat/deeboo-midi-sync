import "./styles.css";
import { api } from "./api";
import { mountMain } from "./view-main";
import { mountSettings } from "./view-settings";

async function boot(root: HTMLElement): Promise<void> {
  let settings = await api.getSettings();
  let main: ReturnType<typeof mountMain>;
  let lastState: Parameters<typeof main.update>[0] | null = null;

  const showMain = (): void => {
    main = mountMain(root, settings, showSettings);
    if (lastState) main.update(lastState);
  };
  const showSettings = (): void => {
    mountSettings(root, settings, () => {
      void api.getSettings().then((s) => {
        settings = s;
        showMain();
      });
    });
  };

  showMain();
  await api.onState((s) => {
    lastState = s;
    main.update(s);
  });
}

const root = document.getElementById("app");
if (root) void boot(root);

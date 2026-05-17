import "./styles.css";
import { getConfig, listPlatformLogins } from "./api";
import { renderApp } from "./render";
import { createInitialState } from "./state";

const root = document.querySelector<HTMLDivElement>("#app");
if (!root) {
  throw new Error("Missing #app root");
}

const state = createInitialState();
renderApp(root, state);

Promise.all([getConfig(), listPlatformLogins()])
  .then(([settings, platforms]) => {
    state.settings = settings;
    state.platforms = platforms;
    renderApp(root, state);
  })
  .catch((error) => {
    console.error("Failed to load startup data", error);
  });

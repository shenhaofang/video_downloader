import "./styles.css";
import { getConfig, getToolStatus, listPlatformLogins, listTaskGroups } from "./api";
import { renderApp } from "./render";
import { createInitialState } from "./state";

const root = document.querySelector<HTMLDivElement>("#app");
if (!root) {
  throw new Error("Missing #app root");
}

const state = createInitialState();
renderApp(root, state);

Promise.all([getConfig(), listPlatformLogins(), listTaskGroups(), getToolStatus()])
  .then(([settings, platforms, taskGroups, toolStatus]) => {
    state.settings = settings;
    state.platforms = platforms;
    state.taskGroups = taskGroups;
    state.toolStatus = toolStatus;
    renderApp(root, state);
  })
  .catch((error) => {
    console.error("Failed to load startup data", error);
  });

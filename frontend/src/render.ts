import { createTask as defaultCreateTask } from "./api";
import {
  platformRowText,
  taskOutputDirectory,
  type AppState,
  type CreatedTaskGroup,
  type DownloadTask,
  type TabId,
} from "./state";

export interface RenderDependencies {
  createTask?: typeof defaultCreateTask;
}

export function renderApp(
  root: HTMLElement,
  state: AppState,
  dependencies: RenderDependencies = {},
): void {
  const createTask = dependencies.createTask ?? defaultCreateTask;
  root.replaceChildren(buildAppShell(state, createTask));
}

function buildAppShell(
  state: AppState,
  createTask: typeof defaultCreateTask,
): HTMLElement {
  const app = element("main", "app-shell");
  const sidebar = element("aside", "sidebar");
  const title = element("div", "brand", "Video Downloader");
  const nav = element("nav", "nav");

  const panels = element("section", "workspace");
  const downloads = buildDownloadsPanel(state, createTask);
  const login = buildLoginPanel(state);
  const settings = buildSettingsPanel(state);
  panels.append(downloads, login, settings);

  const tabs: Array<[TabId, string]> = [
    ["downloads", "下载任务"],
    ["login", "登录状态"],
    ["settings", "设置"],
  ];

  for (const [tabId, label] of tabs) {
    const button = element("button", "", label);
    button.type = "button";
    button.dataset.tab = tabId;
    button.setAttribute("aria-pressed", String(tabId === state.activeTab));
    button.addEventListener("click", () => {
      state.activeTab = tabId;
      app.querySelectorAll<HTMLElement>("[data-panel]").forEach((panel) => {
        panel.hidden = panel.dataset.panel !== tabId;
      });
      nav.querySelectorAll<HTMLButtonElement>("button").forEach((navButton) => {
        navButton.setAttribute("aria-pressed", String(navButton.dataset.tab === tabId));
      });
    });
    nav.append(button);
  }

  sidebar.append(title, nav);
  app.append(sidebar, panels);
  return app;
}

function buildDownloadsPanel(
  state: AppState,
  createTask: typeof defaultCreateTask,
): HTMLElement {
  const panel = element("section", "panel");
  panel.dataset.panel = "downloads";
  panel.hidden = state.activeTab !== "downloads";

  const title = element("div", "panel-heading");
  title.append(element("h1", "", "下载任务"));

  const form = element("form", "download-toolbar");
  form.append(
    field("视频链接", "video-url", "请输入 bilibili 视频或合集链接"),
    outputDirectoryField(state),
  );

  const addButton = element("button", "primary", "添加下载");
  addButton.type = "submit";
  addButton.dataset.testid = "add-task";
  form.append(addButton);
  form.addEventListener("submit", async (event) => {
    event.preventDefault();
    const url = form.querySelector<HTMLInputElement>("[data-testid='video-url']")?.value.trim() ?? "";
    const outputDir =
      form.querySelector<HTMLInputElement>("[data-testid='output-directory']")?.value ??
      taskOutputDirectory(state.settings.downloadRoot);
    const created = await createTask({ url, output_dir: outputDir, has_login: false });
    state.taskGroups.unshift(created);
    taskList.replaceChildren(...state.taskGroups.map(buildTaskGroupCard));
  });

  const taskList = element("div", "task-list");
  taskList.replaceChildren(...state.taskGroups.map(buildTaskGroupCard));
  panel.append(title, form, taskList);
  return panel;
}

function field(labelText: string, testId: string, placeholder: string): HTMLElement {
  const label = element("label", "field");
  label.append(element("span", "", labelText));
  const input = document.createElement("input");
  input.dataset.testid = testId;
  input.placeholder = placeholder;
  label.append(input);
  return label;
}

function outputDirectoryField(state: AppState): HTMLElement {
  const label = element("label", "field output-field");
  label.append(element("span", "", "输出目录"));
  const input = document.createElement("input");
  input.dataset.testid = "output-directory";
  input.value = taskOutputDirectory(state.settings.downloadRoot);
  const button = element("button", "secondary", "选择");
  button.type = "button";
  label.append(input, button);
  return label;
}

function buildTaskGroupCard(created: CreatedTaskGroup): HTMLElement {
  const card = element("article", "task-card");
  const details = document.createElement("details");
  details.open = true;
  const summary = document.createElement("summary");
  summary.className = "task-group-summary";
  summary.append(
    element("strong", "", created.group.title),
    element("span", "", created.group.output_dir),
    element("span", "state-pill", stateLabel(created.group.state)),
  );
  const children = element("div", "child-task-list");
  children.append(...created.tasks.map(buildChildTask));
  details.append(summary, children);
  card.append(details);
  return card;
}

function buildChildTask(task: DownloadTask): HTMLElement {
  const row = element("div", "child-task");
  row.append(
    element("div", "child-title", task.title),
    element("div", "child-file", fileName(task.output_file)),
    element("div", "child-progress", progressText(task)),
    element("div", "child-retry", `重试 ${task.retry_count}/${task.max_retries}`),
    element("div", "child-meta", `${task.quality ?? "自动"} · ${task.engine}`),
  );
  return row;
}

function buildLoginPanel(state: AppState): HTMLElement {
  const panel = element("section", "panel");
  panel.dataset.panel = "login";
  panel.hidden = state.activeTab !== "login";
  panel.append(element("div", "panel-heading", "平台登录"));

  const list = element("div", "platform-list");
  for (const row of state.platforms) {
    const card = element("article", "platform-card");
    const summary = element("button", "platform-summary");
    summary.type = "button";
    const [platform, status] = platformRowText(row);
    const name = element("span", "", platform);
    name.dataset.testid = "platform-name";
    const shortStatus = element("span", "platform-status", status);
    shortStatus.dataset.testid = "platform-status";
    summary.append(name, shortStatus);

    const detail = element(
      "div",
      "platform-detail",
      "后续任务会在这里接入扫码或 Cookie 登录，凭据将保存为本地加密文件。",
    );
    detail.hidden = !state.expandedPlatforms.has(row.platform);
    summary.addEventListener("click", () => {
      if (state.expandedPlatforms.has(row.platform)) {
        state.expandedPlatforms.delete(row.platform);
        detail.hidden = true;
      } else {
        state.expandedPlatforms.add(row.platform);
        detail.hidden = false;
      }
    });
    card.append(summary, detail);
    list.append(card);
  }
  panel.append(list);
  return panel;
}

function buildSettingsPanel(state: AppState): HTMLElement {
  const panel = element("section", "panel");
  panel.dataset.panel = "settings";
  panel.hidden = state.activeTab !== "settings";
  panel.append(element("div", "panel-heading", "设置"));

  const settings = element("div", "settings-grid");
  const root = field("默认根目录", "download-root", "D:\\Videos");
  root.querySelector("input")!.value = state.settings.downloadRoot;

  const concurrency = field("并发数", "concurrency", "2");
  const concurrencyInput = concurrency.querySelector("input")!;
  concurrencyInput.type = "number";
  concurrencyInput.min = "1";
  concurrencyInput.max = "8";
  concurrencyInput.value = String(state.settings.concurrency);

  const engine = element("div", "field");
  engine.dataset.testid = "default-engine";
  engine.append(element("span", "", "默认内核"));
  const segmented = element("div", "segmented");
  for (const value of ["native", "yt-dlp"] as const) {
    const button = element("button", "", value);
    button.type = "button";
    button.setAttribute("aria-pressed", String(value === state.settings.defaultEngine));
    segmented.append(button);
  }
  engine.append(segmented);
  settings.append(root, concurrency, engine);
  panel.append(settings);
  return panel;
}

function progressText(task: DownloadTask): string {
  if (!task.bytes_total) {
    return "进度 0%";
  }
  const percent = Math.round((task.bytes_downloaded / task.bytes_total) * 100);
  return `进度 ${percent}%`;
}

function fileName(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path;
}

function stateLabel(state: string): string {
  const labels: Record<string, string> = {
    queued: "排队中",
    downloading: "下载中",
    completed: "已完成",
    failed: "失败",
  };
  return labels[state] ?? state;
}

function element<K extends keyof HTMLElementTagNameMap>(
  tagName: K,
  className = "",
  text?: string,
): HTMLElementTagNameMap[K] {
  const node = document.createElement(tagName);
  if (className) {
    node.className = className;
  }
  if (text !== undefined) {
    node.textContent = text;
  }
  return node;
}

import {
  createTask as defaultCreateTask,
  runTask as defaultRunTask,
  saveConfig as defaultSaveConfig,
} from "./api";
import {
  platformRowText,
  taskOutputDirectory,
  type AppSettings,
  type AppState,
  type CreatedTaskGroup,
  type DownloadTask,
  type Engine,
  type TabId,
} from "./state";

export interface RenderDependencies {
  createTask?: typeof defaultCreateTask;
  runTask?: typeof defaultRunTask;
  saveConfig?: typeof defaultSaveConfig;
}

export function renderApp(
  root: HTMLElement,
  state: AppState,
  dependencies: RenderDependencies = {},
): void {
  const createTask = dependencies.createTask ?? defaultCreateTask;
  const runTask = dependencies.runTask ?? defaultRunTask;
  const saveConfig = dependencies.saveConfig ?? defaultSaveConfig;
  root.replaceChildren(buildAppShell(root, state, { createTask, runTask, saveConfig }));
}

function buildAppShell(
  root: HTMLElement,
  state: AppState,
  dependencies: Required<RenderDependencies>,
): HTMLElement {
  const app = element("main", "app-shell");
  const sidebar = element("aside", "sidebar");
  const title = element("div", "brand", "Video Downloader");
  const nav = element("nav", "nav");

  const panels = element("section", "workspace");
  const downloads = buildDownloadsPanel(state, dependencies.createTask, dependencies.runTask);
  const login = buildLoginPanel(state);
  const settings = buildSettingsPanel(root, state, dependencies);
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
  runTask: typeof defaultRunTask,
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
    for (const task of created.tasks) {
      const updated = await runTask({ task_id: task.id });
      replaceTask(state, updated);
      taskList.replaceChildren(...state.taskGroups.map(buildTaskGroupCard));
    }
  });

  const taskList = element("div", "task-list");
  taskList.replaceChildren(...state.taskGroups.map(buildTaskGroupCard));
  panel.append(title, form, taskList);
  return panel;
}

function replaceTask(state: AppState, updated: DownloadTask): void {
  for (const group of state.taskGroups) {
    const index = group.tasks.findIndex((task) => task.id === updated.id);
    if (index >= 0) {
      group.tasks[index] = updated;
      group.group.state = group.tasks.every((task) => task.state === "completed")
        ? "completed"
        : group.group.state;
      return;
    }
  }
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

function buildSettingsPanel(
  root: HTMLElement,
  state: AppState,
  dependencies: Required<RenderDependencies>,
): HTMLElement {
  const panel = element("section", "panel");
  panel.dataset.panel = "settings";
  panel.hidden = state.activeTab !== "settings";
  panel.append(element("div", "panel-heading", "设置"));

  const settings = element("form", "settings-grid");
  const rootField = field("默认根目录", "download-root", "D:\\Videos");
  rootField.querySelector("input")!.value = state.settings.downloadRoot;

  const concurrency = field("并发数", "concurrency", "2");
  const concurrencyInput = concurrency.querySelector("input")!;
  concurrencyInput.type = "number";
  concurrencyInput.min = "1";
  concurrencyInput.max = "8";
  concurrencyInput.value = String(state.settings.concurrency);

  const ytdlp = field("yt-dlp 路径", "ytdlp-path", "C:\\tools\\yt-dlp.exe");
  ytdlp.querySelector("input")!.value = state.settings.ytdlpPath ?? "";
  const ffmpeg = field("ffmpeg 路径", "ffmpeg-path", "C:\\tools\\ffmpeg.exe");
  ffmpeg.querySelector("input")!.value = state.settings.ffmpegPath ?? "";
  const ffprobe = field("ffprobe 路径", "ffprobe-path", "C:\\tools\\ffprobe.exe");
  ffprobe.querySelector("input")!.value = state.settings.ffprobePath ?? "";

  const engine = element("div", "field");
  engine.dataset.testid = "default-engine";
  engine.append(element("span", "", "默认内核"));
  const segmented = element("div", "segmented");
  let selectedEngine: Engine = state.settings.defaultEngine;
  for (const value of ["native", "yt-dlp"] as const) {
    const button = element("button", "", value);
    button.type = "button";
    button.dataset.testid = `engine-${value}`;
    button.setAttribute("aria-pressed", String(value === state.settings.defaultEngine));
    button.addEventListener("click", () => {
      selectedEngine = value;
      segmented.querySelectorAll("button").forEach((item) => {
        item.setAttribute("aria-pressed", String(item === button));
      });
    });
    segmented.append(button);
  }
  engine.append(segmented);

  const save = element("button", "primary", "保存");
  save.type = "submit";
  save.dataset.testid = "save-settings";
  settings.addEventListener("submit", async (event) => {
    event.preventDefault();
    const saved = await dependencies.saveConfig({
      downloadRoot: readInput(settings, "download-root") || state.settings.downloadRoot,
      concurrency: Number(readInput(settings, "concurrency") || state.settings.concurrency),
      defaultEngine: selectedEngine,
      ytdlpPath: nullablePath(readInput(settings, "ytdlp-path")),
      ffmpegPath: nullablePath(readInput(settings, "ffmpeg-path")),
      ffprobePath: nullablePath(readInput(settings, "ffprobe-path")),
    });
    state.settings = saved;
    renderApp(root, state, dependencies);
  });
  settings.append(rootField, concurrency, engine, ytdlp, ffmpeg, ffprobe, save);
  panel.append(settings);
  return panel;
}

function readInput(root: HTMLElement, testId: string): string {
  return root.querySelector<HTMLInputElement>(`[data-testid='${testId}']`)?.value.trim() ?? "";
}

function nullablePath(value: string): string | null {
  return value ? value : null;
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

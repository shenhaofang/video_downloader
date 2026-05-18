import {
  clearBilibiliLogin as defaultClearBilibiliLogin,
  createTask as defaultCreateTask,
  getToolStatus as defaultGetToolStatus,
  listTaskGroups as defaultListTaskGroups,
  pollBilibiliLogin as defaultPollBilibiliLogin,
  runTask as defaultRunTask,
  saveConfig as defaultSaveConfig,
  startBilibiliLogin as defaultStartBilibiliLogin,
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
  startBilibiliLogin?: typeof defaultStartBilibiliLogin;
  pollBilibiliLogin?: typeof defaultPollBilibiliLogin;
  clearBilibiliLogin?: typeof defaultClearBilibiliLogin;
  getToolStatus?: typeof defaultGetToolStatus;
  listTaskGroups?: typeof defaultListTaskGroups;
  progressPollMs?: number;
  qrPollMs?: number;
}

export function renderApp(
  root: HTMLElement,
  state: AppState,
  dependencies: RenderDependencies = {},
): void {
  const createTask = dependencies.createTask ?? defaultCreateTask;
  const runTask = dependencies.runTask ?? defaultRunTask;
  const saveConfig = dependencies.saveConfig ?? defaultSaveConfig;
  const startBilibiliLogin = dependencies.startBilibiliLogin ?? defaultStartBilibiliLogin;
  const pollBilibiliLogin = dependencies.pollBilibiliLogin ?? defaultPollBilibiliLogin;
  const clearBilibiliLogin = dependencies.clearBilibiliLogin ?? defaultClearBilibiliLogin;
  const getToolStatus = dependencies.getToolStatus ?? defaultGetToolStatus;
  const listTaskGroups = dependencies.listTaskGroups ?? defaultListTaskGroups;
  const progressPollMs = dependencies.progressPollMs ?? 1000;
  const qrPollMs = dependencies.qrPollMs ?? 2000;
  root.replaceChildren(
    buildAppShell(root, state, {
      createTask,
      runTask,
      saveConfig,
      startBilibiliLogin,
      pollBilibiliLogin,
      clearBilibiliLogin,
      getToolStatus,
      listTaskGroups,
      progressPollMs,
      qrPollMs,
    }),
  );
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
  const downloads = buildDownloadsPanel(state, dependencies);
  const login = buildLoginPanel(root, state, dependencies);
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
  dependencies: Required<RenderDependencies>,
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
    const created = await dependencies.createTask({ url, output_dir: outputDir, has_login: false });
    state.taskGroups.unshift(created);
    renderTaskList(taskList, state);
    for (const task of created.tasks) {
      const updated = await runTaskWithProgressPolling(state, taskList, task.id, {
        runTask: dependencies.runTask,
        listTaskGroups: dependencies.listTaskGroups,
        progressPollMs: dependencies.progressPollMs,
      });
      replaceTask(state, updated);
      renderTaskList(taskList, state);
    }
  });

  const taskList = element("div", "task-list");
  renderTaskList(taskList, state);
  panel.append(title, form, taskList);
  return panel;
}

async function runTaskWithProgressPolling(
  state: AppState,
  taskList: HTMLElement,
  taskId: string,
  dependencies: Pick<Required<RenderDependencies>, "runTask" | "listTaskGroups" | "progressPollMs">,
): Promise<DownloadTask> {
  let isPolling = false;
  const intervalId = window.setInterval(async () => {
    if (isPolling) {
      return;
    }
    isPolling = true;
    try {
      const persistedGroups = await dependencies.listTaskGroups();
      mergePersistedTaskGroups(state, persistedGroups);
      renderTaskList(taskList, state);
    } catch (error) {
      console.error("Failed to refresh task progress", error);
    } finally {
      isPolling = false;
    }
  }, dependencies.progressPollMs);

  try {
    return await dependencies.runTask({ task_id: taskId });
  } finally {
    window.clearInterval(intervalId);
  }
}

function mergePersistedTaskGroups(state: AppState, persistedGroups: CreatedTaskGroup[]): void {
  if (persistedGroups.length === 0) {
    return;
  }
  const persistedById = new Map(persistedGroups.map((group) => [group.group.id, group]));
  state.taskGroups = state.taskGroups.map(
    (current) => persistedById.get(current.group.id) ?? current,
  );
}

function renderTaskList(taskList: HTMLElement, state: AppState): void {
  taskList.replaceChildren(...state.taskGroups.map(buildTaskGroupCard));
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

function buildLoginPanel(
  root: HTMLElement,
  state: AppState,
  dependencies: Required<RenderDependencies>,
): HTMLElement {
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

    const detail =
      row.platform === "bilibili"
        ? buildBilibiliLoginDetail(root, state, dependencies)
        : element("div", "platform-detail", "凭据将保存为本地加密文件。");
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

function buildBilibiliLoginDetail(
  root: HTMLElement,
  state: AppState,
  dependencies: Required<RenderDependencies>,
): HTMLElement {
  const detail = element("div", "platform-detail");
  const copy = element("div", "login-copy", "保存方式：本地加密文件");
  const actions = element("div", "login-actions");
  const start = element("button", "secondary", "生成二维码链接");
  start.type = "button";
  start.dataset.testid = "start-bilibili-login";
  const poll = element("button", "secondary", "检查扫码状态");
  poll.type = "button";
  poll.dataset.testid = "poll-bilibili-login";
  poll.disabled = !state.bilibiliLogin.qrcodeKey;
  const clear = element("button", "secondary", "清除登录");
  clear.type = "button";
  clear.dataset.testid = "clear-bilibili-login";
  actions.append(start, poll, clear);

  const qr = state.bilibiliLogin.url ? element("div", "qr-url", state.bilibiliLogin.url) : null;
  const messageText = loginStatusText(state.bilibiliLogin.status, state.bilibiliLogin.message);
  const message = element("div", "login-message", messageText);
  if (state.bilibiliLogin.error) {
    message.append(element("span", "login-error", state.bilibiliLogin.error));
  }

  start.addEventListener("click", async () => {
    try {
      const result = await dependencies.startBilibiliLogin();
      state.bilibiliLogin = {
        qrcodeKey: result.qrcode_key,
        url: result.url,
        status: "pending",
        message: "请用 bilibili 扫码后检查状态",
        error: null,
        pollTimerId: state.bilibiliLogin.pollTimerId,
      };
      startBilibiliAutoPoll(root, state, dependencies);
    } catch (error) {
      state.bilibiliLogin = {
        ...state.bilibiliLogin,
        error: errorMessage(error),
      };
    }
    renderApp(root, state, dependencies);
  });

  poll.addEventListener("click", async () => {
    if (!state.bilibiliLogin.qrcodeKey) {
      return;
    }
    try {
      const result = await dependencies.pollBilibiliLogin({
        qrcode_key: state.bilibiliLogin.qrcodeKey,
      });
      state.bilibiliLogin = {
        ...state.bilibiliLogin,
        status: result.status,
        message: result.message,
        error: null,
      };
      if (result.status === "confirmed") {
        setPlatformStatus(state, "bilibili", "已登录");
      }
      if (isTerminalLoginPollStatus(result.status)) {
        stopBilibiliAutoPoll(state);
      }
    } catch (error) {
      state.bilibiliLogin = {
        ...state.bilibiliLogin,
        error: errorMessage(error),
      };
    }
    renderApp(root, state, dependencies);
  });

  clear.addEventListener("click", async () => {
    try {
      await dependencies.clearBilibiliLogin();
      stopBilibiliAutoPoll(state);
      state.bilibiliLogin = {
        qrcodeKey: null,
        url: null,
        status: null,
        message: null,
        error: null,
        pollTimerId: null,
      };
      setPlatformStatus(state, "bilibili", "未登录");
    } catch (error) {
      state.bilibiliLogin = {
        ...state.bilibiliLogin,
        error: errorMessage(error),
      };
    }
    renderApp(root, state, dependencies);
  });

  detail.append(copy, actions);
  if (qr) {
    detail.append(qr);
  }
  detail.append(message);
  return detail;
}

function setPlatformStatus(state: AppState, platform: string, status: string): void {
  const row = state.platforms.find((item) => item.platform === platform);
  if (row) {
    row.status = status;
  } else {
    state.platforms.push({ platform, status });
  }
}

function startBilibiliAutoPoll(
  root: HTMLElement,
  state: AppState,
  dependencies: Required<RenderDependencies>,
): void {
  stopBilibiliAutoPoll(state);
  const timerId = window.setInterval(async () => {
    if (!state.bilibiliLogin.qrcodeKey) {
      stopBilibiliAutoPoll(state);
      return;
    }
    try {
      const result = await dependencies.pollBilibiliLogin({
        qrcode_key: state.bilibiliLogin.qrcodeKey,
      });
      state.bilibiliLogin = {
        ...state.bilibiliLogin,
        status: result.status,
        message: result.message,
        error: null,
      };
      if (result.status === "confirmed") {
        setPlatformStatus(state, "bilibili", "已登录");
      }
      if (isTerminalLoginPollStatus(result.status)) {
        stopBilibiliAutoPoll(state);
      }
      renderApp(root, state, dependencies);
    } catch (error) {
      state.bilibiliLogin = {
        ...state.bilibiliLogin,
        error: errorMessage(error),
      };
      renderApp(root, state, dependencies);
    }
  }, dependencies.qrPollMs);
  state.bilibiliLogin = {
    ...state.bilibiliLogin,
    pollTimerId: timerId,
  };
}

function stopBilibiliAutoPoll(state: AppState): void {
  if (state.bilibiliLogin.pollTimerId !== null) {
    window.clearInterval(state.bilibiliLogin.pollTimerId);
    state.bilibiliLogin = {
      ...state.bilibiliLogin,
      pollTimerId: null,
    };
  }
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
  const toolStatus = buildToolStatusPanel(state);

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
    state.toolStatus = await dependencies.getToolStatus();
    renderApp(root, state, dependencies);
  });
  settings.append(rootField, concurrency, engine, ytdlp, ffmpeg, ffprobe, toolStatus, save);
  panel.append(settings);
  return panel;
}

function buildToolStatusPanel(state: AppState): HTMLElement {
  const status = element("div", "tool-status");
  status.append(element("span", "tool-status-title", "工具状态"));
  status.append(
    toolStatusItem("yt-dlp", state.toolStatus.ytdlp),
    toolStatusItem("ffmpeg", state.toolStatus.ffmpeg),
    toolStatusItem("ffprobe", state.toolStatus.ffprobe),
  );
  return status;
}

function toolStatusItem(name: string, status: string): HTMLElement {
  const item = element("div", "tool-status-item");
  item.append(element("span", "", name), element("strong", "", toolStatusLabel(status)));
  return item;
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
    merging: "合并中",
    completed: "已完成",
    failed: "失败",
  };
  return labels[state] ?? state;
}

function loginStatusText(status: string | null, message: string | null): string {
  if (status === "confirmed") {
    return "已登录，Cookie 已加密保存";
  }
  if (status === "scanned") {
    return message || "已扫码，请在手机上确认";
  }
  if (status === "expired") {
    return message || "二维码已过期";
  }
  if (status === "pending") {
    return message || "等待扫码";
  }
  return "可生成 bilibili 登录二维码链接";
}

function isTerminalLoginPollStatus(status: string): boolean {
  return status === "confirmed" || status === "expired";
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function toolStatusLabel(status: string): string {
  const labels: Record<string, string> = {
    available: "可用",
    missing: "缺失",
  };
  return labels[status] ?? status;
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

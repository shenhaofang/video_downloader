import {
  clearBilibiliLogin as defaultClearBilibiliLogin,
  createTask as defaultCreateTask,
  checkAppUpdate as defaultCheckAppUpdate,
  getToolStatus as defaultGetToolStatus,
  installAppUpdate as defaultInstallAppUpdate,
  installMediaTools as defaultInstallMediaTools,
  installYtDlp as defaultInstallYtDlp,
  listTaskGroups as defaultListTaskGroups,
  deleteTask as defaultDeleteTask,
  pauseTask as defaultPauseTask,
  pollBilibiliLogin as defaultPollBilibiliLogin,
  probeBilibiliPages as defaultProbeBilibiliPages,
  retryTask as defaultRetryTask,
  runTask as defaultRunTask,
  saveConfig as defaultSaveConfig,
  selectOutputDirectory as defaultSelectOutputDirectory,
  startBilibiliLogin as defaultStartBilibiliLogin,
  startTask as defaultStartTask,
} from "./api";
import {
  platformRowText,
  taskOutputDirectory,
  emptyPagePreviewState,
  type AppSettings,
  type AppState,
  type CreatedTaskGroup,
  type DownloadTask,
  type Engine,
  type ProbePageItem,
  type TabId,
} from "./state";
import { createQrDataUrl as defaultCreateQrDataUrl } from "./qr";

export interface RenderDependencies {
  createTask?: typeof defaultCreateTask;
  runTask?: typeof defaultRunTask;
  startTask?: typeof defaultStartTask;
  retryTask?: typeof defaultRetryTask;
  pauseTask?: typeof defaultPauseTask;
  deleteTask?: typeof defaultDeleteTask;
  selectOutputDirectory?: typeof defaultSelectOutputDirectory;
  saveConfig?: typeof defaultSaveConfig;
  startBilibiliLogin?: typeof defaultStartBilibiliLogin;
  pollBilibiliLogin?: typeof defaultPollBilibiliLogin;
  probeBilibiliPages?: typeof defaultProbeBilibiliPages;
  clearBilibiliLogin?: typeof defaultClearBilibiliLogin;
  installYtDlp?: typeof defaultInstallYtDlp;
  installMediaTools?: typeof defaultInstallMediaTools;
  getToolStatus?: typeof defaultGetToolStatus;
  checkAppUpdate?: typeof defaultCheckAppUpdate;
  installAppUpdate?: typeof defaultInstallAppUpdate;
  listTaskGroups?: typeof defaultListTaskGroups;
  progressPollMs?: number;
  qrPollMs?: number;
  createQrDataUrl?: typeof defaultCreateQrDataUrl;
}

type TaskRunner = typeof defaultRunTask;

export function renderApp(
  root: HTMLElement,
  state: AppState,
  dependencies: RenderDependencies = {},
): void {
  const createTask = dependencies.createTask ?? defaultCreateTask;
  const runTask = dependencies.runTask ?? defaultRunTask;
  const startTask = dependencies.startTask ?? defaultStartTask;
  const retryTask = dependencies.retryTask ?? defaultRetryTask;
  const pauseTask = dependencies.pauseTask ?? defaultPauseTask;
  const deleteTask = dependencies.deleteTask ?? defaultDeleteTask;
  const selectOutputDirectory =
    dependencies.selectOutputDirectory ?? defaultSelectOutputDirectory;
  const saveConfig = dependencies.saveConfig ?? defaultSaveConfig;
  const startBilibiliLogin = dependencies.startBilibiliLogin ?? defaultStartBilibiliLogin;
  const pollBilibiliLogin = dependencies.pollBilibiliLogin ?? defaultPollBilibiliLogin;
  const probeBilibiliPages = dependencies.probeBilibiliPages ?? defaultProbeBilibiliPages;
  const clearBilibiliLogin = dependencies.clearBilibiliLogin ?? defaultClearBilibiliLogin;
  const installYtDlp = dependencies.installYtDlp ?? defaultInstallYtDlp;
  const installMediaTools = dependencies.installMediaTools ?? defaultInstallMediaTools;
  const getToolStatus = dependencies.getToolStatus ?? defaultGetToolStatus;
  const checkAppUpdate = dependencies.checkAppUpdate ?? defaultCheckAppUpdate;
  const installAppUpdate = dependencies.installAppUpdate ?? defaultInstallAppUpdate;
  const listTaskGroups = dependencies.listTaskGroups ?? defaultListTaskGroups;
  const progressPollMs = dependencies.progressPollMs ?? 1000;
  const qrPollMs = dependencies.qrPollMs ?? 2000;
  const createQrDataUrl = dependencies.createQrDataUrl ?? defaultCreateQrDataUrl;
  root.replaceChildren(
    buildAppShell(root, state, {
      createTask,
      runTask,
      startTask,
      retryTask,
      pauseTask,
      deleteTask,
      selectOutputDirectory,
      saveConfig,
      startBilibiliLogin,
      pollBilibiliLogin,
      probeBilibiliPages,
      clearBilibiliLogin,
      installYtDlp,
      installMediaTools,
      getToolStatus,
      checkAppUpdate,
      installAppUpdate,
      listTaskGroups,
      progressPollMs,
      qrPollMs,
      createQrDataUrl,
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
  form.append(field("视频链接", "video-url", "请输入 bilibili 视频或合集链接"));
  form.append(outputDirectoryField(state, dependencies));

  const actions = element("div", "download-actions");
  const probeButton = element("button", "secondary", "探测选集");
  probeButton.type = "button";
  probeButton.dataset.testid = "probe-pages";

  const addButton = element("button", "primary", "添加下载");
  addButton.type = "submit";
  addButton.dataset.testid = "add-task";
  actions.append(probeButton, addButton);
  form.append(actions);

  const pagePreview = element("div", "page-preview-host");
  renderPagePreview(pagePreview, state);
  const taskList = element("div", "task-list");
  renderTaskList(taskList, state, dependencies);

  probeButton.addEventListener("click", async () => {
    const url = readInput(form, "video-url");
    if (!url) {
      state.pagePreview = {
        ...emptyPagePreviewState(),
        error: "请输入视频链接",
      };
      renderPagePreview(pagePreview, state);
      return;
    }

    state.pagePreview = {
      ...state.pagePreview,
      url,
      isLoading: true,
      error: null,
    };
    renderPagePreview(pagePreview, state);

    try {
      const result = await dependencies.probeBilibiliPages({ url, has_login: false });
      state.pagePreview = {
        url,
        groupTitle: result.groupTitle,
        items: result.items,
        selectedPages: new Set(result.items.map((item) => item.page)),
        isLoading: false,
        error: null,
      };
    } catch (error) {
      state.pagePreview = {
        ...state.pagePreview,
        isLoading: false,
        error: errorMessage(error),
      };
    }
    renderPagePreview(pagePreview, state);
  });

  form.addEventListener("submit", async (event) => {
    event.preventDefault();
    const url = form.querySelector<HTMLInputElement>("[data-testid='video-url']")?.value.trim() ?? "";
    const outputDir =
      form.querySelector<HTMLInputElement>("[data-testid='output-directory']")?.value ??
      taskOutputDirectory(state.settings.downloadRoot);
    const selectedPages = selectedPagesForUrl(state, url);
    if (selectedPages?.length === 0) {
      state.pagePreview = {
        ...state.pagePreview,
        error: "至少选择一个分 P",
      };
      renderPagePreview(pagePreview, state);
      return;
    }

    const createInput = {
      url,
      output_dir: outputDir,
      has_login: false,
      ...(selectedPages ? { selected_pages: selectedPages } : {}),
    };
    const created = await dependencies.createTask(createInput);
    state.taskGroups.unshift(created);
    if (state.pagePreview.url === url) {
      state.pagePreview = emptyPagePreviewState();
      renderPagePreview(pagePreview, state);
    }
    renderTaskList(taskList, state, dependencies);
    await runTasksWithConcurrency(state, taskList, created.tasks, dependencies);
  });

  panel.append(title, form, pagePreview, taskList);
  return panel;
}

async function runTasksWithConcurrency(
  state: AppState,
  taskList: HTMLElement,
  tasks: DownloadTask[],
  dependencies: Required<RenderDependencies>,
  taskRunnerForTask: (task: DownloadTask) => TaskRunner = () => dependencies.runTask,
): Promise<void> {
  let nextTaskIndex = 0;
  const workerCount = Math.min(tasks.length, Math.max(1, state.settings.concurrency));
  const workers = Array.from({ length: workerCount }, async () => {
    while (nextTaskIndex < tasks.length) {
      if (hasAutoRunPauseSignal(state, tasks)) {
        break;
      }
      const task = tasks[nextTaskIndex];
      nextTaskIndex += 1;
      try {
        const updated = await runTaskWithProgressPolling(state, taskList, task.id, {
          dependencies,
          taskRunner: taskRunnerForTask(task),
        });
        replaceTask(state, updated);
      } catch (error) {
        console.error("Failed to run task", error);
        await refreshPersistedTaskGroups(state, taskList, dependencies);
      }
      renderTaskList(taskList, state, dependencies);
    }
  });

  await Promise.all(workers);
}

function hasAutoRunPauseSignal(state: AppState, tasks: DownloadTask[]): boolean {
  return tasks.some((task) => state.autoRunPausedTaskIds.has(task.id));
}

async function runTaskWithProgressPolling(
  state: AppState,
  taskList: HTMLElement,
  taskId: string,
  options: {
    dependencies: Required<RenderDependencies>;
    taskRunner: TaskRunner;
  },
): Promise<DownloadTask> {
  const { dependencies, taskRunner } = options;
  let isPolling = false;
  const intervalId = window.setInterval(async () => {
    if (isPolling) {
      return;
    }
    isPolling = true;
    try {
      await refreshPersistedTaskGroups(state, taskList, dependencies);
    } catch (error) {
      console.error("Failed to refresh task progress", error);
    } finally {
      isPolling = false;
    }
  }, dependencies.progressPollMs);

  try {
    return await taskRunner({ task_id: taskId });
  } finally {
    window.clearInterval(intervalId);
  }
}

async function refreshPersistedTaskGroups(
  state: AppState,
  taskList: HTMLElement,
  dependencies: Required<RenderDependencies>,
): Promise<void> {
  const persistedGroups = await dependencies.listTaskGroups();
  mergePersistedTaskGroups(state, persistedGroups);
  renderTaskList(taskList, state, dependencies);
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

function renderTaskList(
  taskList: HTMLElement,
  state: AppState,
  dependencies: Required<RenderDependencies>,
): void {
  taskList.replaceChildren(
    ...state.taskGroups.map((group) => buildTaskGroupCard(group, state, taskList, dependencies)),
  );
}

function replaceTask(state: AppState, updated: DownloadTask): void {
  for (const group of state.taskGroups) {
    const index = group.tasks.findIndex((task) => task.id === updated.id);
    if (index >= 0) {
      group.tasks[index] = updated;
      group.group.state = taskGroupState(group);
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

function outputDirectoryField(
  state: AppState,
  dependencies: Required<RenderDependencies>,
): HTMLElement {
  const label = element("label", "field output-field");
  label.append(element("span", "", "输出目录"));
  const input = document.createElement("input");
  input.dataset.testid = "output-directory";
  input.value = taskOutputDirectory(state.settings.downloadRoot);
  const button = element("button", "secondary", "选择");
  button.type = "button";
  button.dataset.testid = "select-output-directory";
  button.addEventListener("click", async () => {
    const currentPath = input.value.trim() || taskOutputDirectory(state.settings.downloadRoot);
    const selectedPath = await dependencies.selectOutputDirectory(currentPath);
    if (selectedPath) {
      input.value = selectedPath;
    }
  });
  label.append(input, button);
  return label;
}

function selectedPagesForUrl(state: AppState, url: string): number[] | null {
  const preview = state.pagePreview;
  if (preview.url !== url || preview.items.length === 0) {
    return null;
  }

  return preview.items
    .filter((item) => preview.selectedPages.has(item.page))
    .map((item) => item.page);
}

function renderPagePreview(container: HTMLElement, state: AppState): void {
  const preview = state.pagePreview;
  if (!preview.isLoading && !preview.error && preview.items.length === 0) {
    container.replaceChildren();
    return;
  }

  const panel = element("section", "page-preview");
  const header = element("div", "page-preview-header");
  const title = element("strong", "", preview.groupTitle ?? "视频选集");
  const count = element(
    "span",
    "",
    preview.isLoading ? "探测中" : `已选 ${preview.selectedPages.size}/${preview.items.length}`,
  );
  count.dataset.testid = "page-selection-count";
  header.append(title, count);
  panel.append(header);

  if (preview.error) {
    panel.append(element("div", "page-preview-error", preview.error));
  }

  if (preview.isLoading) {
    panel.append(element("div", "page-preview-empty", "正在探测选集..."));
    container.replaceChildren(panel);
    return;
  }

  if (preview.items.length > 0) {
    panel.append(buildPagePreviewControls(container, state));
    const list = element("div", "page-preview-list");
    list.append(
      ...preview.items.map((item) => buildPagePreviewRow(container, state, item)),
    );
    panel.append(list);
  }

  container.replaceChildren(panel);
}

function buildPagePreviewControls(container: HTMLElement, state: AppState): HTMLElement {
  const controls = element("div", "page-preview-controls");
  const selectAll = element("button", "secondary compact", "全选");
  selectAll.type = "button";
  selectAll.dataset.testid = "select-all-pages";
  selectAll.addEventListener("click", () => {
    state.pagePreview = {
      ...state.pagePreview,
      selectedPages: new Set(state.pagePreview.items.map((item) => item.page)),
      error: null,
    };
    renderPagePreview(container, state);
  });

  const clear = element("button", "secondary compact", "清空");
  clear.type = "button";
  clear.dataset.testid = "clear-page-selection";
  clear.addEventListener("click", () => {
    state.pagePreview = {
      ...state.pagePreview,
      selectedPages: new Set<number>(),
      error: null,
    };
    renderPagePreview(container, state);
  });

  const range = element("div", "page-range");
  const start = document.createElement("input");
  start.type = "number";
  start.dataset.testid = "page-range-start";
  start.value = String(firstSelectedOrFirstPage(state.pagePreview.items, state.pagePreview.selectedPages));
  const end = document.createElement("input");
  end.type = "number";
  end.dataset.testid = "page-range-end";
  end.value = String(lastSelectedOrLastPage(state.pagePreview.items, state.pagePreview.selectedPages));
  const apply = element("button", "secondary compact", "应用范围");
  apply.type = "button";
  apply.dataset.testid = "apply-page-range";
  apply.addEventListener("click", () => {
    const startPage = Number(start.value);
    const endPage = Number(end.value);
    if (!Number.isFinite(startPage) || !Number.isFinite(endPage)) {
      return;
    }
    const low = Math.min(startPage, endPage);
    const high = Math.max(startPage, endPage);
    const next = state.pagePreview.items
      .filter((item) => item.page >= low && item.page <= high)
      .map((item) => item.page);
    state.pagePreview = {
      ...state.pagePreview,
      selectedPages: new Set(next),
      error: next.length > 0 ? null : "范围内没有分 P",
    };
    renderPagePreview(container, state);
  });
  range.append(start, element("span", "", "-"), end, apply);

  controls.append(selectAll, clear, range);
  return controls;
}

function buildPagePreviewRow(
  container: HTMLElement,
  state: AppState,
  item: ProbePageItem,
): HTMLElement {
  const label = element("label", "page-preview-row");
  const checkbox = document.createElement("input");
  checkbox.type = "checkbox";
  checkbox.checked = state.pagePreview.selectedPages.has(item.page);
  checkbox.dataset.testid = `page-checkbox-${item.page}`;
  checkbox.addEventListener("change", () => {
    const selectedPages = new Set(state.pagePreview.selectedPages);
    if (checkbox.checked) {
      selectedPages.add(item.page);
    } else {
      selectedPages.delete(item.page);
    }
    state.pagePreview = {
      ...state.pagePreview,
      selectedPages,
      error: null,
    };
    updatePagePreviewSelectionSummary(container, state);
  });
  label.append(
    checkbox,
    element("span", "page-number", String(item.page)),
    element("span", "page-title", item.title),
    element("span", "page-quality", item.quality ?? "自动"),
  );
  return label;
}

function updatePagePreviewSelectionSummary(container: HTMLElement, state: AppState): void {
  const preview = state.pagePreview;
  const count = container.querySelector<HTMLElement>("[data-testid='page-selection-count']");
  if (count) {
    count.textContent = `已选 ${preview.selectedPages.size}/${preview.items.length}`;
  }
  if (!preview.error) {
    container.querySelector(".page-preview-error")?.remove();
  }
}

function firstSelectedOrFirstPage(items: ProbePageItem[], selectedPages: Set<number>): number {
  return items.find((item) => selectedPages.has(item.page))?.page ?? items[0]?.page ?? 1;
}

function lastSelectedOrLastPage(items: ProbePageItem[], selectedPages: Set<number>): number {
  const selectedItems = items.filter((item) => selectedPages.has(item.page));
  return selectedItems[selectedItems.length - 1]?.page ?? items[items.length - 1]?.page ?? 1;
}

function buildTaskGroupCard(
  created: CreatedTaskGroup,
  state: AppState,
  taskList: HTMLElement,
  dependencies: Required<RenderDependencies>,
): HTMLElement {
  const card = element("article", "task-card");
  const details = document.createElement("details");
  details.open = true;
  const summary = document.createElement("summary");
  summary.className = "task-group-summary";
  summary.append(
    element("strong", "", created.group.title),
    element("span", "", created.group.output_dir),
    stateElement("span", "state-pill", taskGroupState(created)),
  );
  summary.append(buildTaskGroupActions(created, state, taskList, dependencies));
  const children = element("div", "child-task-list");
  children.append(
    ...created.tasks.map((task) => buildChildTask(task, state, taskList, dependencies)),
  );
  details.append(summary, children);
  card.append(details);
  return card;
}

function buildTaskGroupActions(
  created: CreatedTaskGroup,
  state: AppState,
  taskList: HTMLElement,
  dependencies: Required<RenderDependencies>,
): HTMLElement {
  const actions = element("div", "task-group-actions");
  const continuable = continuableTasks(created.tasks);
  if (continuable.length > 0) {
    const button = element("button", "secondary compact", "继续") as HTMLButtonElement;
    button.type = "button";
    button.dataset.testid = `continue-group-${created.group.id}`;
    button.addEventListener("click", async (event) => {
      event.preventDefault();
      event.stopPropagation();
      button.disabled = true;
      continuable.forEach((task) => state.autoRunPausedTaskIds.delete(task.id));
      try {
        await runTasksWithConcurrency(state, taskList, continuable, dependencies, (task) =>
          task.state === "failed" ? dependencies.retryTask : dependencies.startTask,
        );
      } catch (error) {
        console.error("Failed to continue task group", error);
      } finally {
        button.disabled = false;
      }
    });
    actions.append(button);
  }

  const pausable = pausableTasks(created.tasks);
  if (pausable.length > 0) {
    const pause = element("button", "secondary compact", "暂停") as HTMLButtonElement;
    pause.type = "button";
    pause.dataset.testid = `pause-group-${created.group.id}`;
    pause.addEventListener("click", async (event) => {
      event.preventDefault();
      event.stopPropagation();
      pause.disabled = true;
      const pausedIds = pausable.map((task) => task.id);
      pausedIds.forEach((taskId) => state.autoRunPausedTaskIds.add(taskId));
      try {
        for (const task of pausable) {
          const updated = await dependencies.pauseTask({ task_id: task.id });
          replaceTask(state, updated);
        }
        renderTaskList(taskList, state, dependencies);
      } catch (error) {
        console.error("Failed to pause task group", error);
        pausedIds.forEach((taskId) => state.autoRunPausedTaskIds.delete(taskId));
        pause.disabled = false;
      }
    });
    actions.append(pause);
  }

  const remove = element("button", "secondary compact", "删除") as HTMLButtonElement;
  remove.type = "button";
  remove.dataset.testid = `delete-group-${created.group.id}`;
  remove.addEventListener("click", async (event) => {
    event.preventDefault();
    event.stopPropagation();
    remove.disabled = true;
    try {
      let groups = state.taskGroups;
      for (const task of created.tasks) {
        groups = await dependencies.deleteTask({ task_id: task.id });
      }
      state.taskGroups = groups;
      renderTaskList(taskList, state, dependencies);
    } catch (error) {
      console.error("Failed to delete task group", error);
      remove.disabled = false;
    }
  });
  actions.append(remove);
  return actions;
}

function continuableTasks(tasks: DownloadTask[]): DownloadTask[] {
  return tasks.filter((task) =>
    ["queued", "paused", "interrupted", "failed"].includes(task.state),
  );
}

function pausableTasks(tasks: DownloadTask[]): DownloadTask[] {
  return tasks.filter((task) =>
    ["pending", "probing", "queued", "downloading", "merging", "interrupted"].includes(task.state),
  );
}

function buildChildTask(
  task: DownloadTask,
  state: AppState,
  taskList: HTMLElement,
  dependencies: Required<RenderDependencies>,
): HTMLElement {
  const row = element("div", "child-task");
  row.append(
    element("div", "child-title", task.title),
    element("div", "child-file", fileName(task.output_file)),
    stateElement("div", "child-state", task.state),
    element("div", "child-progress", progressText(task)),
    element("div", "child-retry", `重试 ${task.retry_count}/${task.max_retries}`),
    element("div", "child-meta", `${task.quality ?? "自动"} · ${task.engine}`),
    buildChildActions(task, state, taskList, dependencies),
  );
  if (task.error_message) {
    row.append(element("div", "child-error", `失败原因：${task.error_message}`));
  }
  return row;
}

function buildChildActions(
  task: DownloadTask,
  state: AppState,
  taskList: HTMLElement,
  dependencies: Required<RenderDependencies>,
): HTMLElement {
  const actions = element("div", "child-actions");
  const buttons: HTMLButtonElement[] = [];
  if (["queued", "paused", "interrupted"].includes(task.state)) {
    buttons.push(
      childActionButton("开始", `start-task-${task.id}`, async () => {
        state.autoRunPausedTaskIds.delete(task.id);
        await runChildTask(task.id, state, taskList, dependencies, dependencies.startTask);
      }),
    );
  }

  if (
    [
      "pending",
      "probing",
      "queued",
      "downloading",
      "merging",
      "interrupted",
    ].includes(task.state)
  ) {
    buttons.push(
      childActionButton("暂停", `pause-task-${task.id}`, async () => {
        state.autoRunPausedTaskIds.add(task.id);
        try {
          const updated = await dependencies.pauseTask({ task_id: task.id });
          replaceTask(state, updated);
          renderTaskList(taskList, state, dependencies);
        } catch (error) {
          state.autoRunPausedTaskIds.delete(task.id);
          throw error;
        }
      }),
    );
  }

  if (task.state === "failed") {
    buttons.push(
      childActionButton("重试", `retry-task-${task.id}`, async () => {
        state.autoRunPausedTaskIds.delete(task.id);
        await runChildTask(task.id, state, taskList, dependencies, dependencies.retryTask);
      }),
    );
  }

  buttons.push(
    childActionButton("删除", `delete-task-${task.id}`, async () => {
      state.autoRunPausedTaskIds.delete(task.id);
      state.taskGroups = await dependencies.deleteTask({ task_id: task.id });
      renderTaskList(taskList, state, dependencies);
    }),
  );

  actions.append(...buttons);
  return actions;
}

function childActionButton(
  label: string,
  testId: string,
  onClick: () => Promise<void>,
): HTMLButtonElement {
  const button = element("button", "secondary compact", label) as HTMLButtonElement;
  button.type = "button";
  button.dataset.testid = testId;
  button.addEventListener("click", async () => {
    button.disabled = true;
    try {
      await onClick();
    } catch (error) {
      console.error(`Failed to ${label} task`, error);
      button.disabled = false;
    }
  });
  return button;
}

async function runChildTask(
  taskId: string,
  state: AppState,
  taskList: HTMLElement,
  dependencies: Required<RenderDependencies>,
  taskRunner: TaskRunner,
): Promise<void> {
  try {
    const updated = await runTaskWithProgressPolling(state, taskList, taskId, {
      dependencies,
      taskRunner,
    });
    replaceTask(state, updated);
    renderTaskList(taskList, state, dependencies);
  } catch (error) {
    console.error("Failed to run child task", error);
    await refreshPersistedTaskGroups(state, taskList, dependencies);
  }
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
  const qrImage = state.bilibiliLogin.qrImageDataUrl
    ? buildQrImage(state.bilibiliLogin.qrImageDataUrl)
    : null;
  const messageText = loginStatusText(state.bilibiliLogin.status, state.bilibiliLogin.message);
  const message = element("div", "login-message", messageText);
  if (state.bilibiliLogin.error) {
    message.append(element("span", "login-error", state.bilibiliLogin.error));
  }

  start.addEventListener("click", async () => {
    try {
      const result = await dependencies.startBilibiliLogin();
      const qrImageDataUrl = await dependencies.createQrDataUrl(result.url);
      state.bilibiliLogin = {
        qrcodeKey: result.qrcode_key,
        url: result.url,
        qrImageDataUrl,
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
        qrImageDataUrl: null,
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
  if (qrImage) {
    detail.append(qrImage);
  }
  if (qr) {
    detail.append(qr);
  }
  detail.append(message);
  return detail;
}

function buildQrImage(src: string): HTMLImageElement {
  const image = document.createElement("img");
  image.className = "qr-image";
  image.dataset.testid = "bilibili-qr-image";
  image.alt = "bilibili 登录二维码";
  image.src = src;
  return image;
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

  const ytdlp = field("yt-dlp 路径", "ytdlp-path", "dependencies\\yt-dlp\\yt-dlp.exe");
  ytdlp.querySelector("input")!.value = state.settings.ytdlpPath ?? "";
  const installYtdlp = element("button", "secondary", "下载 yt-dlp");
  installYtdlp.type = "button";
  installYtdlp.dataset.testid = "install-ytdlp";
  installYtdlp.addEventListener("click", async () => {
    const idleLabel = "下载 yt-dlp";
    installYtdlp.setAttribute("aria-busy", "true");
    installYtdlp.textContent = "下载中";
    try {
      state.settings = await dependencies.installYtDlp();
      state.toolStatus = await dependencies.getToolStatus();
      renderApp(root, state, dependencies);
    } finally {
      installYtdlp.removeAttribute("aria-busy");
      installYtdlp.textContent = idleLabel;
    }
  });
  const installFfmpeg = element("button", "secondary", "下载 FFmpeg");
  installFfmpeg.type = "button";
  installFfmpeg.dataset.testid = "install-media-tools";
  installFfmpeg.addEventListener("click", async () => {
    const idleLabel = "下载 FFmpeg";
    installFfmpeg.setAttribute("aria-busy", "true");
    installFfmpeg.textContent = "下载中";
    try {
      state.settings = await dependencies.installMediaTools();
      state.toolStatus = await dependencies.getToolStatus();
      renderApp(root, state, dependencies);
    } finally {
      installFfmpeg.removeAttribute("aria-busy");
      installFfmpeg.textContent = idleLabel;
    }
  });
  const ffmpeg = field("ffmpeg 路径", "ffmpeg-path", "dependencies\\ffmpeg\\bin\\ffmpeg.exe");
  ffmpeg.querySelector("input")!.value = state.settings.ffmpegPath ?? "";
  const ffprobe = field("ffprobe 路径", "ffprobe-path", "dependencies\\ffmpeg\\bin\\ffprobe.exe");
  ffprobe.querySelector("input")!.value = state.settings.ffprobePath ?? "";
  const toolStatus = buildToolStatusPanel(state);
  const updatePanel = buildUpdatePanel(root, state, dependencies);

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
  settings.append(
    rootField,
    concurrency,
    engine,
    ytdlp,
    installYtdlp,
    ffmpeg,
    ffprobe,
    installFfmpeg,
    toolStatus,
    updatePanel,
    save,
  );
  panel.append(settings);
  return panel;
}

function buildUpdatePanel(
  root: HTMLElement,
  state: AppState,
  dependencies: Required<RenderDependencies>,
): HTMLElement {
  const panel = element("div", "update-panel");
  panel.dataset.testid = "app-update";
  panel.append(element("span", "tool-status-title", "应用更新"));
  panel.append(element("div", "update-version", `当前版本 ${state.update.currentVersion}`));

  const message = updateMessage(state);
  if (message) {
    panel.append(element("div", state.update.phase === "error" ? "update-error" : "update-message", message));
  }
  if (state.update.notes) {
    panel.append(element("div", "update-notes", state.update.notes));
  }

  const actions = element("div", "update-actions");
  const check = element(
    "button",
    "secondary",
    state.update.phase === "checking" ? "检查中" : "检查更新",
  );
  check.type = "button";
  check.dataset.testid = "check-update";
  check.disabled = state.update.phase === "checking" || state.update.phase === "installing";
  check.addEventListener("click", async () => {
    state.update = {
      ...state.update,
      phase: "checking",
      error: null,
    };
    renderApp(root, state, dependencies);
    try {
      const status = await dependencies.checkAppUpdate();
      state.update = {
        phase: status.available ? "available" : "current",
        currentVersion: status.currentVersion,
        latestVersion: status.latestVersion,
        notes: status.notes,
        error: null,
      };
    } catch (error) {
      state.update = {
        ...state.update,
        phase: "error",
        error: errorMessage(error),
      };
    }
    renderApp(root, state, dependencies);
  });
  actions.append(check);

  if (state.update.phase === "available" || state.update.phase === "installing") {
    const blocked = hasUnfinishedRuntimeTasks(state);
    const install = element(
      "button",
      "primary",
      state.update.phase === "installing" ? "正在更新" : "立即更新",
    );
    install.type = "button";
    install.dataset.testid = "install-update";
    install.disabled = blocked || state.update.phase === "installing";
    install.addEventListener("click", async () => {
      state.update = {
        ...state.update,
        phase: "installing",
        error: null,
      };
      renderApp(root, state, dependencies);
      try {
        await dependencies.installAppUpdate();
      } catch (error) {
        state.update = {
          ...state.update,
          phase: "error",
          error: errorMessage(error),
        };
        renderApp(root, state, dependencies);
      }
    });
    actions.append(install);
    if (blocked) {
      panel.append(element("div", "update-message", "请先暂停或完成下载任务再更新"));
    }
  }

  panel.append(actions);
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

function updateMessage(state: AppState): string | null {
  if (state.update.phase === "current") {
    return "已是最新版本";
  }
  if (state.update.phase === "available") {
    return `发现 ${state.update.latestVersion ?? "新版本"}`;
  }
  if (state.update.phase === "installing") {
    return "正在下载并安装，应用将自动重启";
  }
  if (state.update.phase === "error") {
    return state.update.error ?? "更新失败";
  }
  return null;
}

function hasUnfinishedRuntimeTasks(state: AppState): boolean {
  return state.taskGroups.some((group) =>
    group.tasks.some((task) =>
      ["pending", "probing", "queued", "downloading", "merging"].includes(task.state),
    ),
  );
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
    pending: "等待中",
    probing: "探测中",
    queued: "排队中",
    downloading: "下载中",
    merging: "合并中",
    completed: "已完成",
    failed: "失败",
    paused: "已暂停",
    interrupted: "已中断",
    cancelled: "已取消",
  };
  return labels[state] ?? state;
}

function stateElement(
  tag: keyof HTMLElementTagNameMap,
  className: string,
  state: string,
): HTMLElement {
  return element(tag, `${className} state-${state}`, stateLabel(state));
}

function taskGroupState(created: CreatedTaskGroup): DownloadTask["state"] {
  const states = created.tasks.map((task) => task.state);
  if (states.length === 0) {
    return created.group.state;
  }
  if (states.includes("merging")) {
    return "merging";
  }
  if (states.includes("downloading")) {
    return "downloading";
  }
  if (states.includes("probing")) {
    return "probing";
  }
  if (states.every((state) => state === "completed")) {
    return "completed";
  }
  if (states.includes("failed")) {
    return "failed";
  }
  if (states.includes("interrupted")) {
    return "interrupted";
  }
  if (states.includes("paused")) {
    return "paused";
  }
  if (states.includes("cancelled")) {
    return "cancelled";
  }
  if (states.includes("pending")) {
    return "pending";
  }
  return "queued";
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

export type TabId = "downloads" | "login" | "settings";
export type Engine = "native" | "yt-dlp";
export type TaskState =
  | "pending"
  | "probing"
  | "queued"
  | "downloading"
  | "merging"
  | "completed"
  | "failed"
  | "paused"
  | "interrupted"
  | "cancelled";

export interface AppSettings {
  downloadRoot: string;
  concurrency: number;
  defaultEngine: Engine;
  ytdlpPath: string | null;
  ffmpegPath: string | null;
  ffprobePath: string | null;
}

export interface PlatformLoginRow {
  platform: string;
  status: string;
}

export interface TaskGroup {
  id: string;
  title: string;
  output_dir: string;
  state: TaskState;
}

export interface DownloadTask {
  id: string;
  title: string;
  output_file: string;
  state: TaskState;
  bytes_downloaded: number;
  bytes_total: number | null;
  retry_count: number;
  max_retries: number;
  error_code?: string | null;
  error_message?: string | null;
  quality: string | null;
  used_login: boolean;
  engine: Engine;
}

export interface CreatedTaskGroup {
  group: TaskGroup;
  tasks: DownloadTask[];
}

export interface ProbePageItem {
  page: number;
  title: string;
  quality: string | null;
  requiresLogin: boolean;
}

export interface PagePreviewState {
  url: string | null;
  groupTitle: string | null;
  items: ProbePageItem[];
  selectedPages: Set<number>;
  isLoading: boolean;
  error: string | null;
}

export interface BilibiliLoginState {
  qrcodeKey: string | null;
  url: string | null;
  qrImageDataUrl: string | null;
  status: string | null;
  message: string | null;
  error: string | null;
  pollTimerId: number | null;
}

export interface ToolStatus {
  ytdlp: string;
  ffmpeg: string;
  ffprobe: string;
}

export interface AppState {
  activeTab: TabId;
  settings: AppSettings;
  platforms: PlatformLoginRow[];
  expandedPlatforms: Set<string>;
  taskGroups: CreatedTaskGroup[];
  pagePreview: PagePreviewState;
  bilibiliLogin: BilibiliLoginState;
  toolStatus: ToolStatus;
}

export function createInitialState(): AppState {
  return {
    activeTab: "downloads",
    settings: {
      downloadRoot: "D:\\Videos",
      concurrency: 2,
      defaultEngine: "native",
      ytdlpPath: null,
      ffmpegPath: null,
      ffprobePath: null,
    },
    platforms: [{ platform: "bilibili", status: "未登录" }],
    expandedPlatforms: new Set<string>(),
    taskGroups: [],
    pagePreview: emptyPagePreviewState(),
    bilibiliLogin: {
      qrcodeKey: null,
      url: null,
      qrImageDataUrl: null,
      status: null,
      message: null,
      error: null,
      pollTimerId: null,
    },
    toolStatus: {
      ytdlp: "missing",
      ffmpeg: "missing",
      ffprobe: "missing",
    },
  };
}

export function emptyPagePreviewState(): PagePreviewState {
  return {
    url: null,
    groupTitle: null,
    items: [],
    selectedPages: new Set<number>(),
    isLoading: false,
    error: null,
  };
}

export function taskOutputDirectory(downloadRoot: string): string {
  return `${downloadRoot.replace(/[\\/]+$/, "")}\\bilibili`;
}

export function platformRowText(row: PlatformLoginRow): [string, string] {
  return [row.platform, row.status];
}

export function normalizeEngine(value: string): Engine {
  return value === "yt_dlp" || value === "yt-dlp" ? "yt-dlp" : "native";
}

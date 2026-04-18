import { convertFileSrc, invoke, isTauri } from "@tauri-apps/api/core";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { readBubbleSkin, SKIN_SRC } from "./bubbleSkins";

const DISPLAY_NAME = "zyx";

type NavId = "home" | "agent" | "rewind" | "tasks";
type CenterFilter = "events" | 15 | 30 | 60;

type DesktopEventJson =
  | {
      type: "app_switch";
      app: string;
      title?: string | null;
      /** 前台窗口客户区截图，旧数据可能无 */
      image_rel?: string | null;
      width_px?: number | null;
      height_px?: number | null;
      at: string;
    }
  | {
      type: "clipboard";
      text_preview: string;
      char_len: number;
      truncated: boolean;
      at: string;
    }
  | {
      type: "learning_snapshot";
      image_rel: string;
      app: string;
      title?: string | null;
      width_px: number;
      height_px: number;
      at: string;
    }
  | {
      type: "browser_moment";
      trigger: string;
      browser_app: string;
      window_title: string;
      page_title: string;
      /** 从页签标题解析的学习主题（旧数据可能无） */
      learning_content?: string;
      summary: string;
      keywords: string[];
      image_rel: string;
      width_px: number;
      height_px: number;
      at: string;
    };

interface WindowStats {
  window_minutes: number;
  app_switch_count: number;
  clipboard_event_count: number;
  /** 新版后端统计字段；缺省时按 0 展示 */
  learning_snapshot_count?: number;
  browser_moment_count?: number;
  dominant_apps: { app: string; count: number }[];
}

function greetingForNow(): string {
  const h = new Date().getHours();
  if (h < 12) return "早上好";
  if (h < 18) return "下午好";
  return "晚上好";
}

function formatZhDate(d: Date): string {
  return `${d.getMonth() + 1}月${d.getDate()}日`;
}

/** 与后端 `is_bilibili_desktop_exe` 一致：桌面客户端进程名常含 bilibili 或「哔哩」 */
function isBilibiliDesktopExe(app: string): boolean {
  return app.toLowerCase().includes("bilibili") || app.includes("哔哩");
}

const ZH_WEEKDAYS_SHORT = ["日", "一", "二", "三", "四", "五", "六"];
const ZH_MONTHS = [
  "一月",
  "二月",
  "三月",
  "四月",
  "五月",
  "六月",
  "七月",
  "八月",
  "九月",
  "十月",
  "十一月",
  "十二月",
];

function formatRewindDatePill(d: Date): string {
  const w = ZH_WEEKDAYS_SHORT[d.getDay()];
  return `${w}, ${d.getMonth() + 1}月 ${d.getDate()}`;
}

/** 日历从周日起，与界面「日一二三四五六」一致 */
function buildMonthCalendarCells(year: number, month: number): { date: Date; inMonth: boolean }[] {
  const first = new Date(year, month, 1);
  const pad = first.getDay();
  const dim = new Date(year, month + 1, 0).getDate();
  const prevDim = new Date(year, month, 0).getDate();
  const cells: { date: Date; inMonth: boolean }[] = [];
  for (let i = 0; i < pad; i++) {
    const day = prevDim - pad + i + 1;
    cells.push({ date: new Date(year, month - 1, day), inMonth: false });
  }
  for (let d = 1; d <= dim; d++) {
    cells.push({ date: new Date(year, month, d), inMonth: true });
  }
  let n = 1;
  while (cells.length % 7 !== 0 || cells.length < 42) {
    cells.push({ date: new Date(year, month + 1, n), inMonth: false });
    n++;
  }
  return cells;
}

function sameDay(a: Date, b: Date): boolean {
  return (
    a.getFullYear() === b.getFullYear() &&
    a.getMonth() === b.getMonth() &&
    a.getDate() === b.getDate()
  );
}

function startOfWeekSunday(d: Date): Date {
  const x = new Date(d);
  const day = x.getDay();
  x.setDate(x.getDate() - day);
  x.setHours(0, 0, 0, 0);
  return x;
}

function parseTimelineEvents(raw: string): DesktopEventJson[] | null {
  if (!raw || raw.startsWith("加载") || raw.startsWith("当前不是")) return null;
  try {
    const v = JSON.parse(raw) as unknown;
    if (!Array.isArray(v)) return null;
    return v as DesktopEventJson[];
  } catch {
    return null;
  }
}

function parseWindowStats(raw: string): WindowStats | null {
  if (!raw || raw === "—") return null;
  try {
    return JSON.parse(raw) as WindowStats;
  } catch {
    return null;
  }
}

/** 从某日时间线生成「学习卡片」：仅 Chrome，去掉空白新标签；同日相同主题去重保留最近一条 */
function learningCardsFromDayEvents(
  events: DesktopEventJson[],
): Extract<DesktopEventJson, { type: "browser_moment" }>[] {
  const seen = new Map<string, Extract<DesktopEventJson, { type: "browser_moment" }>>();
  for (const e of events) {
    if (e.type !== "browser_moment") continue;
    const app = e.browser_app.toLowerCase();
    if (!app.includes("chrome")) continue;
    if (e.trigger === "chrome_new_tab") continue;
    const lc = (e.learning_content?.trim() || e.page_title || "").trim();
    if (!lc || lc === "空白新标签页") continue;
    const prev = seen.get(lc);
    if (!prev || new Date(e.at).getTime() > new Date(prev.at).getTime()) {
      seen.set(lc, e);
    }
  }
  return [...seen.values()].sort((a, b) => new Date(b.at).getTime() - new Date(a.at).getTime());
}

/** 卡片时间线：仅时、分 */
function eventTimeHm(ev: DesktopEventJson): string {
  try {
    const d = new Date(ev.at);
    return d.toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" });
  } catch {
    return "—";
  }
}

function appBadgeKind(ev: DesktopEventJson): "chrome" | "edge" | "wechat" | "code" | "terminal" | "default" {
  const a = (
    ev.type === "app_switch" || ev.type === "learning_snapshot"
      ? ev.app
      : ev.type === "browser_moment"
        ? ev.browser_app
        : "clipboard"
  ).replace(/\.exe$/i, "");
  const s = a.toLowerCase();
  if (s.includes("chrome") || s.includes("chromium")) return "chrome";
  if (s.includes("msedge") || s.includes("edge")) return "edge";
  if (s.includes("wechat") || s.includes("微信")) return "wechat";
  if (s.includes("code") || s.includes("vscode")) return "code";
  if (s.includes("wt") || s.includes("terminal") || s.includes("powershell")) return "terminal";
  return "default";
}

const BADGE_GRADIENT: Record<
  ReturnType<typeof appBadgeKind>,
  string
> = {
  chrome: "from-blue-500 to-blue-600 shadow-blue-500/20",
  edge: "from-cyan-500 to-blue-600 shadow-cyan-500/20",
  wechat: "from-emerald-500 to-green-600 shadow-emerald-500/20",
  code: "from-sky-500 to-indigo-600 shadow-sky-500/20",
  terminal: "from-neutral-600 to-neutral-800 shadow-neutral-500/20",
  default: "from-violet-500 to-purple-600 shadow-violet-500/20",
};

function AppBadge({ ev }: { ev: DesktopEventJson }) {
  if (ev.type === "browser_moment") {
    const label = ev.browser_app.replace(/\.exe$/i, "");
    const initials = label.slice(0, 2).toUpperCase() || "?";
    const k = appBadgeKind(ev);
    return (
      <div
        className={`flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-gradient-to-br text-[11px] font-bold text-white shadow-md ${BADGE_GRADIENT[k]}`}
        aria-hidden
      >
        {initials}
      </div>
    );
  }
  if (ev.type === "clipboard") {
    return (
      <div
        className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-gradient-to-br from-amber-400 to-orange-500 text-[11px] font-bold text-white shadow-md shadow-amber-500/25"
        aria-hidden
      >
        Cb
      </div>
    );
  }
  const label = ev.app.replace(/\.exe$/i, "");
  const initials = label.slice(0, 2).toUpperCase() || "?";
  const k = appBadgeKind(ev);
  return (
    <div
      className={`flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-gradient-to-br text-[11px] font-bold text-white shadow-md ${BADGE_GRADIENT[k]}`}
      aria-hidden
    >
      {initials}
    </div>
  );
}

function EventTypeTags({ ev }: { ev: DesktopEventJson }) {
  if (ev.type === "app_switch") {
    return (
      <div className="flex flex-wrap justify-end gap-1">
        <span className="rounded-full bg-sky-50 px-2 py-0.5 text-[10px] font-medium text-sky-800">前台</span>
        <span className="rounded-full bg-neutral-100 px-2 py-0.5 text-[10px] font-medium text-neutral-600">切换</span>
        {ev.image_rel ? (
          <span className="rounded-full bg-blue-50 px-2 py-0.5 text-[10px] font-medium text-blue-900">窗口截图</span>
        ) : null}
      </div>
    );
  }
  if (ev.type === "clipboard") {
    return (
      <div className="flex flex-wrap justify-end gap-1">
        <span className="rounded-full bg-amber-50 px-2 py-0.5 text-[10px] font-medium text-amber-900">剪贴板</span>
        {ev.truncated ? (
          <span className="rounded-full bg-neutral-100 px-2 py-0.5 text-[10px] font-medium text-neutral-500">已截断</span>
        ) : null}
      </div>
    );
  }
  if (ev.type === "learning_snapshot") {
    return (
      <div className="flex flex-wrap justify-end gap-1">
        <span className="rounded-full bg-emerald-50 px-2 py-0.5 text-[10px] font-medium text-emerald-900">学习快照</span>
        <span className="rounded-full bg-violet-50 px-2 py-0.5 text-[10px] font-medium text-violet-800">全屏</span>
      </div>
    );
  }
  if (ev.type === "browser_moment") {
    const t = ev.trigger;
    const isBili = t === "bilibili" || t === "bilibili_open" || t === "bilibili_video";
    const site = isBili ? (
      <span className="rounded-full bg-pink-50 px-2 py-0.5 text-[10px] font-medium text-pink-900">
        {isBilibiliDesktopExe(ev.browser_app) ? "哔哩·桌面" : "哔哩哔哩"}
      </span>
    ) : (
      <span className="rounded-full bg-indigo-50 px-2 py-0.5 text-[10px] font-medium text-indigo-900">Chrome</span>
    );
    const sub =
      t === "bilibili_video" || t === "bilibili"
        ? "新内容/视频"
        : t === "bilibili_open"
          ? "进入学习页"
          : t === "chrome_new_tab"
            ? "新标签页"
            : t === "chrome_tab" || t === "tab_switch"
              ? "标签切换"
              : "学习时刻";
    return (
      <div className="flex flex-wrap justify-end gap-1">
        {site}
        <span className="rounded-full bg-blue-50 px-2 py-0.5 text-[10px] font-medium text-blue-900">{sub}</span>
      </div>
    );
  }
  return null;
}

function EventPrimaryTitle({ ev }: { ev: DesktopEventJson }) {
  if (ev.type === "app_switch") {
    const t = ev.title?.trim();
    return <span className="font-semibold text-neutral-900">{t || ev.app}</span>;
  }
  if (ev.type === "clipboard") {
    return <span className="font-semibold text-neutral-900">剪贴板文本更新</span>;
  }
  if (ev.type === "browser_moment") {
    return <span className="font-semibold text-neutral-900">{ev.summary}</span>;
  }
  if (ev.type === "learning_snapshot") {
    const t = ev.title?.trim();
    return <span className="font-semibold text-neutral-900">{t ? `${ev.app} · ${t}` : ev.app}</span>;
  }
  return null;
}

function EventSecondaryLine({ ev }: { ev: DesktopEventJson }) {
  if (ev.type === "app_switch") {
    const dims =
      ev.image_rel && ev.width_px != null && ev.height_px != null
        ? ` · 截图 ${ev.width_px}×${ev.height_px}`
        : null;
    return (
      <p className="mt-1 line-clamp-2 text-[12px] leading-relaxed text-neutral-500">
        前台应用：<span className="text-neutral-700">{ev.app}</span>
        {ev.title?.trim() ? ` · ${ev.title.trim()}` : null}
        {dims}
      </p>
    );
  }
  if (ev.type === "clipboard") {
    const text = ev.text_preview.length > 160 ? `${ev.text_preview.slice(0, 160)}…` : ev.text_preview;
    return (
      <p className="mt-1 line-clamp-3 whitespace-pre-wrap text-[12px] leading-relaxed text-neutral-500">{text}</p>
    );
  }
  if (ev.type === "browser_moment") {
    const lc = ev.learning_content?.trim() || ev.page_title || "—";
    return (
      <p className="mt-1 line-clamp-2 text-[12px] leading-relaxed text-neutral-500">
        学习内容：<span className="text-neutral-700">{lc}</span>
      </p>
    );
  }
  if (ev.type === "learning_snapshot") {
    return (
      <p className="mt-1 text-[12px] leading-relaxed text-neutral-500">
        主显示器截图 · {ev.width_px}×{ev.height_px} · 已保存至数据目录
      </p>
    );
  }
  return null;
}

function SnapshotThumb({
  imageRel,
  variant = "compact",
}: {
  imageRel: string;
  /** compact：时间线卡片内小预览；full：详情弹窗内完整展示截图比例，不裁切 */
  variant?: "compact" | "full";
}) {
  const [src, setSrc] = useState<string | null>(null);
  const [phase, setPhase] = useState<"load" | "ok" | "err">("load");

  useEffect(() => {
    if (!isTauri() || !imageRel) {
      setPhase("err");
      return;
    }
    let alive = true;
    void invoke<string>("resolve_data_file", { rel: imageRel })
      .then((abs) => {
        if (!alive) return;
        try {
          setSrc(convertFileSrc(abs));
          setPhase("ok");
        } catch {
          setPhase("err");
        }
      })
      .catch(() => {
        if (alive) setPhase("err");
      });
    return () => {
      alive = false;
    };
  }, [imageRel]);

  if (!isTauri()) {
    return (
      <div className="mt-3 flex h-28 items-center justify-center rounded-xl bg-neutral-50 text-[11px] text-neutral-400 ring-1 ring-neutral-100">
        桌面端可预览截图
      </div>
    );
  }
  if (phase === "load") {
    return (
      <div className="mt-3 h-32 animate-pulse rounded-xl bg-neutral-100 ring-1 ring-neutral-100" aria-hidden />
    );
  }
  if (phase === "err" || !src) {
    return (
      <div className="mt-3 flex h-24 items-center justify-center rounded-xl bg-neutral-50 text-[11px] text-neutral-400 ring-1 ring-neutral-100">
        无法加载预览
      </div>
    );
  }
  const imgClass =
    variant === "full"
      ? "mt-3 h-auto w-full max-w-full rounded-xl bg-neutral-50 shadow-sm ring-1 ring-neutral-200/80"
      : "mt-3 max-h-44 w-full rounded-xl bg-neutral-50 object-contain object-top shadow-sm ring-1 ring-neutral-200/80";
  return <img src={src} alt="" className={imgClass} draggable={false} />;
}

function DailyEventCard({
  ev,
  onOpenDetail,
}: {
  ev: DesktopEventJson;
  onOpenDetail?: (ev: DesktopEventJson) => void;
}) {
  const hasAppShot = ev.type === "app_switch" && !!ev.image_rel;
  const sub =
    ev.type === "app_switch"
      ? ev.app
      : ev.type === "clipboard"
        ? "系统剪贴板"
        : ev.type === "browser_moment"
          ? ev.browser_app
          : ev.app;
  const canDetail = ev.type === "browser_moment" || ev.type === "learning_snapshot" || hasAppShot;
  return (
    <div
      className={`rounded-2xl border border-neutral-200/90 bg-white p-4 shadow-sm shadow-neutral-200/40 ${
        canDetail && onOpenDetail ? "cursor-pointer transition hover:border-blue-200/80 hover:shadow-md" : ""
      }`}
      role={canDetail ? "button" : undefined}
      tabIndex={canDetail ? 0 : undefined}
      onClick={() => {
        if (canDetail && onOpenDetail) onOpenDetail(ev);
      }}
      onKeyDown={(e) => {
        if (canDetail && onOpenDetail && (e.key === "Enter" || e.key === " ")) {
          e.preventDefault();
          onOpenDetail(ev);
        }
      }}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="flex min-w-0 flex-1 items-start gap-3">
          <AppBadge ev={ev} />
          <div className="min-w-0 flex-1">
            <div className="flex flex-wrap items-center gap-2">
              <span className="text-[12px] font-medium text-neutral-400">{sub}</span>
            </div>
            <div className="mt-0.5">
              <EventPrimaryTitle ev={ev} />
            </div>
            <EventSecondaryLine ev={ev} />
          </div>
        </div>
        <div className="max-w-[48%] shrink-0">
          <EventTypeTags ev={ev} />
        </div>
      </div>
      {ev.type === "learning_snapshot" || ev.type === "browser_moment" ? (
        <SnapshotThumb imageRel={ev.image_rel} />
      ) : hasAppShot ? (
        <SnapshotThumb imageRel={ev.image_rel!} />
      ) : null}
    </div>
  );
}

function EventDetailModal({
  ev,
  onClose,
}: {
  ev: DesktopEventJson | null;
  onClose: () => void;
}) {
  if (!ev) return null;
  const appShot = ev.type === "app_switch" && ev.image_rel;
  if (ev.type !== "browser_moment" && ev.type !== "learning_snapshot" && !appShot) return null;

  return (
    <div
      className="fixed inset-0 z-[200] flex items-center justify-center bg-black/40 p-4 backdrop-blur-[2px]"
      role="presentation"
      onClick={onClose}
    >
      <div
        role="dialog"
        aria-modal="true"
        className="max-h-[90vh] w-full max-w-4xl overflow-y-auto rounded-2xl border border-neutral-200/90 bg-white p-6 shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="mb-4 flex items-start justify-between gap-3">
          <div className="min-w-0">
            <p className="text-[11px] font-medium uppercase tracking-wide text-neutral-400">活动详情</p>
            <h2 className="mt-1 text-[16px] font-semibold leading-snug text-neutral-900">
              {ev.type === "browser_moment"
                ? ev.summary
                : ev.type === "learning_snapshot"
                  ? "学习快照"
                  : ev.title?.trim() || ev.app}
            </h2>
            <p className="mt-1 font-mono text-[12px] text-neutral-500">{eventTimeHm(ev)}</p>
          </div>
          <button
            type="button"
            className="shrink-0 rounded-lg px-2 py-1 text-[13px] text-neutral-500 hover:bg-neutral-100"
            onClick={onClose}
          >
            关闭
          </button>
        </div>

        {ev.type === "browser_moment" ? (
          <>
            <section className="mb-4">
              <h3 className="mb-1.5 text-[11px] font-semibold uppercase tracking-wide text-neutral-400">摘要</h3>
              <p className="text-[13px] leading-relaxed text-neutral-700">{ev.summary}</p>
            </section>
            <section className="mb-4">
              <h3 className="mb-1.5 text-[11px] font-semibold uppercase tracking-wide text-neutral-400">详情</h3>
              <dl className="space-y-1.5 text-[12px]">
                <div className="flex gap-2">
                  <dt className="w-24 shrink-0 text-neutral-500">触发</dt>
                  <dd className="text-neutral-800">
                    {ev.trigger === "bilibili_video" || ev.trigger === "bilibili"
                      ? isBilibiliDesktopExe(ev.browser_app)
                        ? "哔哩哔哩桌面端：窗口标题变化（新内容/新视频）"
                        : "Chrome 网页哔哩：标题变化（新视频/新页面）"
                      : ev.trigger === "bilibili_open"
                        ? isBilibiliDesktopExe(ev.browser_app)
                          ? "哔哩哔哩桌面端：首次记录到窗口标题"
                          : "进入哔哩哔哩网页学习页"
                        : ev.trigger === "chrome_new_tab"
                          ? "Chrome 新标签页"
                          : ev.trigger === "chrome_tab" || ev.trigger === "tab_switch"
                            ? "Chrome 标签/标题变化"
                            : ev.trigger}
                  </dd>
                </div>
                <div className="flex gap-2">
                  <dt className="w-24 shrink-0 text-neutral-500">窗口标题</dt>
                  <dd className="break-all text-neutral-800">{ev.window_title}</dd>
                </div>
                <div className="flex gap-2">
                  <dt className="w-24 shrink-0 text-neutral-500">学习内容</dt>
                  <dd className="break-all text-neutral-800">
                    {ev.learning_content?.trim() || ev.page_title || "—"}
                  </dd>
                </div>
                <div className="flex gap-2">
                  <dt className="w-24 shrink-0 text-neutral-500">页签近似</dt>
                  <dd className="break-all text-neutral-800">{ev.page_title || "—"}</dd>
                </div>
                <div className="flex gap-2">
                  <dt className="w-24 shrink-0 text-neutral-500">分辨率</dt>
                  <dd className="text-neutral-800">
                    {ev.width_px}×{ev.height_px}
                  </dd>
                </div>
              </dl>
            </section>
            <section className="mb-4">
              <h3 className="mb-2 text-[11px] font-semibold uppercase tracking-wide text-neutral-400">关键词</h3>
              <div className="flex flex-wrap gap-1.5">
                {ev.keywords.map((k) => (
                  <span
                    key={k}
                    className="rounded-full bg-neutral-100 px-2.5 py-0.5 text-[11px] font-medium text-neutral-700"
                  >
                    {k}
                  </span>
                ))}
              </div>
            </section>
            <section>
              <h3 className="mb-2 text-[11px] font-semibold uppercase tracking-wide text-neutral-400">截图</h3>
              <SnapshotThumb imageRel={ev.image_rel} variant="full" />
            </section>
          </>
        ) : ev.type === "learning_snapshot" ? (
          <section>
            <p className="mb-3 text-[13px] text-neutral-600">
              {ev.title?.trim() ? `${ev.app} · ${ev.title.trim()}` : ev.app} · {ev.width_px}×{ev.height_px}
            </p>
            <SnapshotThumb imageRel={ev.image_rel} variant="full" />
          </section>
        ) : ev.type === "app_switch" && ev.image_rel ? (
          <section>
            <p className="mb-3 text-[13px] leading-relaxed text-neutral-600">
              每次切换到此前台应用时，会尝试保存<strong className="font-medium">当前前台窗口</strong>客户区截图（与标题栏所示窗口一致），仅保存在本机数据目录。
            </p>
            <dl className="mb-4 space-y-1.5 text-[12px]">
              <div className="flex gap-2">
                <dt className="w-24 shrink-0 text-neutral-500">进程</dt>
                <dd className="break-all text-neutral-800">{ev.app}</dd>
              </div>
              {ev.title?.trim() ? (
                <div className="flex gap-2">
                  <dt className="w-24 shrink-0 text-neutral-500">窗口标题</dt>
                  <dd className="break-all text-neutral-800">{ev.title.trim()}</dd>
                </div>
              ) : null}
              {ev.width_px != null && ev.height_px != null ? (
                <div className="flex gap-2">
                  <dt className="w-24 shrink-0 text-neutral-500">截图尺寸</dt>
                  <dd className="text-neutral-800">
                    {ev.width_px}×{ev.height_px}
                  </dd>
                </div>
              ) : null}
            </dl>
            <SnapshotThumb imageRel={ev.image_rel} variant="full" />
          </section>
        ) : null}
      </div>
    </div>
  );
}

function DailyEventTimeline({
  events,
  onOpenDetail,
}: {
  events: DesktopEventJson[];
  onOpenDetail?: (ev: DesktopEventJson) => void;
}) {
  const ordered = useMemo(() => [...events].reverse(), [events]);
  return (
    <div className="max-h-[520px] overflow-auto pr-1">
      <div className="relative border-l-2 border-neutral-100 pl-5">
        {ordered.map((ev, i) => {
          const dotClass =
            ev.type === "app_switch"
              ? "bg-sky-500"
              : ev.type === "clipboard"
                ? "bg-amber-400"
                : ev.type === "browser_moment"
                  ? ev.trigger === "bilibili_video" ||
                      ev.trigger === "bilibili_open" ||
                      ev.trigger === "bilibili"
                    ? "bg-pink-500"
                    : "bg-indigo-500"
                  : "bg-emerald-500";
          return (
            <article key={`${ev.at}-${i}`} className="relative mb-8 last:mb-2">
              <div
                className={`absolute -left-[21px] top-1 z-10 h-3 w-3 rounded-full border-2 border-white ${dotClass} shadow-sm`}
                aria-hidden
              />
              <div className="mb-2 text-[12px] font-semibold tabular-nums text-neutral-800">{eventTimeHm(ev)}</div>
              <DailyEventCard ev={ev} onOpenDetail={onOpenDetail} />
            </article>
          );
        })}
      </div>
    </div>
  );
}

// --- icons (inline SVG) ---

function IconPanelLeft({ className }: { className?: string }) {
  return (
    <svg className={className} width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
      <rect x="3" y="4" width="18" height="16" rx="2" />
      <path d="M9 4v16" />
    </svg>
  );
}

function IconHome({ className }: { className?: string }) {
  return (
    <svg className={className} width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M3 10.5L12 3l9 7.5" />
      <path d="M5 10v10h14V10" />
    </svg>
  );
}

function IconAgent({ className }: { className?: string }) {
  return (
    <svg className={className} width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
      <path d="M5 9h14M8 14h8M6 19h12" />
      <path d="M4 5h4v4H4z" />
    </svg>
  );
}

function IconRewind({ className }: { className?: string }) {
  return (
    <svg className={className} width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
      <circle cx="12" cy="12" r="8" />
      <path d="M12 8v4l3 2" />
      <path d="M6 12a6 6 0 0 1 6-6" />
    </svg>
  );
}

function IconTasks({ className }: { className?: string }) {
  return (
    <svg className={className} width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
      <path d="M8 6h13M8 12h13M8 18h13" />
      <path d="M3 6h1M3 12h1M3 18h1" />
    </svg>
  );
}

function IconFolderPlus({ className }: { className?: string }) {
  return (
    <svg className={className} width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
      <path d="M4 20h16a1 1 0 0 0 1-1V7a1 1 0 0 0-1-1h-7l-2-2H4a1 1 0 0 0-1 1v13a1 1 0 0 0 1 1z" />
      <path d="M12 10v6M9 13h6" />
    </svg>
  );
}

function IconCalendar({ className }: { className?: string }) {
  return (
    <svg className={className} width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
      <rect x="3" y="5" width="18" height="16" rx="2" />
      <path d="M16 3v4M8 3v4M3 11h18" />
    </svg>
  );
}

function IconBell({ className }: { className?: string }) {
  return (
    <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M18 8A6 6 0 0 0 6 8c0 7-3 7-3 7h18s-3 0-3-7" />
      <path d="M13.73 21a2 2 0 0 1-3.46 0" />
    </svg>
  );
}

function IconClockEmpty({ className }: { className?: string }) {
  return (
    <svg className={className} width="56" height="56" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round">
      <circle cx="12" cy="12" r="9" />
      <path d="M12 7v6l4 2" />
    </svg>
  );
}

function IconStarOrange({ className }: { className?: string }) {
  return (
    <svg className={className} width="18" height="18" viewBox="0 0 24 24" fill="#f97316" aria-hidden>
      <path d="M12 2l2.4 7.4H22l-6 4.6 2.3 7L12 17.8 5.7 21l2.3-7-6-4.6h7.6L12 2z" />
    </svg>
  );
}

/** 回顾：今日卡片 + 复习日历（占位数据，后续接卡片/复习队列） */
function RewindReviewPage() {
  const [taskFilter, setTaskFilter] = useState<"confirmed" | "pending">("confirmed");
  const [calView, setCalView] = useState<"month" | "week">("month");
  const [calMonth, setCalMonth] = useState(() => {
    const d = new Date();
    return new Date(d.getFullYear(), d.getMonth(), 1);
  });
  const [selectedDate, setSelectedDate] = useState(() => new Date());
  const [dayTimelineRaw, setDayTimelineRaw] = useState<string | null>(null);
  const [dayTimelineLoading, setDayTimelineLoading] = useState(() => isTauri());

  const y = calMonth.getFullYear();
  const m = calMonth.getMonth();
  const monthCells = useMemo(() => buildMonthCalendarCells(y, m), [y, m]);

  const weekCells = useMemo(() => {
    const start = startOfWeekSunday(selectedDate);
    return Array.from({ length: 7 }, (_, i) => {
      const d = new Date(start);
      d.setDate(start.getDate() + i);
      return d;
    });
  }, [selectedDate]);

  const goMonth = (delta: number) => {
    setCalMonth((prev) => new Date(prev.getFullYear(), prev.getMonth() + delta, 1));
  };

  const goWeek = (delta: number) => {
    setSelectedDate((prev) => {
      const n = new Date(prev);
      n.setDate(n.getDate() + delta * 7);
      setCalMonth(new Date(n.getFullYear(), n.getMonth(), 1));
      return n;
    });
  };

  /** 与稿一致：四月 12 - 18, 2026 */
  const weekRangeLabel = useMemo(() => {
    const a = weekCells[0];
    const b = weekCells[6];
    const y = a.getFullYear();
    if (a.getMonth() === b.getMonth() && a.getFullYear() === b.getFullYear()) {
      return `${ZH_MONTHS[a.getMonth()]} ${a.getDate()} - ${b.getDate()}, ${y}`;
    }
    if (a.getFullYear() === b.getFullYear()) {
      return `${ZH_MONTHS[a.getMonth()]} ${a.getDate()} - ${ZH_MONTHS[b.getMonth()]} ${b.getDate()}, ${y}`;
    }
    return `${ZH_MONTHS[a.getMonth()]} ${a.getDate()}, ${a.getFullYear()} - ${ZH_MONTHS[b.getMonth()]} ${b.getDate()}, ${b.getFullYear()}`;
  }, [weekCells]);

  useEffect(() => {
    if (!isTauri()) {
      setDayTimelineRaw(null);
      return;
    }
    let cancelled = false;
    (async () => {
      setDayTimelineLoading(true);
      try {
        const d = selectedDate;
        const raw = await invoke<string>("get_timeline_local_date", {
          year: d.getFullYear(),
          month: d.getMonth() + 1,
          day: d.getDate(),
        });
        if (!cancelled) setDayTimelineRaw(raw);
      } catch {
        if (!cancelled) setDayTimelineRaw("[]");
      } finally {
        if (!cancelled) setDayTimelineLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [selectedDate]);

  const learningCards = useMemo(() => {
    const events = parseTimelineEvents(dayTimelineRaw ?? "");
    if (!events) return [];
    return learningCardsFromDayEvents(events);
  }, [dayTimelineRaw]);

  return (
    <div className="flex h-full min-h-[560px] flex-col gap-4">
      <div className="flex min-h-0 flex-1 flex-col gap-4 lg:flex-row lg:gap-6">
        {/* 左侧：今日卡片（Chrome / 哔哩哔哩 学习主题） */}
        <section className="flex w-full shrink-0 flex-col rounded-2xl border border-neutral-200/80 bg-white p-4 shadow-sm lg:w-[280px]">
          <div className="mb-3 flex items-center gap-2">
            <IconStarOrange />
            <h2 className="text-[15px] font-semibold text-neutral-900">今日卡片</h2>
            <span className="rounded-md bg-neutral-100 px-1.5 py-0.5 text-[10px] font-semibold uppercase tracking-wide text-neutral-500">
              BETA
            </span>
          </div>
          <div className="mb-4 inline-flex w-fit rounded-full border border-neutral-200 bg-neutral-50/80 px-3 py-1.5 text-[13px] font-medium text-neutral-800">
            {formatRewindDatePill(selectedDate)}
          </div>
          <div className="flex min-h-[200px] flex-1 flex-col gap-2 overflow-y-auto rounded-xl bg-neutral-50/60 p-2">
            {!isTauri() ? (
              <p className="py-8 text-center text-[13px] text-neutral-400">桌面版连接后展示学习卡片</p>
            ) : dayTimelineLoading ? (
              <p className="py-8 text-center text-[13px] text-neutral-400">加载中…</p>
            ) : learningCards.length === 0 ? (
              <p className="py-8 text-center text-[13px] text-neutral-400">
                该日暂无从 Chrome / 哔哩哔哩解析的学习主题
              </p>
            ) : (
              learningCards.map((ev) => {
                const title = ev.learning_content?.trim() || ev.page_title || ev.summary;
                const isBili =
                  ev.trigger === "bilibili" ||
                  ev.trigger === "bilibili_open" ||
                  ev.trigger === "bilibili_video";
                return (
                  <article
                    key={`${ev.at}-${title}`}
                    className="rounded-xl border border-neutral-200/90 bg-white p-3 shadow-sm"
                  >
                    <div className="mb-1.5 flex items-start justify-between gap-2">
                      <span
                        className={`shrink-0 rounded-md px-1.5 py-0.5 text-[10px] font-semibold ${
                          isBili
                            ? "bg-pink-50 text-pink-800"
                            : "bg-indigo-50 text-indigo-800"
                        }`}
                      >
                        {isBili ? "哔哩哔哩" : "Chrome"}
                      </span>
                      <span className="font-mono text-[10px] text-neutral-400">{eventTimeHm(ev)}</span>
                    </div>
                    <h3 className="line-clamp-3 text-[13px] font-semibold leading-snug text-neutral-900">{title}</h3>
                    {ev.keywords.length > 0 ? (
                      <div className="mt-2 flex flex-wrap gap-1">
                        {ev.keywords.slice(0, 5).map((k) => (
                          <span
                            key={k}
                            className="rounded-full bg-neutral-100 px-2 py-0.5 text-[10px] font-medium text-neutral-600"
                          >
                            {k}
                          </span>
                        ))}
                      </div>
                    ) : null}
                  </article>
                );
              })
            )}
          </div>
        </section>

        {/* 右侧：日历 */}
        <section className="flex min-h-[480px] min-w-0 flex-1 flex-col rounded-2xl border border-neutral-200/80 bg-white p-4 shadow-sm lg:p-5">
          <div className="mb-4 flex flex-col gap-3 sm:flex-row sm:flex-wrap sm:items-center sm:justify-between">
            <div className="inline-flex rounded-lg bg-neutral-100 p-0.5">
              <button
                type="button"
                onClick={() => setTaskFilter("confirmed")}
                className={`rounded-md px-3 py-1.5 text-[12px] font-medium transition-colors ${
                  taskFilter === "confirmed"
                    ? "bg-blue-950 text-white"
                    : "text-neutral-600 hover:text-neutral-900"
                }`}
              >
                已确认任务
              </button>
              <button
                type="button"
                onClick={() => setTaskFilter("pending")}
                className={`rounded-md px-3 py-1.5 text-[12px] font-medium transition-colors ${
                  taskFilter === "pending"
                    ? "bg-blue-950 text-white"
                    : "text-neutral-600 hover:text-neutral-900"
                }`}
              >
                待处理任务
              </button>
            </div>

            <div className="flex flex-wrap items-center justify-center gap-2 sm:flex-1">
              <button
                type="button"
                onClick={() => (calView === "month" ? goMonth(-1) : goWeek(-1))}
                className="rounded-lg p-1.5 text-neutral-500 hover:bg-neutral-100 hover:text-neutral-800"
                aria-label={calView === "month" ? "上一月" : "上一周"}
              >
                ‹
              </button>
              <span className="min-w-[10rem] text-center text-[13px] font-semibold text-neutral-900 sm:min-w-[14rem] sm:text-[14px]">
                {calView === "month" ? `${ZH_MONTHS[m]} ${y}` : weekRangeLabel}
              </span>
              <button
                type="button"
                onClick={() => (calView === "month" ? goMonth(1) : goWeek(1))}
                className="rounded-lg p-1.5 text-neutral-500 hover:bg-neutral-100 hover:text-neutral-800"
                aria-label={calView === "month" ? "下一月" : "下一周"}
              >
                ›
              </button>
            </div>

            <div className="inline-flex rounded-lg bg-neutral-100 p-0.5">
              <button
                type="button"
                onClick={() => setCalView("week")}
                className={`rounded-md px-3 py-1.5 text-[12px] font-medium transition-colors ${
                  calView === "week"
                    ? "bg-white text-neutral-900 shadow-sm ring-1 ring-neutral-200/80"
                    : "text-neutral-600 hover:text-neutral-900"
                }`}
              >
                周视图
              </button>
              <button
                type="button"
                onClick={() => setCalView("month")}
                className={`rounded-md px-3 py-1.5 text-[12px] font-medium transition-colors ${
                  calView === "month"
                    ? "bg-white text-neutral-900 shadow-sm ring-1 ring-neutral-200/80"
                    : "text-neutral-600 hover:text-neutral-900"
                }`}
              >
                月视图
              </button>
            </div>
          </div>

          {calView === "month" ? (
            <div className="flex min-h-0 flex-1 flex-col overflow-auto">
              <div className="mb-2 grid grid-cols-7 gap-1 text-center text-[11px] font-medium text-neutral-400">
                {ZH_WEEKDAYS_SHORT.map((d) => (
                  <div key={d} className="py-1">
                    {d}
                  </div>
                ))}
              </div>
              <div className="grid grid-cols-7 gap-1">
                {monthCells.map(({ date, inMonth }, idx) => {
                  const isSel = sameDay(date, selectedDate);
                  const isToday = sameDay(date, new Date());
                  return (
                    <button
                      key={`${date.toISOString()}-${idx}`}
                      type="button"
                      onClick={() => {
                        setSelectedDate(date);
                        setCalMonth(new Date(date.getFullYear(), date.getMonth(), 1));
                      }}
                      className={`relative flex aspect-square max-h-[52px] flex-col rounded-lg border-2 bg-white p-1.5 text-left transition-colors ${
                        isToday
                          ? "border-[#5c7097]"
                          : isSel
                            ? "border-slate-400 hover:bg-neutral-50/80"
                            : "border-transparent hover:bg-neutral-50"
                      }`}
                    >
                      <div className="flex w-full flex-1 flex-col items-end">
                        {isToday ? (
                          <span className="flex h-7 w-7 shrink-0 items-center justify-center rounded-full bg-[#5c7097] text-[13px] font-semibold text-white shadow-sm">
                            {date.getDate()}
                          </span>
                        ) : (
                          <span
                            className={`text-[13px] font-medium tabular-nums ${
                              inMonth ? "text-neutral-800" : "text-neutral-300"
                            } ${isSel ? "font-semibold text-neutral-900" : ""}`}
                          >
                            {date.getDate()}
                          </span>
                        )}
                      </div>
                    </button>
                  );
                })}
              </div>
            </div>
          ) : (
            <div className="flex min-h-0 flex-1 flex-col gap-2 overflow-hidden">
              <div className="grid grid-cols-7 gap-2 text-center text-[11px] font-medium text-neutral-400">
                {ZH_WEEKDAYS_SHORT.map((d) => (
                  <div key={`w-${d}`} className="py-1">
                    {d}
                  </div>
                ))}
              </div>
              <div className="grid min-h-[min(420px,52vh)] flex-1 grid-cols-7 gap-2">
                {weekCells.map((date, idx) => {
                  const isSel = sameDay(date, selectedDate);
                  const isToday = sameDay(date, new Date());
                  return (
                    <button
                      key={idx}
                      type="button"
                      onClick={() => setSelectedDate(date)}
                      className={`flex min-h-0 min-w-0 flex-col rounded-2xl border text-left transition-colors ${
                        isSel
                          ? "border-blue-400 bg-neutral-100/95 ring-1 ring-blue-200/80"
                          : "border-neutral-200/90 bg-neutral-100/80 hover:border-neutral-300 hover:bg-neutral-50"
                      }`}
                    >
                      <div className="flex shrink-0 justify-end px-2 pb-1 pt-2">
                        <span
                          className={`flex h-7 min-w-[1.75rem] items-center justify-center tabular-nums ${
                            isToday
                              ? "rounded-full bg-blue-900 text-[13px] font-semibold text-white shadow-sm"
                              : "text-[13px] font-medium text-neutral-600"
                          }`}
                        >
                          {date.getDate()}
                        </span>
                      </div>
                      <div className="min-h-0 flex-1 overflow-y-auto px-2 pb-3 [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
                        {/* 该日复习卡片将放在此处 */}
                      </div>
                    </button>
                  );
                })}
              </div>
            </div>
          )}
        </section>
      </div>
    </div>
  );
}

function ToggleSwitch({
  on,
  onToggle,
  disabled,
  ariaLabel,
}: {
  on: boolean;
  onToggle: () => void;
  disabled?: boolean;
  ariaLabel: string;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={on}
      aria-label={ariaLabel}
      disabled={disabled}
      onClick={onToggle}
      className={`relative h-7 w-12 shrink-0 rounded-full transition-colors ${
        on ? "bg-[#3b82f6]" : "bg-neutral-300"
      } ${disabled ? "opacity-50 cursor-not-allowed" : "cursor-pointer"}`}
    >
      <span
        className={`absolute top-0.5 h-6 w-6 rounded-full bg-white shadow transition-transform ${
          on ? "left-5" : "left-0.5"
        }`}
      />
    </button>
  );
}

export default function MainPanel() {
  const [nav, setNav] = useState<NavId>("home");
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [centerFilter, setCenterFilter] = useState<CenterFilter>("events");
  const [displayDate] = useState(() => new Date());

  const [timeline, setTimeline] = useState<string>("加载中…");
  const [fg, setFg] = useState<string>("");
  const [analyze, setAnalyze] = useState<string>("");
  const [recording, setRecording] = useState<boolean>(false);
  /** 气泡球窗口是否显示（与 tauri 默认可见一致，刷新时从后端同步） */
  const [bubbleVisible, setBubbleVisible] = useState<boolean>(true);
  const [aiText, setAiText] = useState<string>("");
  const [aiLoading, setAiLoading] = useState<boolean>(false);
  const [bubbleSkin, setBubbleSkin] = useState(() => readBubbleSkin());
  /** 学习快照快捷键防连点（部分环境会重复触发） */
  const lastLearningSnapAt = useRef<number>(0);
  const [learningSnapHint, setLearningSnapHint] = useState<string | null>(null);
  /** 首次开启「记录」时提示前台窗口截图说明（sessionStorage 记一次） */
  const [fgShotTip, setFgShotTip] = useState(false);
  const [detailEvent, setDetailEvent] = useState<DesktopEventJson | null>(null);

  const analyzeMinutes = centerFilter === "events" ? 15 : centerFilter;

  const refresh = useCallback(async () => {
    setBubbleSkin(readBubbleSkin());
    if (!isTauri()) {
      setTimeline(
        "当前不是 Tauri 窗口（例如在纯浏览器里打开了 Vite）。请关闭本页，在项目目录执行 npm run tauri:dev，使用弹出的桌面窗口。"
      );
      setFg("");
      setAnalyze("");
      return;
    }
    try {
      const t = await invoke<string>("get_timeline_today");
      setTimeline(t);
      const f = await invoke<string>("get_foreground_snapshot");
      setFg(f);
      const mins = analyzeMinutes;
      const a = await invoke<string>("analyze_window_minutes", { minutes: mins });
      setAnalyze(a);
      const on = await invoke<boolean>("get_recording_state");
      setRecording(on);
      const bubbleOn = await invoke<boolean>("is_bubble_visible");
      setBubbleVisible(bubbleOn);
    } catch (e) {
      setTimeline(String(e));
      setFg("");
      setAnalyze("");
    }
  }, [analyzeMinutes]);

  useEffect(() => {
    void refresh();
    if (!isTauri()) return;
    const id = window.setInterval(() => void refresh(), 8000);
    return () => window.clearInterval(id);
  }, [refresh]);

  useEffect(() => {
    if (!isTauri()) return;
    let cancelled = false;
    const shortcut = "CommandOrControl+Shift+L";

    void (async () => {
      try {
        const { register } = await import("@tauri-apps/plugin-global-shortcut");
        if (cancelled) return;
        await register(shortcut, async () => {
          if (cancelled) return;
          const now = Date.now();
          if (now - lastLearningSnapAt.current < 900) return;
          lastLearningSnapAt.current = now;
          try {
            await invoke("record_learning_snapshot");
            setLearningSnapHint("已保存学习快照（主屏截图）");
            window.setTimeout(() => setLearningSnapHint(null), 3500);
            await refresh();
          } catch (e) {
            setLearningSnapHint(String(e));
            window.setTimeout(() => setLearningSnapHint(null), 5000);
          }
        });
      } catch (e) {
        if (!cancelled) {
          setLearningSnapHint(`快捷键未就绪：${String(e)}`);
          window.setTimeout(() => setLearningSnapHint(null), 5000);
        }
      }
    })();

    return () => {
      cancelled = true;
      void import("@tauri-apps/plugin-global-shortcut")
        .then(({ unregister }) => unregister(shortcut))
        .catch(() => {});
    };
  }, [refresh]);

  const events = useMemo(() => parseTimelineEvents(timeline), [timeline]);
  const stats = useMemo(() => parseWindowStats(analyze), [analyze]);
  const todayCount = events?.length ?? 0;
  const loadingTimeline = timeline.startsWith("加载");
  const nonTauriHint = timeline.startsWith("当前不是");
  const timelineParseError =
    !loadingTimeline && !nonTauriHint && events === null;

  const startRecording = async () => {
    if (!isTauri()) return;
    try {
      await invoke("start_recording");
      setRecording(true);
      if (
        typeof sessionStorage !== "undefined" &&
        sessionStorage.getItem("guanghe-fg-shot-tip") !== "1"
      ) {
        setFgShotTip(true);
      }
    } catch (e) {
      setTimeline(String(e));
    }
  };

  const stopRecording = async () => {
    if (!isTauri()) return;
    try {
      await invoke("stop_recording");
      setRecording(false);
      await refresh();
    } catch (e) {
      setTimeline(String(e));
    }
  };

  const runAiAnalysis = async () => {
    if (!isTauri()) return;
    setAiLoading(true);
    setAiText("");
    try {
      const t = await invoke<string>("ai_analyze_today");
      setAiText(t);
    } catch (e) {
      setAiText(String(e));
    } finally {
      setAiLoading(false);
    }
  };

  const showBubble = async () => {
    if (!isTauri()) return;
    try {
      await invoke("show_bubble_window");
      setBubbleVisible(true);
    } catch (e) {
      setTimeline(String(e));
    }
  };

  const toggleBubbleVisible = async () => {
    if (!isTauri()) return;
    try {
      if (bubbleVisible) {
        await invoke("hide_bubble_window");
        setBubbleVisible(false);
      } else {
        await invoke("show_bubble_window");
        setBubbleVisible(true);
      }
    } catch (e) {
      setTimeline(String(e));
    }
  };

  const toggleRecording = () => {
    if (recording) void stopRecording();
    else void startRecording();
  };

  const navBtn = (id: NavId, label: string, Icon: typeof IconHome) => {
    const active = nav === id;
    return (
      <button
        type="button"
        onClick={() => setNav(id)}
        className={`flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-[13px] font-medium transition-colors ${
          active
            ? "bg-neutral-100 text-neutral-900"
            : "text-neutral-600 hover:bg-neutral-50 hover:text-neutral-900"
        }`}
      >
        <Icon className={active ? "text-neutral-900" : "text-neutral-500"} />
        {!sidebarCollapsed ? <span>{label}</span> : null}
      </button>
    );
  };

  const filterBtn = (id: CenterFilter, label: string) => {
    const active = centerFilter === id;
    return (
      <button
        type="button"
        onClick={() => setCenterFilter(id)}
        className={`rounded-lg px-3 py-1.5 text-[12px] font-medium transition-colors ${
          active
            ? "bg-neutral-900 text-white"
            : "bg-white text-neutral-600 ring-1 ring-neutral-200 hover:bg-neutral-50"
        }`}
      >
        {label}
      </button>
    );
  };

  return (
    <div className="flex min-h-screen bg-[#f4f4f5] text-neutral-900 antialiased">
      {/*左侧栏 */}
      <aside
        className={`flex shrink-0 flex-col border-r border-neutral-200/90 bg-white ${
          sidebarCollapsed ? "w-[72px]" : "w-[248px]"
        }`}
      >
        <div className="flex h-14 items-center justify-between gap-2 border-b border-neutral-100 px-3">
          <div className={`flex min-w-0 items-center gap-2 ${sidebarCollapsed ? "justify-center" : ""}`}>
            <img
              src={SKIN_SRC.initial}
              alt=""
              className="h-7 w-7 shrink-0 object-contain"
              draggable={false}
            />
            {!sidebarCollapsed ? (
              <span className="truncate text-[15px] font-semibold tracking-tight text-neutral-900">
                光合桌面
              </span>
            ) : null}
          </div>
          {!sidebarCollapsed ? (
            <button
              type="button"
              onClick={() => setSidebarCollapsed(true)}
              className="rounded-lg p-1.5 text-neutral-500 hover:bg-neutral-100 hover:text-neutral-800"
              aria-label="收起侧栏"
            >
              <IconPanelLeft className="h-4 w-4" />
            </button>
          ) : (
            <button
              type="button"
              onClick={() => setSidebarCollapsed(false)}
              className="mx-auto rounded-lg p-1.5 text-neutral-500 hover:bg-neutral-100"
              aria-label="展开侧栏"
            >
              <IconPanelLeft className="h-4 w-4" />
            </button>
          )}
        </div>

        <nav className="flex flex-1 flex-col gap-0.5 p-2">
          {navBtn("home", "首页", IconHome)}
          {navBtn("agent", "助手", IconAgent)}
          {navBtn("rewind", "回顾", IconRewind)}
          {navBtn("tasks", "任务", IconTasks)}

          <div className={`mt-4 px-2 ${sidebarCollapsed ? "hidden" : ""}`}>
            <div className="mb-1 flex items-center justify-between text-[11px] font-semibold uppercase tracking-wide text-neutral-400">
              <span>会话</span>
              <IconFolderPlus className="text-neutral-400" />
            </div>
            <p className="rounded-lg bg-neutral-50 px-2 py-3 text-center text-[12px] text-neutral-400">
              暂无活动会话
            </p>
          </div>
        </nav>

        <div className="border-t border-neutral-100 p-3">
          {!sidebarCollapsed ? (
            <button
              type="button"
              className="mb-3 w-full rounded-xl border border-neutral-200 bg-white px-3 py-2 text-left text-[12px] text-neutral-600 hover:bg-neutral-50"
            >
              向光合桌面发送反馈
            </button>
          ) : null}
          <div className={`flex items-center gap-2 ${sidebarCollapsed ? "justify-center" : ""}`}>
            <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-[#3b82f6] text-[13px] font-semibold text-white">
              {DISPLAY_NAME.slice(0, 1).toUpperCase()}
            </div>
            {!sidebarCollapsed ? (
              <span className="truncate text-[13px] font-medium text-neutral-800">{DISPLAY_NAME}</span>
            ) : null}
          </div>
        </div>
      </aside>

      {/* 中间 + 右侧 */}
      <div className="flex min-w-0 flex-1">
        <main className="min-w-0 flex-1 overflow-auto p-6">
          {nav === "home" ? (
            <>
              <div className="mb-6 flex flex-wrap items-start justify-between gap-4">
                <h1 className="text-[26px] font-semibold tracking-tight text-neutral-900">
                  {greetingForNow()}，{DISPLAY_NAME}
                </h1>
                <div className="flex flex-wrap items-center gap-2">
                  {filterBtn("events", "事件")}
                  {filterBtn(15, "15 分钟")}
                  {filterBtn(30, "30 分钟")}
                  {filterBtn(60, "1 小时")}
                  <div className="ml-1 flex items-center gap-1 rounded-lg bg-white px-2 py-1 ring-1 ring-neutral-200">
                    <button
                      type="button"
                      className="rounded p-1 text-neutral-400 hover:bg-neutral-50 hover:text-neutral-700"
                      aria-label="上一天"
                    >
                      ‹
                    </button>
                    <IconCalendar className="text-neutral-500" />
                    <span className="min-w-[4.5rem] text-center text-[12px] font-medium text-neutral-800">
                      {formatZhDate(displayDate)}
                    </span>
                    <button
                      type="button"
                      className="rounded p-1 text-neutral-400 hover:bg-neutral-50 hover:text-neutral-700"
                      aria-label="下一天"
                    >
                      ›
                    </button>
                  </div>
                </div>
              </div>

              {centerFilter === "events" ? (
                <p className="mb-3 text-[12px] text-neutral-400">
                  以下为今日记录卡片视图；开启记录后，<strong className="font-medium text-neutral-500">前台切换</strong>
                  会尽量附带当前窗口缩略图；调试用的
                  <strong className="font-medium text-neutral-500">原始 JSON</strong>请在「助手」页查看。
                </p>
              ) : null}

              <div className="rounded-2xl border border-neutral-200/80 bg-white p-6 shadow-sm min-h-[420px]">
                {centerFilter === "events" ? (
                  <>
                    {loadingTimeline ? (
                      <div className="flex h-[360px] items-center justify-center text-sm text-neutral-400">
                        加载中…
                      </div>
                    ) : nonTauriHint ? (
                      <div className="flex flex-col items-center justify-center py-16 px-4 text-center">
                        <p className="text-[13px] leading-relaxed text-neutral-600">{timeline}</p>
                      </div>
                    ) : timelineParseError ? (
                      <div className="flex flex-col items-center justify-center py-16 text-center">
                        <p className="text-[13px] font-medium text-neutral-700">无法加载今日事件</p>
                        <pre className="mt-3 max-h-[200px] max-w-full overflow-auto rounded-lg bg-neutral-50 px-3 py-2 text-left text-[11px] text-neutral-600">
                          {timeline}
                        </pre>
                      </div>
                    ) : events!.length === 0 ? (
                      <div className="flex flex-col items-center justify-center py-20 text-center">
                        <IconClockEmpty className="mb-4 text-neutral-300" />
                        <p className="text-[15px] font-medium text-neutral-700">暂无事件</p>
                        <p className="mt-1 max-w-sm text-[13px] text-neutral-500">
                          开始桌面记录后，每次切换到新前台会记录进程与窗口标题，并在 Windows 下尽量附带
                          <strong className="font-medium text-neutral-600">前台窗口截图</strong>；另有剪贴板、浏览器启发式时刻等。Ctrl+Shift+L
                          可手动保存全屏学习快照。
                        </p>
                      </div>
                    ) : (
                      <DailyEventTimeline
                        events={events!}
                        onOpenDetail={(ev) => setDetailEvent(ev)}
                      />
                    )}
                  </>
                ) : stats ? (
                  <div className="space-y-6">
                    <p className="text-[13px] text-neutral-500">
                      近 {stats.window_minutes} 分钟 · 应用切换{" "}
                      <strong className="text-neutral-800">{stats.app_switch_count}</strong> 次 · 剪贴板{" "}
                      <strong className="text-neutral-800">{stats.clipboard_event_count}</strong> 次 · 学习快照{" "}
                      <strong className="text-neutral-800">{stats.learning_snapshot_count ?? 0}</strong> 次 · 浏览器时刻{" "}
                      <strong className="text-neutral-800">{stats.browser_moment_count ?? 0}</strong> 次
                    </p>
                    <div>
                      <h3 className="mb-2 text-[12px] font-semibold text-neutral-500">主导应用 Top 10</h3>
                      <ul className="space-y-1">
                        {stats.dominant_apps.length === 0 ? (
                          <li className="text-[13px] text-neutral-400">暂无数据</li>
                        ) : (
                          stats.dominant_apps.map((row) => (
                            <li
                              key={row.app}
                              className="flex justify-between rounded-lg bg-neutral-50 px-3 py-2 text-[13px]"
                            >
                              <span className="truncate text-neutral-800">{row.app}</span>
                              <span className="shrink-0 font-medium tabular-nums text-neutral-600">
                                {row.count}
                              </span>
                            </li>
                          ))
                        )}
                      </ul>
                    </div>
                  </div>
                ) : (
                  <div className="flex h-[360px] items-center justify-center text-sm text-neutral-400">
                    暂无统计数据
                  </div>
                )}
              </div>
            </>
          ) : null}

          {nav === "agent" ? (
            <div className="space-y-6">
              <h1 className="text-[22px] font-semibold text-neutral-900">助手 · 调试</h1>
              <div className="flex flex-wrap gap-2">
                <button
                  type="button"
                  disabled={aiLoading}
                  onClick={() => void runAiAnalysis()}
                  className="rounded-xl bg-[#3b82f6] px-4 py-2 text-[13px] font-medium text-white hover:bg-blue-600 disabled:opacity-50"
                >
                  {aiLoading ? "AI 分析中…" : "AI 分析今日"}
                </button>
                <button
                  type="button"
                  onClick={() => void showBubble()}
                  className="rounded-xl border border-neutral-200 bg-white px-4 py-2 text-[13px] font-medium text-neutral-800 hover:bg-neutral-50"
                >
                  显示桌面气泡球
                </button>
                <button
                  type="button"
                  onClick={() => void refresh()}
                  className="rounded-xl border border-neutral-200 bg-white px-4 py-2 text-[13px] font-medium text-neutral-800 hover:bg-neutral-50"
                >
                  立即刷新
                </button>
              </div>
              {aiText ? (
                <section>
                  <h2 className="mb-2 text-[12px] font-semibold text-neutral-500">方舟模型分析</h2>
                  <pre className="max-h-[40vh] overflow-auto whitespace-pre-wrap rounded-2xl border border-neutral-200 bg-neutral-50 p-4 text-[12px] text-neutral-800">
                    {aiText}
                  </pre>
                </section>
              ) : null}
              <section>
                <h2 className="mb-2 text-[12px] font-semibold text-neutral-500">前台快照</h2>
                <pre className="max-h-[30vh] overflow-auto rounded-2xl border border-neutral-200 bg-white p-4 text-[11px] text-neutral-800">
                  {fg || "—"}
                </pre>
              </section>
              <section>
                <h2 className="mb-2 text-[12px] font-semibold text-neutral-500">今日事件（JSON）</h2>
                <pre className="max-h-[40vh] overflow-auto whitespace-pre-wrap rounded-2xl border border-neutral-200 bg-white p-4 text-[11px] text-neutral-800">
                  {timeline}
                </pre>
              </section>
            </div>
          ) : null}

          {nav === "rewind" ? (
            <RewindReviewPage />
          ) : null}
          {nav === "tasks" ? (
            <PlaceholderPage title="任务" hint="与日程、待办联动后将在此展示。" />
          ) : null}
        </main>

        {/* 右侧栏：仅首页展示（桌面光合精灵 + 主动助手） */}
        {nav === "home" ? (
          <aside className="w-[280px] shrink-0 border-l border-neutral-200/90 bg-white p-4 lg:w-[300px] lg:p-5">
          <div className="rounded-2xl border border-neutral-200/90 bg-white p-4 shadow-sm">
            <div className="mb-3 space-y-2">
              <div className="flex items-start justify-between gap-2">
                <div>
                  <h2 className="text-[13px] font-semibold text-neutral-900">桌面光合精灵</h2>
                  <p className="text-[11px] text-neutral-500">气泡球与记录联动</p>
                </div>
                <div className="flex shrink-0 flex-col items-end gap-2">
                  <div className="flex items-center gap-2">
                    <span className="text-[10px] text-neutral-400">气泡</span>
                    <ToggleSwitch
                      on={bubbleVisible}
                      onToggle={() => void toggleBubbleVisible()}
                      disabled={!isTauri()}
                      ariaLabel={bubbleVisible ? "隐藏气泡球" : "显示气泡球"}
                    />
                  </div>
                  <div className="flex items-center gap-2">
                    <span className="text-[10px] text-neutral-400">记录</span>
                    <ToggleSwitch
                      on={recording}
                      onToggle={() => toggleRecording()}
                      disabled={!isTauri()}
                      ariaLabel={recording ? "停止记录" : "开始记录"}
                    />
                  </div>
                </div>
              </div>
              <p className="text-[10px] leading-snug text-neutral-400">
                学习快照：<kbd className="rounded bg-neutral-100 px-1 font-mono text-[9px]">Ctrl</kbd>
                +
                <kbd className="rounded bg-neutral-100 px-1 font-mono text-[9px]">Shift</kbd>
                +
                <kbd className="rounded bg-neutral-100 px-1 font-mono text-[9px]">L</kbd>
                （主显示器截图 + 前台窗口，写入今日时间线）
              </p>
              {learningSnapHint ? (
                <p className="rounded-lg bg-blue-50 px-2 py-1.5 text-[11px] text-blue-900 ring-1 ring-blue-100">
                  {learningSnapHint}
                </p>
              ) : null}
              {fgShotTip ? (
                <div className="rounded-lg bg-sky-50 px-2 py-2 text-[11px] leading-snug text-sky-950 ring-1 ring-sky-100">
                  <p>
                    首次开启记录后，每次<strong>切换前台应用</strong>会尝试截取该窗口画面并显示在卡片上。本应用在 Windows
                    下使用系统绘图接口，一般<strong>无需</strong>单独申请「桌面权限」；若某次截图失败，仍会保留进程与标题。
                  </p>
                  <button
                    type="button"
                    className="mt-2 font-medium text-sky-800 underline decoration-sky-300 hover:text-sky-950"
                    onClick={() => {
                      setFgShotTip(false);
                      try {
                        sessionStorage.setItem("guanghe-fg-shot-tip", "1");
                      } catch {
                        /* ignore */
                      }
                    }}
                  >
                    知道了
                  </button>
                </div>
              ) : null}
            </div>
            <div className="flex justify-center py-2">
              <div className="relative h-[168px] w-[168px] overflow-hidden rounded-full bg-[#eff6ff] ring-1 ring-blue-100">
                <img
                  src={SKIN_SRC[bubbleSkin]}
                  alt=""
                  className="h-full w-full object-contain"
                  draggable={false}
                />
              </div>
            </div>
            <div className="mt-2 flex items-center justify-center gap-3 text-[12px] text-neutral-600">
              <span>
今日 <strong className="text-neutral-900 tabular-nums">{todayCount}</strong>
              </span>
              <span className="h-3 w-px bg-neutral-200" />
              <span>
                总计 <strong className="text-neutral-900 tabular-nums">{todayCount}</strong>
              </span>
            </div>
            <button
              type="button"
              onClick={() => void showBubble()}
              className="mt-3 w-full rounded-xl border border-neutral-200 py-2 text-[12px] font-medium text-neutral-700 hover:bg-neutral-50"
            >
              切换情绪
            </button>
            <p className="mt-2 text-center text-[10px] text-neutral-400">精灵外观在气泡上右键切换</p>
          </div>

          <div className="mt-5">
            <h2 className="mb-2 text-[13px] font-semibold text-neutral-900">主动助手</h2>
            <div className="rounded-2xl border border-neutral-200/90 bg-neutral-50/80 p-4">
              <p className="text-[12px] leading-relaxed text-neutral-600">
                我需要一点时间来收集你的节奏与上下文…
              </p>
              <div className="mt-3 flex flex-wrap gap-2">
                <span className="rounded-full bg-blue-100 px-2.5 py-0.5 text-[11px] font-medium text-blue-800">
                  待处理
                </span>
                <span className="inline-flex items-center gap-1 rounded-full bg-white px-2.5 py-0.5 text-[11px] font-medium text-neutral-600 ring-1 ring-neutral-200">
                  <IconBell className="h-3 w-3 text-neutral-400" />
                  执行{" "}
                  {new Date().toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" })}
                </span>
              </div>
            </div>
          </div>
          </aside>
        ) : null}
      </div>
      <EventDetailModal ev={detailEvent} onClose={() => setDetailEvent(null)} />
    </div>
  );
}

function PlaceholderPage({ title, hint }: { title: string; hint: string }) {
  return (
    <div className="rounded-2xl border border-dashed border-neutral-200 bg-white/80 p-12 text-center">
      <h1 className="text-[22px] font-semibold text-neutral-900">{title}</h1>
      <p className="mt-2 text-[13px] text-neutral-500">{hint}</p>
    </div>
  );
}

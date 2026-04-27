import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import clsx from "clsx";
import { AlertTriangle, CheckCircle2, Loader2, Sparkles, X } from "lucide-react";
import { Link } from "react-router-dom";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { generateCards as apiGenerate } from "@/features/card-generation/services/api";
import { rememberBatch } from "@/lib/recent-batches";
import {
  GENERATION_FROM_SCREENSHOT_EVENT,
  type GenerationFromScreenshotPayload,
} from "@/lib/generation-from-screenshot";
import { useToast } from "@/lib/toast";
import { humanizeError, formatRelative } from "@/lib/format";
import { notifyScreenshotCardGenerationStarted } from "@/lib/system-notification";
import {
  computeTrayGenerationState,
  syncTrayGenerationState,
} from "@/lib/tray-generation-sync";
import { isTauri } from "@/lib/tauri";
import type {
  GenerateCardsInput,
  GeneratedCardBatchResult,
} from "@/types/card";
import type { CommandError } from "@/types/common";

/**
 * 一次后台卡片生成任务。
 *
 * 生命周期：`running → success | failed`。
 * 完成（success/failed）后的任务会保留在 store，直到用户手动 dismiss，
 * 或调用 `clearFinished()` 清理。
 */
export interface GenerationTask {
  id: string;
  startedAt: string;
  finishedAt?: string;
  /** 用来在任务栏里显示的简短描述（图片/文本/关键词等）。 */
  summary: string;
  status: "running" | "success" | "failed";
  /** 成功时的结果 batch。 */
  batch?: GeneratedCardBatchResult;
  /** 失败时的错误描述（已经过 `humanizeError` 处理）。 */
  error?: string;
  /** 成功任务是否已被 GeneratePage 消费过，避免重复 hydrate 预览区。 */
  consumed?: boolean;
}

interface GenerationTasksApi {
  tasks: GenerationTask[];
  /** 正在跑的任务数，用于侧边栏徽标。 */
  runningCount: number;
  /**
   * 启动一个新任务。**立即返回 taskId**，不 await 后台执行。
   * 用户可以自由离开页面；完成时 store 会自动更新 + toast 通知。
   */
  startGeneration: (input: GenerateCardsInput) => string;
  /** 把某个完成（success/failed）任务从任务栏移走；running 任务忽略。 */
  dismissTask: (id: string) => void;
  /** 清理所有已完成任务。 */
  clearFinished: () => void;
  /** 把一个 success 任务标记为"已被预览页消费"，避免重复 hydrate。 */
  markConsumed: (id: string) => void;
}

const Ctx = createContext<GenerationTasksApi | null>(null);

/**
 * 生成任务 Provider，必须嵌在 `ToastProvider` 之内（它会调 toast）。
 *
 * 挂在 App 顶层，这样即使用户离开 GeneratePage，任务依然在跑，
 * 完成时通过 toast 通知，并把 batch 登记进 `recent-batches`。
 */
export function GenerationTasksProvider({ children }: { children: ReactNode }) {
  const toast = useToast();
  const [tasks, setTasks] = useState<GenerationTask[]>([]);
  // useToast 里的 api 是稳定的，但防万一，用 ref 保个底；
  // 让 startGeneration 回调不依赖 toast 引用，避免频繁重建。
  const toastRef = useRef(toast);
  useEffect(() => {
    toastRef.current = toast;
  }, [toast]);

  const startGeneration = useCallback(
    (input: GenerateCardsInput): string => {
      const id =
        typeof crypto !== "undefined" && "randomUUID" in crypto
          ? crypto.randomUUID()
          : `gen-${Date.now()}-${Math.random().toString(36).slice(2, 10)}`;
      const startedAt = new Date().toISOString();

      const summary = buildSummary(input);

      setTasks((prev) => [
        ...prev,
        { id, startedAt, summary, status: "running" },
      ]);
      toastRef.current.info("已提交后台生成", `${summary}。完成后会在此通知你。`);

      // 真正的异步：立即 kick off，不 await。
      void (async () => {
        try {
          const batch = await apiGenerate(input);
          const finishedAt = new Date().toISOString();

          rememberBatch({
            batchId: batch.batchId,
            title: batch.cards[0]?.question ?? batch.cards[0]?.keyword,
            cardCount: batch.cards.length,
            sourceType: input.sourceType,
          });

          setTasks((prev) =>
            prev.map((t) =>
              t.id === id
                ? {
                    ...t,
                    status: "success",
                    finishedAt,
                    batch,
                  }
                : t
            )
          );
          toastRef.current.success(
            "卡片已生成",
            `共 ${batch.cards.length} 张。可在「生成卡片」页预览入库，或到「知识宝库」查看。`
          );
        } catch (err) {
          const ce = err as CommandError;
          const msg = humanizeError(ce);
          setTasks((prev) =>
            prev.map((t) =>
              t.id === id
                ? {
                    ...t,
                    status: "failed",
                    finishedAt: new Date().toISOString(),
                    error: msg,
                  }
                : t
            )
          );
          toastRef.current.error("生成失败", msg);
        }
      })();

      return id;
    },
    []
  );

  /** 截图 overlay 独立窗口内通过事件把图片交给主窗口，走同一套后台生成与任务栏逻辑。 */
  useEffect(() => {
    if (!isTauri()) return;
    if (getCurrentWindow().label !== "main") return;

    let cancelled = false;
    let unlisten: (() => void) | undefined;

    void listen<GenerationFromScreenshotPayload>(
      GENERATION_FROM_SCREENSHOT_EVENT,
      (e) => {
        if (cancelled) return;
        const p = e.payload;
        startGeneration({
          sourceText: p.sourceText ?? "",
          sourceType: p.sourceType,
          imageUrls: p.imageUrls ?? [],
          selectedKeyword: p.selectedKeyword,
          contextTitle: p.contextTitle,
          modelProfileId: p.modelProfileId,
        });
        void notifyScreenshotCardGenerationStarted();
      }
    ).then((fn) => {
      if (!cancelled) unlisten = fn;
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [startGeneration]);

  const dismissTask = useCallback((id: string) => {
    setTasks((prev) =>
      prev.filter((t) => !(t.id === id && t.status !== "running"))
    );
  }, []);

  const clearFinished = useCallback(() => {
    setTasks((prev) => prev.filter((t) => t.status === "running"));
  }, []);

  const markConsumed = useCallback((id: string) => {
    setTasks((prev) =>
      prev.map((t) => (t.id === id ? { ...t, consumed: true } : t))
    );
  }, []);

  const runningCount = useMemo(
    () => tasks.filter((t) => t.status === "running").length,
    [tasks]
  );

  /** 主窗口：托盘图标与生成任务状态同步（短防抖，减少图标闪烁） */
  useEffect(() => {
    if (!isTauri()) return;
    if (getCurrentWindow().label !== "main") return;

    const state = computeTrayGenerationState(tasks);
    const t = window.setTimeout(() => {
      void syncTrayGenerationState(state).catch(() => {});
    }, 200);
    return () => clearTimeout(t);
  }, [tasks]);

  const api = useMemo<GenerationTasksApi>(
    () => ({
      tasks,
      runningCount,
      startGeneration,
      dismissTask,
      clearFinished,
      markConsumed,
    }),
    [tasks, runningCount, startGeneration, dismissTask, clearFinished, markConsumed]
  );

  return (
    <Ctx.Provider value={api}>
      {children}
      <TaskTray />
    </Ctx.Provider>
  );
}

export function useGenerationTasks(): GenerationTasksApi {
  const ctx = useContext(Ctx);
  if (!ctx)
    throw new Error("useGenerationTasks 必须在 GenerationTasksProvider 内使用");
  return ctx;
}

function buildSummary(input: GenerateCardsInput): string {
  const hasImages = (input.imageUrls?.length ?? 0) > 0;
  const hasText = !!input.sourceText && input.sourceText.trim().length > 0;
  const kw = input.selectedKeyword?.trim();

  // 优先用用户给的关键词
  if (kw) return `围绕「${kw}」生成`;

  if (hasImages && hasText) return `图文混合生成`;
  if (hasImages) return `图片生成（${input.imageUrls!.length} 张）`;
  if (hasText) {
    const text = input.sourceText.trim();
    const preview = text.length > 16 ? `${text.slice(0, 16)}…` : text;
    return `从「${preview}」生成`;
  }
  return "生成卡片";
}

// ---------------------------------------------------------------------------
// TaskTray：右下角浮动面板，显示所有"进行中 / 失败（未 dismiss）"的任务。
// 成功任务有 toast 通知 + 宝库可查，不在任务栏常驻，避免遮挡界面。
// ---------------------------------------------------------------------------

function TaskTray() {
  const { tasks, dismissTask, clearFinished } = useGenerationTasks();

  // 展示：进行中 + 失败；成功任务已经被 toast 通知过，不在此常驻。
  const visible = useMemo(
    () =>
      tasks.filter((t) => t.status === "running" || t.status === "failed"),
    [tasks]
  );

  if (visible.length === 0) return null;

  return (
    <div className="pointer-events-none fixed bottom-5 right-5 z-[9998] flex w-80 flex-col gap-2">
      {visible.length > 1 && (
        <div className="pointer-events-auto flex items-center justify-between rounded-lg bg-ink-900/80 px-3 py-1.5 text-[11px] font-medium text-white shadow-card-hover backdrop-blur">
          <span>后台任务 · {visible.length}</span>
          <button
            className="rounded px-1.5 py-0.5 text-white/80 transition hover:bg-white/10 hover:text-white"
            onClick={clearFinished}
          >
            清理已完成
          </button>
        </div>
      )}
      {visible.map((t) => (
        <TaskRow key={t.id} task={t} onDismiss={() => dismissTask(t.id)} />
      ))}
    </div>
  );
}

function TaskRow({
  task,
  onDismiss,
}: {
  task: GenerationTask;
  onDismiss: () => void;
}) {
  const Icon =
    task.status === "running"
      ? Loader2
      : task.status === "failed"
        ? AlertTriangle
        : CheckCircle2;
  const accent =
    task.status === "running"
      ? "text-brand-600"
      : task.status === "failed"
        ? "text-rose-600"
        : "text-emerald-600";

  return (
    <div
      className={clsx(
        "pointer-events-auto flex items-start gap-3 rounded-xl border border-ink-200",
        "bg-white/95 px-4 py-3 shadow-card-hover backdrop-blur animate-fade-in"
      )}
    >
      <Icon
        className={clsx(
          "mt-0.5 h-5 w-5 flex-none",
          accent,
          task.status === "running" && "animate-spin"
        )}
      />
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2 text-sm font-medium text-ink-900">
          <Sparkles className="h-3.5 w-3.5 text-brand-500" />
          <span className="truncate">{task.summary}</span>
        </div>
        <div className="mt-0.5 text-[11px] text-ink-500">
          {task.status === "running" ? "生成中" : "失败"} ·{" "}
          {formatRelative(task.startedAt)}
        </div>
        {task.status === "failed" && task.error && (
          <div className="mt-1 line-clamp-3 text-xs leading-5 text-rose-600">
            {task.error}
          </div>
        )}
        {task.status === "failed" && (
          <div className="mt-2">
            <Link
              to="/generate"
              className="text-[11px] font-medium text-brand-600 underline-offset-2 hover:underline"
            >
              返回生成页重试
            </Link>
          </div>
        )}
      </div>
      {task.status !== "running" && (
        <button
          onClick={onDismiss}
          className="rounded p-1 text-ink-400 hover:bg-ink-100 hover:text-ink-600"
          aria-label="关闭"
        >
          <X className="h-4 w-4" />
        </button>
      )}
    </div>
  );
}

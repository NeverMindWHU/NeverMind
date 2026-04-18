/**
 * 光合日历 · 桌面气泡球（独立 Webview，label === "bubble"）
 *
 * ── 调尺寸（最重要）────────────────────────────────────────────
 * 窗口逻辑像素在 src-tauri/tauri.conf.json → label "bubble" 的 width/height。
 * 窗口需大于球体本身，以便右键菜单在「球」上方展开而不被裁切（见下方 BUBBLE_LOGICAL_PX）。
 * 球体仍为圆形，仅占窗口右下角 BUBBLE_LOGICAL_PX 区域。
 *
 * ── 本文件可改项（简要）────────────────────────────────────────
 * - 球体：`BUBBLE_LOGICAL_PX` 须与 tauri.conf 里球区设计一致。
 * - 主图：object-contain；右键菜单见 BubbleContextMenu。
 * - 右键菜单：Portal 挂 body；定位「豆包式」——菜单右下角对齐球体左上角（fixed + right/bottom）。
 * - 透明穿透：由 Rust 后台线程对球外区域 `set_ignore_cursor_events`（仅 OS 层穿透，CSS 不够）。
 * - 拖动：勿用 `data-tauri-drag-region`（会全程手型且一点击就拖）。短按走 click 打开助手；按住超过阈值再 `startDragging()`（豆包/360 悬浮球式）。
 */
import { invoke, isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  forwardRef,
  useCallback,
  useEffect,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
} from "react";
import { createPortal } from "react-dom";
import {
  type BubbleSkinId,
  readBubbleSkin,
  SKIN_SRC,
  SPIRIT_MENU_ITEMS,
  writeBubbleSkin,
} from "./bubbleSkins";

/** 球体边长（逻辑像素），须与布局中球形容器、`src-tauri` 中 BUBBLE_BALL_LOGICAL_PX 一致 */
const BUBBLE_LOGICAL_PX = 72;

/** 按住超过此时长（毫秒）再触发系统窗口拖动，短按仍为「点击」打开助手 */
const HOLD_MS_BEFORE_WINDOW_DRAG = 280;

type AssistantMsg =
  | { role: "user"; text: string; imageDataUrl?: string }
  | { role: "assistant"; text: string };

export default function BubbleBall() {
  const [menuOpen, setMenuOpen] = useState(false);
  const [assistantOpen, setAssistantOpen] = useState(false);
  const [assistantMessages, setAssistantMessages] = useState<AssistantMsg[]>(
    () => [
      {
        role: "assistant",
        text: "你好，我是手边助手。输入文字或点左下角「+」添加配图，Enter 发送。",
      },
    ]
  );
  const [assistantInput, setAssistantInput] = useState("");
  const [assistantPending, setAssistantPending] = useState<{
    dataUrl: string;
    mime: string;
  } | null>(null);
  const [assistantSending, setAssistantSending] = useState(false);
  const assistantScrollRef = useRef<HTMLDivElement>(null);
  const assistantFileRef = useRef<HTMLInputElement>(null);
  const ballClickTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const holdDragTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pointerDownOnBallRef = useRef(false);
  /** 本次按下是否已触发长按拖动（用于忽略随后的 click，避免误开助手） */
  const startedWindowDragRef = useRef(false);

  const [skinId, setSkinId] = useState<BubbleSkinId>(() => readBubbleSkin());
  const menuPanelRef = useRef<HTMLDivElement>(null);
  const ballButtonRef = useRef<HTMLButtonElement>(null);

  const setSkin = useCallback((id: BubbleSkinId) => {
    setSkinId(id);
    writeBubbleSkin(id);
  }, []);

  /**
   * 小窗内 #root 默认无高度，h-full 会塌成 0；这里把 html/body/#root 拉满，
   * 与外层 h-screen 双保险。仅气泡窗口进程会执行，不影响主窗口。
   */
  useEffect(() => {
    const root = document.getElementById("root");
    document.documentElement.style.background = "transparent";
    document.documentElement.style.height = "100%";
    document.body.style.background = "transparent";
    document.body.style.height = "100%";
    document.body.style.margin = "0";
    document.body.style.overflow = "hidden";
    if (root) {
      root.style.height = "100%";
      root.style.minHeight = "100%";
      root.style.overflow = "hidden";
      root.style.pointerEvents = "none";
    }
    return () => {
      document.documentElement.style.height = "";
      document.documentElement.style.background = "";
      document.body.style.height = "";
      document.body.style.overflow = "";
      document.body.style.background = "";
      if (root) {
        root.style.height = "";
        root.style.minHeight = "";
        root.style.overflow = "";
        root.style.pointerEvents = "";
      }
    };
  }, []);

  const closeMenu = useCallback(() => setMenuOpen(false), []);

  /** 同步 Rust：菜单打开时整窗需接收鼠标；关闭后恢复「仅球体可点、其余穿透」 */
  useEffect(() => {
    if (!isTauri()) return;
    void invoke("set_bubble_menu_open", { open: menuOpen });
  }, [menuOpen]);

  /** 助手面板打开时整窗可交互（与菜单一致） */
  useEffect(() => {
    if (!isTauri()) return;
    void invoke("set_bubble_assistant_open", { open: assistantOpen });
  }, [assistantOpen]);

  /** 打开助手时收起右键菜单 */
  useEffect(() => {
    if (assistantOpen && menuOpen) setMenuOpen(false);
  }, [assistantOpen, menuOpen]);

  useEffect(() => {
    if (!assistantOpen) return;
    const el = assistantScrollRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [assistantOpen, assistantMessages, assistantSending]);

  /** 菜单打开时：点外部关闭、Esc 关闭；失焦关闭（透明区穿透后窗口易失焦） */
  useEffect(() => {
    if (!menuOpen) return;
    const onMouseDown = (e: MouseEvent) => {
      if (menuPanelRef.current?.contains(e.target as Node)) return;
      if (ballButtonRef.current?.contains(e.target as Node)) return;
      closeMenu();
    };
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") closeMenu();
    };
    const onBlur = () => closeMenu();
    document.addEventListener("mousedown", onMouseDown, true);
    document.addEventListener("keydown", onKeyDown);
    window.addEventListener("blur", onBlur);
    return () => {
      document.removeEventListener("mousedown", onMouseDown, true);
      document.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("blur", onBlur);
    };
  }, [menuOpen, closeMenu]);

  /** 助手打开时 Esc 关闭 */
  useEffect(() => {
    if (!assistantOpen) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") setAssistantOpen(false);
    };
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [assistantOpen]);

  const openMain = useCallback(async () => {
    if (!isTauri()) return;
    try {
      await invoke("focus_main_window");
    } catch {
      /* ignore */
    }
  }, []);

  const hideBubble = useCallback(async () => {
    if (!isTauri()) return;
    try {
      await getCurrentWindow().hide();
    } catch {
      /* ignore */
    }
  }, []);

  const onOpenMain = useCallback(() => {
    closeMenu();
    void openMain();
  }, [closeMenu, openMain]);

  const onHideBubble = useCallback(() => {
    closeMenu();
    void hideBubble();
  }, [closeMenu, hideBubble]);

  const onSelectSkin = useCallback(
    (id: BubbleSkinId) => {
      setSkin(id);
      closeMenu();
    },
    [closeMenu, setSkin]
  );

  const onRestoreInitialSkin = useCallback(() => {
    onSelectSkin("initial");
  }, [onSelectSkin]);

  const onContextMenu = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setAssistantOpen(false);
    setMenuOpen(true);
  }, []);

  const closeAssistant = useCallback(() => {
    setAssistantOpen(false);
  }, []);

  const clearHoldDragTimer = useCallback(() => {
    if (holdDragTimerRef.current) {
      clearTimeout(holdDragTimerRef.current);
      holdDragTimerRef.current = null;
    }
  }, []);

  const onBallPointerDown = useCallback(
    (e: React.PointerEvent<HTMLButtonElement>) => {
      if (e.button !== 0) return;
      startedWindowDragRef.current = false;
      pointerDownOnBallRef.current = true;
      clearHoldDragTimer();
      try {
        e.currentTarget.setPointerCapture(e.pointerId);
      } catch {
        /* ignore */
      }
      holdDragTimerRef.current = window.setTimeout(() => {
        holdDragTimerRef.current = null;
        if (!pointerDownOnBallRef.current) return;
        startedWindowDragRef.current = true;
        void getCurrentWindow()
          .startDragging()
          .catch(() => {
            /* ignore */
          });
      }, HOLD_MS_BEFORE_WINDOW_DRAG);
    },
    [clearHoldDragTimer]
  );

  const onBallPointerUpOrCancel = useCallback(
    (e: React.PointerEvent<HTMLButtonElement>) => {
      pointerDownOnBallRef.current = false;
      clearHoldDragTimer();
      try {
        if (e.currentTarget.hasPointerCapture(e.pointerId)) {
          e.currentTarget.releasePointerCapture(e.pointerId);
        }
      } catch {
        /* ignore */
      }
    },
    [clearHoldDragTimer]
  );

  const onBallClick = useCallback(() => {
    if (startedWindowDragRef.current) {
      startedWindowDragRef.current = false;
      return;
    }
    if (ballClickTimerRef.current) clearTimeout(ballClickTimerRef.current);
    ballClickTimerRef.current = setTimeout(() => {
      ballClickTimerRef.current = null;
      setAssistantOpen((o) => !o);
    }, 260);
  }, []);

  const onBallDoubleClick = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      if (ballClickTimerRef.current) {
        clearTimeout(ballClickTimerRef.current);
        ballClickTimerRef.current = null;
      }
      setAssistantOpen(false);
      void openMain();
    },
    [openMain]
  );

  const clearAssistantChat = useCallback(() => {
    setAssistantMessages([
      {
        role: "assistant",
        text: "已开始新对话。可继续输入文字或添加图片。",
      },
    ]);
    setAssistantInput("");
    setAssistantPending(null);
  }, []);

  const showAssistantInfo = useCallback(() => {
    window.alert(
      "手边助手与主界面共用方舟模型（需在 .env 配置 ARK_API_KEY）。支持文字与一张配图；Enter 发送，Shift+Enter 换行。"
    );
  }, []);

  const readFileAsDataUrl = (file: File) =>
    new Promise<{ dataUrl: string; mime: string }>((resolve, reject) => {
      const r = new FileReader();
      r.onload = () => {
        const dataUrl = typeof r.result === "string" ? r.result : "";
        resolve({ dataUrl, mime: file.type || "image/png" });
      };
      r.onerror = () => reject(new Error("读取文件失败"));
      r.readAsDataURL(file);
    });

  const pickAssistantImage = useCallback(() => {
    assistantFileRef.current?.click();
  }, []);

  const onAssistantFileChange = useCallback(
    async (e: React.ChangeEvent<HTMLInputElement>) => {
      const f = e.target.files?.[0];
      e.target.value = "";
      if (!f || !f.type.startsWith("image/")) return;
      try {
        const { dataUrl, mime } = await readFileAsDataUrl(f);
        setAssistantPending({ dataUrl, mime });
      } catch {
        /* ignore */
      }
    },
    []
  );

  const sendAssistant = useCallback(async () => {
    const t = assistantInput.trim();
    if (!t && !assistantPending) return;
    if (!isTauri()) {
      setAssistantMessages((m) => [
        ...m,
        {
          role: "assistant",
          text: "（仅在桌面应用中可调用模型）",
        },
      ]);
      return;
    }
    setAssistantSending(true);
    const userMsg: AssistantMsg = {
      role: "user",
      text: t,
      imageDataUrl: assistantPending?.dataUrl,
    };
    setAssistantMessages((m) => [...m, userMsg]);
    setAssistantInput("");
    const pending = assistantPending;
    setAssistantPending(null);

    let imageMime: string | undefined;
    let imageB64: string | undefined;
    if (pending) {
      const comma = pending.dataUrl.indexOf(",");
      if (comma >= 0) {
        imageB64 = pending.dataUrl.slice(comma + 1);
        imageMime = pending.mime;
      }
    }

    try {
      const reply = await invoke<string>("bubble_assistant_chat", {
        text: t,
        imageMime: imageMime ?? null,
        imageBase64: imageB64 ?? null,
      });
      setAssistantMessages((m) => [...m, { role: "assistant", text: reply }]);
    } catch (err) {
      const msg =
        typeof err === "string"
          ? err
          : err instanceof Error
            ? err.message
            : "发送失败";
      setAssistantMessages((m) => [
        ...m,
        {
          role: "assistant",
          text: `（未能完成回复）${msg}`,
        },
      ]);
    } finally {
      setAssistantSending(false);
    }
  }, [assistantInput, assistantPending]);

  const onAssistantKeyDown = useCallback(
    (e: ReactKeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        void sendAssistant();
      }
    },
    [sendAssistant]
  );

  return (
    <>
      {/* 视口 = 窗口客户区；扩大后的透明区不接收事件，仅球与菜单可点 */}
      <div
        className="pointer-events-none relative h-full w-full min-h-0 select-none overflow-visible"
        style={{ background: "transparent", boxShadow: "none" }}
      >
        {assistantOpen ? (
          <button
            type="button"
            className="pointer-events-auto absolute inset-0 z-[40] cursor-default border-0 bg-transparent p-0"
            aria-label="关闭助手"
            onClick={closeAssistant}
          />
        ) : null}

        {assistantOpen ? (
          <aside
            className="pointer-events-auto absolute z-[50] flex flex-col overflow-hidden rounded-[28px] border border-neutral-200/95 bg-white text-neutral-800 shadow-[0_24px_48px_-12px_rgba(15,23,42,0.18)]"
            style={{
              top: 10,
              left: 8,
              right: 8,
              bottom: BUBBLE_LOGICAL_PX + 8,
            }}
            role="dialog"
            aria-label="手边 AI 助手"
            onClick={(e) => e.stopPropagation()}
          >
            <BubbleAssistantChrome
              onNewChat={clearAssistantChat}
              onOpenMain={onOpenMain}
              onInfo={showAssistantInfo}
            />
            <div
              ref={assistantScrollRef}
              className="min-h-0 flex-1 space-y-2 overflow-y-auto overflow-x-hidden px-3 py-2 text-[13px] leading-relaxed text-neutral-600 [scrollbar-width:thin]"
            >
              {assistantMessages.map((m, i) =>
                m.role === "user" ? (
                  <div key={i} className="flex flex-col items-end gap-1 pl-4">
                    {m.imageDataUrl ? (
                      <img
                        src={m.imageDataUrl}
                        alt=""
                        className="max-h-24 max-w-full rounded-lg border border-neutral-200 object-contain"
                      />
                    ) : null}
                    {m.text ? (
                      <div className="rounded-2xl rounded-br-md bg-neutral-100 px-3 py-2 text-left text-neutral-800">
                        {m.text}
                      </div>
                    ) : null}
                  </div>
                ) : (
                  <div key={i} className="pr-4 text-left">
                    {m.text}
                  </div>
                )
              )}
              {assistantSending ? (
                <div className="text-neutral-400">正在思考…</div>
              ) : null}
            </div>
            <div className="shrink-0 bg-white px-3 pb-3 pt-1">
              <div className="rounded-[18px] border border-neutral-200/90 bg-white p-3 shadow-[0_1px_3px_rgba(15,23,42,0.06)]">
                {assistantPending ? (
                  <div className="mb-3 flex items-center gap-2 rounded-xl bg-neutral-50/90 px-2 py-2">
                    <img
                      src={assistantPending.dataUrl}
                      alt=""
                      className="h-11 w-11 rounded-lg border border-neutral-200 object-cover"
                    />
                    <button
                      type="button"
                      className="text-[11px] text-neutral-500 underline decoration-neutral-300 hover:text-neutral-800"
                      onClick={() => setAssistantPending(null)}
                    >
                      移除图片
                    </button>
                  </div>
                ) : null}
                <textarea
                  value={assistantInput}
                  onChange={(e) => setAssistantInput(e.target.value)}
                  onKeyDown={onAssistantKeyDown}
                  placeholder='发消息或输入 "/" 选择技能'
                  disabled={assistantSending}
                  rows={2}
                  className="min-h-[44px] w-full resize-none border-0 bg-transparent px-0.5 py-0 text-[13px] text-neutral-800 outline-none ring-0 placeholder:text-neutral-400 focus:ring-0 disabled:opacity-60"
                />
                <input
                  ref={assistantFileRef}
                  type="file"
                  accept="image/*"
                  className="hidden"
                  onChange={onAssistantFileChange}
                />
                <div className="mt-2 flex items-center gap-1">
                  <div className="flex min-w-0 flex-1 items-center gap-0.5">
                    <button
                      type="button"
                      className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg border-0 bg-transparent text-neutral-500 hover:bg-neutral-100 hover:text-neutral-800 disabled:opacity-50"
                      aria-label="添加图片或附件"
                      onClick={pickAssistantImage}
                      disabled={assistantSending}
                    >
                      <IconPlusLarge />
                    </button>
                    <div
                      className="mx-0.5 h-4 w-px shrink-0 bg-neutral-200"
                      aria-hidden
                    />
                    <button
                      type="button"
                      className="flex h-8 shrink-0 items-center gap-0.5 rounded-lg px-1.5 text-[12px] text-neutral-600 hover:bg-neutral-100"
                      title="快捷操作（即将推出）"
                    >
                      <IconBoltSmall />
                      <span>快速</span>
                      <IconChevronRightTiny />
                    </button>
                    <button
                      type="button"
                      className="flex h-8 shrink-0 items-center gap-1 rounded-lg px-1.5 text-[12px] text-neutral-400"
                      disabled
                      title="编程模式（即将推出）"
                    >
                      <IconCodeSmall />
                      <span>编程</span>
                    </button>
                    <button
                      type="button"
                      className="flex h-8 shrink-0 items-center gap-1 rounded-lg px-1.5 text-[12px] text-neutral-600 hover:bg-neutral-100"
                      title="更多说明"
                      onClick={showAssistantInfo}
                    >
                      <IconGridSmall />
                      <span>更多</span>
                    </button>
                  </div>
                  <button
                    type="button"
                    className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full border border-neutral-200 bg-white text-neutral-500 hover:bg-neutral-50 disabled:opacity-50"
                    aria-label="语音输入"
                    disabled
                    title="语音输入（即将推出）"
                  >
                    <IconMic />
                  </button>
                </div>
              </div>
              <p className="mt-1.5 px-1 text-center text-[10px] text-neutral-400">
                Enter 发送 · Shift+Enter 换行
              </p>
            </div>
          </aside>
        ) : null}

        <button
          ref={ballButtonRef}
          type="button"
          className="pointer-events-auto absolute bottom-0 right-0 z-[60] m-0 flex items-center justify-center rounded-full border-0 bg-transparent p-0 cursor-default shadow-none select-none touch-none"
          style={{
            width: BUBBLE_LOGICAL_PX,
            height: BUBBLE_LOGICAL_PX,
            boxShadow: "none",
          }}
          onPointerDown={onBallPointerDown}
          onPointerUp={onBallPointerUpOrCancel}
          onPointerCancel={onBallPointerUpOrCancel}
          onClick={onBallClick}
          onDoubleClick={onBallDoubleClick}
          onContextMenu={onContextMenu}
          title="短按：开关手边助手；按住不放再移动：拖动窗口；双击：打开主窗口；右键：菜单"
          aria-label="光合日历气泡球，短按打开助手，长按拖动，双击打开主窗口"
          aria-haspopup="dialog"
          aria-expanded={assistantOpen}
        >
          <img
            src={SKIN_SRC[skinId]}
            alt=""
            className="pointer-events-none h-full w-full rounded-full object-contain shadow-none"
            style={{ filter: "none" }}
            draggable={false}
          />
        </button>
      </div>

      {menuOpen
        ? createPortal(
            <BubbleContextMenu
              ref={menuPanelRef}
              bubblePx={BUBBLE_LOGICAL_PX}
              skinId={skinId}
              onOpenMain={onOpenMain}
              onHideBubble={onHideBubble}
              onSelectSkin={onSelectSkin}
              onRestoreInitialSkin={onRestoreInitialSkin}
            />,
            document.body
          )
        : null}
    </>
  );
}

function BubbleAssistantChrome({
  onNewChat,
  onOpenMain,
  onInfo,
}: {
  onNewChat: () => void;
  onOpenMain: () => void;
  onInfo: () => void;
}) {
  const [pinned, setPinned] = useState(true);

  useEffect(() => {
    if (!isTauri()) return;
    void getCurrentWindow()
      .isAlwaysOnTop()
      .then(setPinned)
      .catch(() => {});
  }, []);

  const togglePin = useCallback(async () => {
    if (!isTauri()) return;
    try {
      const w = getCurrentWindow();
      const next = !pinned;
      await w.setAlwaysOnTop(next);
      setPinned(next);
    } catch {
      /* ignore */
    }
  }, [pinned]);

  const minimizeWindow = useCallback(() => {
    if (!isTauri()) return;
    void getCurrentWindow().minimize().catch(() => {});
  }, []);

  const btnClass =
    "flex h-8 w-8 shrink-0 items-center justify-center rounded-lg border-0 bg-transparent text-neutral-500 hover:bg-neutral-100 hover:text-neutral-800";

  return (
    <header className="flex shrink-0 items-center justify-between gap-1 border-b border-neutral-100 px-2 py-2">
      <div className="flex items-center gap-0.5">
        <button
          type="button"
          className={btnClass}
          title="新对话"
          aria-label="新对话"
          onClick={onNewChat}
        >
          <IconEditCompose />
        </button>
        <button
          type="button"
          className={btnClass}
          title="打开主窗口"
          aria-label="打开主窗口"
          onClick={onOpenMain}
        >
          <IconPhone />
        </button>
        <button
          type="button"
          className={btnClass}
          title="说明"
          aria-label="说明"
          onClick={onInfo}
        >
          <IconChatOutline />
        </button>
      </div>
      <div className="flex items-center gap-0.5">
        <button
          type="button"
          className={`${btnClass} ${pinned ? "text-neutral-700" : "text-neutral-400"}`}
          title={pinned ? "已置顶" : "置顶"}
          aria-label={pinned ? "取消置顶" : "置顶"}
          aria-pressed={pinned}
          onClick={() => void togglePin()}
        >
          <IconPin />
        </button>
        <button
          type="button"
          className={btnClass}
          title="最小化窗口"
          aria-label="最小化窗口"
          onClick={minimizeWindow}
        >
          <IconMinimizeLine />
        </button>
      </div>
    </header>
  );
}

function IconEditCompose() {
  return (
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" aria-hidden>
      <rect x="4" y="4" width="12" height="12" rx="2" strokeLinejoin="round" />
      <path d="M14 4l2 2-7 7H8v-3l7-7z" strokeLinejoin="round" />
    </svg>
  );
}

function IconPhone() {
  return (
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" aria-hidden>
      <path
        d="M6.5 4.5h3l1.2 3.2a1 1 0 0 1-.24 1.03l-1.6 1.6a12.1 12.1 0 0 0 5.37 5.37l1.6-1.6a1 1 0 0 1 1.03-.24L19.5 15v3a2 2 0 0 1-2.2 1.98 18 18 0 0 1-15.28-15.28A2 2 0 0 1 5.5 2h1z"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function IconChatOutline() {
  return (
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" aria-hidden>
      <path
        d="M8 17H5a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v7a2 2 0 0 1-2 2h-6l-5 4v-4z"
        strokeLinejoin="round"
      />
    </svg>
  );
}

function IconPin() {
  return (
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" aria-hidden>
      <path d="M12 21s7-4.5 7-11a7 7 0 1 0-14 0c0 6.5 7 11 7 11z" strokeLinejoin="round" />
      <circle cx="12" cy="10" r="2.5" />
    </svg>
  );
}

function IconMinimizeLine() {
  return (
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
      <path d="M5 12h14" strokeLinecap="round" />
    </svg>
  );
}

function IconPlusLarge() {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
      <path d="M12 5v14M5 12h14" strokeLinecap="round" />
    </svg>
  );
}

function IconBoltSmall() {
  return (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
      <path d="M13 2L3 14h8l-1 8 10-12h-8l1-8z" strokeLinejoin="round" />
    </svg>
  );
}

function IconChevronRightTiny() {
  return (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden>
      <path d="M9 6l6 6-6 6" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

function IconCodeSmall() {
  return (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" aria-hidden>
      <path d="M8 7l-3 5 3 5M16 7l3 5-3 5" strokeLinecap="round" strokeLinejoin="round" />
      <path d="M14 5l-4 14" strokeLinecap="round" />
    </svg>
  );
}

function IconGridSmall() {
  return (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" aria-hidden>
      <rect x="3" y="3" width="7" height="7" rx="1" />
      <rect x="14" y="3" width="7" height="7" rx="1" />
      <rect x="3" y="14" width="7" height="7" rx="1" />
      <rect x="14" y="14" width="7" height="7" rx="1" />
    </svg>
  );
}

function IconMic() {
  return (
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" aria-hidden>
      <rect x="9" y="2" width="6" height="11" rx="3" />
      <path d="M5 10v1a7 7 0 0 0 14 0v-1M12 19v3M8 22h8" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

type BubbleContextMenuProps = {
  bubblePx: number;
  skinId: BubbleSkinId;
  onOpenMain: () => void;
  onHideBubble: () => void;
  onSelectSkin: (id: BubbleSkinId) => void;
  onRestoreInitialSkin: () => void;
};

/**
 * 挂到 document.body。定位：菜单「右下角」对齐球体「左上角」（向左上展开，贴屏幕角时不被裁切）。
 * right/bottom 与球同偏移 = 球占右下角 bubblePx×bubblePx。
 */
const BubbleContextMenu = forwardRef<HTMLDivElement, BubbleContextMenuProps>(
  function BubbleContextMenu(
    {
      bubblePx,
      skinId,
      onOpenMain,
      onHideBubble,
      onSelectSkin,
      onRestoreInitialSkin,
    },
    ref
  ) {
    return (
      <div
        ref={ref}
        role="menu"
        aria-label="气泡球功能"
        className="pointer-events-auto fixed z-[9999] max-h-[min(340px,calc(100vh-60px))] min-w-[148px] overflow-y-auto overflow-x-hidden rounded-lg border border-slate-200/90 bg-white/95 py-1 text-left text-slate-800 shadow-lg shadow-slate-900/15 backdrop-blur-sm [scrollbar-width:none] [-ms-overflow-style:none] [&::-webkit-scrollbar]:hidden"
        style={{
          right: bubblePx,
          bottom: bubblePx,
        }}
      >
        <div className="border-b border-slate-100 px-3 py-1.5 text-[11px] font-medium text-slate-500">
          光合日历
        </div>
        <button
          type="button"
          role="menuitem"
          className="block w-full px-3 py-2 text-left text-[13px] hover:bg-emerald-50 active:bg-emerald-100"
          onClick={onOpenMain}
        >
          打开主界面
        </button>
        <button
          type="button"
          role="menuitem"
          className="block w-full px-3 py-2 text-left text-[13px] hover:bg-emerald-50 active:bg-emerald-100"
          onClick={onHideBubble}
        >
          隐藏气泡球
        </button>
        <div
          className="border-t border-slate-100 px-3 py-1.5 text-[11px] font-medium text-slate-500"
          role="presentation"
        >
          切换外观
        </div>
        {SPIRIT_MENU_ITEMS.map(({ id, label }) => {
          const active = skinId === id;
          return (
            <button
              key={id}
              type="button"
              role="menuitemradio"
              aria-checked={active}
              className={`block w-full px-3 py-2 text-left text-[13px] hover:bg-emerald-50 active:bg-emerald-100 ${
                active ? "bg-emerald-50/80 font-medium text-emerald-900" : ""
              }`}
              onClick={() => onSelectSkin(id)}
            >
              {label}
            </button>
          );
        })}
        <button
          type="button"
          role="menuitem"
          className="block w-full border-t border-slate-100 px-3 py-1.5 text-left text-[11px] text-slate-500 hover:bg-slate-50 hover:text-slate-700"
          onClick={onRestoreInitialSkin}
        >
          恢复初始外观
        </button>
      </div>
    );
  }
);

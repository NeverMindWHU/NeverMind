import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import clsx from "clsx";
import { CheckCircle2, AlertTriangle, Info, X } from "lucide-react";

type ToastKind = "success" | "error" | "info";

interface ToastItem {
  id: number;
  kind: ToastKind;
  title: string;
  description?: string;
}

interface ToastApi {
  success: (title: string, description?: string) => void;
  error: (title: string, description?: string) => void;
  info: (title: string, description?: string) => void;
}

const ToastCtx = createContext<ToastApi | null>(null);

export function ToastProvider({ children }: { children: ReactNode }) {
  const [items, setItems] = useState<ToastItem[]>([]);
  const nextId = useRef(1);

  const push = useCallback(
    (kind: ToastKind, title: string, description?: string) => {
      const id = nextId.current++;
      setItems((prev) => [...prev, { id, kind, title, description }]);
      window.setTimeout(() => {
        setItems((prev) => prev.filter((x) => x.id !== id));
      }, 3800);
    },
    []
  );

  const api = useMemo<ToastApi>(
    () => ({
      success: (t, d) => push("success", t, d),
      error: (t, d) => push("error", t, d),
      info: (t, d) => push("info", t, d),
    }),
    [push]
  );

  const close = useCallback((id: number) => {
    setItems((prev) => prev.filter((x) => x.id !== id));
  }, []);

  return (
    <ToastCtx.Provider value={api}>
      {children}
      <div className="pointer-events-none fixed right-5 top-5 z-[9999] flex w-80 flex-col gap-2">
        {items.map((t) => (
          <ToastView key={t.id} item={t} onClose={() => close(t.id)} />
        ))}
      </div>
    </ToastCtx.Provider>
  );
}

export function useToast(): ToastApi {
  const ctx = useContext(ToastCtx);
  if (!ctx) throw new Error("useToast 必须在 ToastProvider 内使用");
  return ctx;
}

function ToastView({ item, onClose }: { item: ToastItem; onClose: () => void }) {
  const Icon =
    item.kind === "success" ? CheckCircle2 : item.kind === "error" ? AlertTriangle : Info;
  const accent =
    item.kind === "success"
      ? "text-emerald-600"
      : item.kind === "error"
        ? "text-rose-600"
        : "text-brand-600";
  return (
    <div
      className={clsx(
        "pointer-events-auto flex items-start gap-3 rounded-xl border border-ink-200",
        "bg-white/95 px-4 py-3 shadow-card-hover backdrop-blur animate-fade-in"
      )}
    >
      <Icon className={clsx("mt-0.5 h-5 w-5 flex-none", accent)} />
      <div className="flex-1">
        <div className="text-sm font-medium text-ink-900">{item.title}</div>
        {item.description && (
          <div className="mt-1 text-xs leading-5 text-ink-600">{item.description}</div>
        )}
      </div>
      <button
        onClick={onClose}
        className="rounded p-1 text-ink-400 hover:bg-ink-100 hover:text-ink-600"
        aria-label="关闭"
      >
        <X className="h-4 w-4" />
      </button>
    </div>
  );
}

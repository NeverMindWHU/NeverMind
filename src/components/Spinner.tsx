import clsx from "clsx";
import { Loader2 } from "lucide-react";

export function Spinner({ className, label }: { className?: string; label?: string }) {
  return (
    <div className={clsx("flex items-center gap-2 text-ink-500", className)} role="status">
      <Loader2 className="h-4 w-4 animate-spin" aria-hidden />
      {label && <span className="text-sm">{label}</span>}
    </div>
  );
}

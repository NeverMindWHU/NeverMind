import type { ReactNode } from "react";
import clsx from "clsx";

export function EmptyState({
  icon,
  title,
  description,
  action,
  className,
}: {
  icon?: ReactNode;
  title: string;
  description?: ReactNode;
  action?: ReactNode;
  className?: string;
}) {
  return (
    <div
      className={clsx(
        "flex flex-col items-center justify-center gap-3 rounded-xl border border-dashed border-ink-200 bg-white px-6 py-14 text-center",
        className
      )}
    >
      {icon && <div className="text-ink-400">{icon}</div>}
      <div>
        <div className="text-sm font-medium text-ink-800">{title}</div>
        {description && (
          <div className="mt-1 max-w-md text-xs leading-5 text-ink-500">
            {description}
          </div>
        )}
      </div>
      {action && <div className="mt-1">{action}</div>}
    </div>
  );
}

import clsx from "clsx";
import type { InputHTMLAttributes, ReactNode, SelectHTMLAttributes, TextareaHTMLAttributes } from "react";

export function Panel({
  title,
  description,
  actions,
  children,
  className,
  contentClassName,
}: {
  title?: ReactNode;
  description?: ReactNode;
  actions?: ReactNode;
  children: ReactNode;
  className?: string;
  contentClassName?: string;
}) {
  return (
    <section
      className={clsx(
        "rounded-2xl border border-ink-200 bg-white shadow-card",
        className
      )}
    >
      {(title || actions) && (
        <header className="flex items-start justify-between gap-4 border-b border-ink-100 px-5 py-4">
          <div className="min-w-0">
            {title && (
              <h2 className="truncate text-base font-semibold text-ink-900">{title}</h2>
            )}
            {description && (
              <p className="mt-0.5 text-xs text-ink-500">{description}</p>
            )}
          </div>
          {actions && <div className="flex flex-none items-center gap-2">{actions}</div>}
        </header>
      )}
      <div className={clsx("px-5 py-5", contentClassName)}>{children}</div>
    </section>
  );
}

export function StatTile({
  label,
  value,
  hint,
  icon,
}: {
  label: string;
  value: ReactNode;
  hint?: ReactNode;
  icon?: ReactNode;
}) {
  return (
    <div className="rounded-xl border border-ink-200 bg-white px-4 py-3 shadow-card">
      <div className="flex items-center justify-between gap-2">
        <span className="text-xs text-ink-500">{label}</span>
        {icon && <span className="text-ink-400">{icon}</span>}
      </div>
      <div className="mt-1 text-2xl font-semibold text-ink-900">{value}</div>
      {hint && <div className="mt-1 text-xs text-ink-500">{hint}</div>}
    </div>
  );
}

export function FieldLabel({
  children,
  required,
  className,
}: {
  children: ReactNode;
  required?: boolean;
  className?: string;
}) {
  return (
    <label
      className={clsx("mb-1 block text-xs font-medium text-ink-600", className)}
    >
      {children}
      {required && <span className="ml-1 text-rose-500">*</span>}
    </label>
  );
}

export function Input(props: InputHTMLAttributes<HTMLInputElement>) {
  const { className, ...rest } = props;
  return (
    <input
      className={clsx(
        "block h-10 w-full rounded-lg border border-ink-200 bg-white px-3 text-sm text-ink-900",
        "placeholder:text-ink-400 focus:border-brand-400 focus:outline-none focus:ring-2 focus:ring-brand-100",
        "disabled:cursor-not-allowed disabled:bg-ink-50 disabled:text-ink-400",
        className
      )}
      {...rest}
    />
  );
}

export function Textarea(props: TextareaHTMLAttributes<HTMLTextAreaElement>) {
  const { className, ...rest } = props;
  return (
    <textarea
      className={clsx(
        "block w-full rounded-lg border border-ink-200 bg-white px-3 py-2 text-sm leading-6 text-ink-900",
        "placeholder:text-ink-400 focus:border-brand-400 focus:outline-none focus:ring-2 focus:ring-brand-100",
        "disabled:cursor-not-allowed disabled:bg-ink-50",
        className
      )}
      {...rest}
    />
  );
}

export function Select(
  props: SelectHTMLAttributes<HTMLSelectElement> & {
    options: Array<{ value: string; label: string }>;
  }
) {
  const { className, options, ...rest } = props;
  return (
    <select
      className={clsx(
        "block h-10 w-full rounded-lg border border-ink-200 bg-white px-3 text-sm text-ink-900",
        "focus:border-brand-400 focus:outline-none focus:ring-2 focus:ring-brand-100",
        "disabled:cursor-not-allowed disabled:bg-ink-50 disabled:text-ink-400",
        className
      )}
      {...rest}
    >
      {options.map((o) => (
        <option key={o.value} value={o.value}>
          {o.label}
        </option>
      ))}
    </select>
  );
}

export function Tag({ children, tone = "default" }: { children: ReactNode; tone?: "default" | "brand" | "success" | "warn" }) {
  const cls =
    tone === "brand"
      ? "bg-brand-50 text-brand-700 border-brand-200"
      : tone === "success"
        ? "bg-emerald-50 text-emerald-700 border-emerald-200"
        : tone === "warn"
          ? "bg-amber-50 text-amber-700 border-amber-200"
          : "bg-ink-100 text-ink-700 border-ink-200";
  return (
    <span
      className={clsx(
        "inline-flex items-center rounded-full border px-2 py-0.5 text-xs leading-5",
        cls
      )}
    >
      {children}
    </span>
  );
}

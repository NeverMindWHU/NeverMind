import clsx from "clsx";
import { Loader2 } from "lucide-react";
import { forwardRef, type ButtonHTMLAttributes, type ReactNode } from "react";

type Variant = "primary" | "secondary" | "ghost" | "danger" | "success";
type Size = "sm" | "md" | "lg";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
  loading?: boolean;
  leftIcon?: ReactNode;
  rightIcon?: ReactNode;
  fullWidth?: boolean;
}

const variantClass: Record<Variant, string> = {
  primary:
    "bg-brand-600 text-white hover:bg-brand-700 focus-visible:ring-brand-300 disabled:bg-brand-300",
  secondary:
    "bg-white text-ink-800 border border-ink-200 hover:bg-ink-50 focus-visible:ring-ink-200 disabled:text-ink-400",
  ghost:
    "bg-transparent text-ink-700 hover:bg-ink-100 focus-visible:ring-ink-200 disabled:text-ink-400",
  danger:
    "bg-rose-600 text-white hover:bg-rose-700 focus-visible:ring-rose-300 disabled:bg-rose-300",
  success:
    "bg-emerald-600 text-white hover:bg-emerald-700 focus-visible:ring-emerald-300 disabled:bg-emerald-300",
};

const sizeClass: Record<Size, string> = {
  sm: "h-8 px-3 text-xs gap-1.5",
  md: "h-10 px-4 text-sm gap-2",
  lg: "h-12 px-6 text-base gap-2",
};

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(function Button(
  {
    variant = "primary",
    size = "md",
    loading,
    disabled,
    leftIcon,
    rightIcon,
    fullWidth,
    className,
    children,
    ...rest
  },
  ref
) {
  return (
    <button
      ref={ref}
      disabled={disabled || loading}
      className={clsx(
        "inline-flex select-none items-center justify-center rounded-lg font-medium transition",
        "focus:outline-none focus-visible:ring-2 focus-visible:ring-offset-1",
        "disabled:cursor-not-allowed",
        variantClass[variant],
        sizeClass[size],
        fullWidth && "w-full",
        className
      )}
      {...rest}
    >
      {loading ? (
        <Loader2 className="h-4 w-4 animate-spin" aria-hidden />
      ) : (
        leftIcon && <span className="flex-none">{leftIcon}</span>
      )}
      <span className="truncate">{children}</span>
      {rightIcon && !loading && <span className="flex-none">{rightIcon}</span>}
    </button>
  );
});

import type { ComponentPropsWithoutRef, ReactNode } from "react";
import { cn } from "@/lib/cn";

type ButtonVariant = "primary" | "secondary" | "ghost" | "danger";
type ButtonSize = "sm" | "md" | "icon";

function buttonVariantClassName(variant: ButtonVariant) {
  switch (variant) {
    case "primary":
      return "border-transparent bg-zinc-950 text-white shadow-sm hover:bg-zinc-800 focus-visible:ring-zinc-950/20";
    case "ghost":
      return "border-transparent bg-transparent text-zinc-700 hover:bg-zinc-100 hover:text-zinc-950 focus-visible:ring-zinc-950/10";
    case "danger":
      return "border-red-700 bg-red-700 text-white shadow-sm hover:bg-red-800 focus-visible:ring-red-700/20";
    case "secondary":
      return "border-zinc-300 bg-white text-zinc-800 shadow-sm hover:border-zinc-400 hover:bg-zinc-50 focus-visible:ring-zinc-950/10";
  }
}

function buttonSizeClassName(size: ButtonSize) {
  switch (size) {
    case "sm":
      return "h-8 px-2.5 text-xs";
    case "icon":
      return "size-9 p-0";
    case "md":
      return "h-9 px-3 text-sm";
  }
}

export function buttonClassName({
  className,
  size = "md",
  variant = "secondary",
}: {
  className?: string;
  size?: ButtonSize;
  variant?: ButtonVariant;
} = {}) {
  return cn(
    "inline-flex shrink-0 items-center justify-center gap-2 rounded-md border font-medium transition-colors outline-none focus-visible:ring-4 disabled:cursor-not-allowed disabled:opacity-55",
    buttonVariantClassName(variant),
    buttonSizeClassName(size),
    className
  );
}

interface ButtonProps extends ComponentPropsWithoutRef<"button"> {
  size?: ButtonSize;
  variant?: ButtonVariant;
}

export function Button({ className, size, type = "button", variant, ...props }: ButtonProps) {
  return (
    <button className={buttonClassName({ className, size, variant })} type={type} {...props} />
  );
}

export function Input({ className, ...props }: ComponentPropsWithoutRef<"input">) {
  return (
    <input
      className={cn(
        "h-9 w-full rounded-md border border-zinc-300 bg-white px-3 text-sm text-zinc-950 shadow-xs transition-colors outline-none placeholder:text-zinc-400 focus:border-zinc-950 focus:ring-4 focus:ring-zinc-950/10 disabled:cursor-not-allowed disabled:bg-zinc-100 disabled:text-zinc-500",
        className
      )}
      {...props}
    />
  );
}

export function Textarea({ className, ...props }: ComponentPropsWithoutRef<"textarea">) {
  return (
    <textarea
      className={cn(
        "w-full rounded-md border border-zinc-300 bg-white px-3 py-2 text-sm text-zinc-950 shadow-xs transition-colors outline-none placeholder:text-zinc-400 focus:border-zinc-950 focus:ring-4 focus:ring-zinc-950/10 disabled:cursor-not-allowed disabled:bg-zinc-100 disabled:text-zinc-500",
        className
      )}
      {...props}
    />
  );
}

type BadgeTone = "neutral" | "success" | "warning" | "danger" | "info";

function badgeToneClassName(tone: BadgeTone) {
  switch (tone) {
    case "success":
      return "border-emerald-200 bg-emerald-50 text-emerald-800";
    case "warning":
      return "border-amber-200 bg-amber-50 text-amber-800";
    case "danger":
      return "border-red-200 bg-red-50 text-red-800";
    case "info":
      return "border-sky-200 bg-sky-50 text-sky-800";
    case "neutral":
      return "border-zinc-200 bg-zinc-100 text-zinc-700";
  }
}

export function Badge({
  children,
  className,
  tone = "neutral",
}: {
  children: ReactNode;
  className?: string;
  tone?: BadgeTone;
}) {
  return (
    <span
      className={cn(
        "inline-flex w-fit items-center rounded-md border px-2 py-0.5 text-xs font-medium whitespace-nowrap",
        badgeToneClassName(tone),
        className
      )}
    >
      {children}
    </span>
  );
}

export function Panel({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <section className={cn("rounded-lg border border-zinc-200 bg-white shadow-sm", className)}>
      {children}
    </section>
  );
}

export function PageHeader({
  actions,
  children,
  description,
  title,
}: {
  actions?: ReactNode;
  children?: ReactNode;
  description?: string;
  title: string;
}) {
  return (
    <div className="flex flex-col gap-4 border-b border-zinc-200 pb-5 sm:flex-row sm:items-end sm:justify-between">
      <div className="min-w-0">
        <h1 className="text-2xl font-semibold tracking-normal text-zinc-950">{title}</h1>
        {description ? <p className="mt-1.5 text-sm text-zinc-600">{description}</p> : null}
        {children}
      </div>
      {actions ? <div className="shrink-0">{actions}</div> : null}
    </div>
  );
}

export function EmptyState({ children, title }: { children?: ReactNode; title: string }) {
  return (
    <div className="px-6 py-10 text-center">
      <h2 className="text-sm font-semibold text-zinc-950">{title}</h2>
      {children ? <div className="mt-2 text-sm text-zinc-600">{children}</div> : null}
    </div>
  );
}

export function StatusMessage({
  children,
  className,
  tone = "error",
}: {
  children: ReactNode;
  className?: string;
  tone?: "error" | "muted";
}) {
  return (
    <div
      className={cn(
        "rounded-md border px-3 py-2 text-sm",
        tone === "error"
          ? "border-red-200 bg-red-50 text-red-800"
          : "border-zinc-200 bg-zinc-50 text-zinc-600",
        className
      )}
      role={tone === "error" ? "alert" : undefined}
    >
      {children}
    </div>
  );
}

export function Table({ className, ...props }: ComponentPropsWithoutRef<"table">) {
  return (
    <table
      className={cn("w-full border-collapse text-left text-sm text-zinc-800", className)}
      {...props}
    />
  );
}

export function TableHead({ className, ...props }: ComponentPropsWithoutRef<"thead">) {
  return (
    <thead
      className={cn(
        "border-b border-zinc-200 bg-zinc-50 text-xs text-zinc-500 uppercase",
        className
      )}
      {...props}
    />
  );
}

export function TableHeaderCell({ className, ...props }: ComponentPropsWithoutRef<"th">) {
  return <th className={cn("px-4 py-3 font-medium", className)} {...props} />;
}

export function TableCell({ className, ...props }: ComponentPropsWithoutRef<"td">) {
  return <td className={cn("px-4 py-3 align-middle", className)} {...props} />;
}

export function SkeletonLine({ className }: { className?: string }) {
  return <div className={cn("animate-pulse rounded-md bg-zinc-200", className)} />;
}

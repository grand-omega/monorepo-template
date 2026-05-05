import type { ReactNode } from "react";

interface ConfirmDialogProps {
  children?: ReactNode;
  confirmLabel: string;
  description: string;
  disabled?: boolean;
  onCancel: () => void;
  onConfirm: () => void;
  open: boolean;
  title: string;
}

export function ConfirmDialog({
  children,
  confirmLabel,
  description,
  disabled = false,
  onCancel,
  onConfirm,
  open,
  title,
}: ConfirmDialogProps) {
  if (!open) return null;

  return (
    <div
      aria-labelledby="confirm-dialog-title"
      aria-modal="true"
      className="fixed inset-0 z-50 grid place-items-center bg-black/30 px-4"
      role="dialog"
    >
      <section className="w-full max-w-md rounded-lg border border-zinc-200 bg-white p-5 shadow-lg">
        <h2 className="text-lg font-semibold" id="confirm-dialog-title">
          {title}
        </h2>
        <p className="mt-2 text-sm text-zinc-600">{description}</p>
        {children ? <div className="mt-4">{children}</div> : null}
        <div className="mt-5 flex justify-end gap-2">
          <button
            className="rounded-md border border-zinc-300 px-3 py-2 text-sm text-zinc-700"
            onClick={onCancel}
            type="button"
          >
            Cancel
          </button>
          <button
            className="rounded-md bg-zinc-900 px-3 py-2 text-sm font-medium text-white disabled:cursor-not-allowed disabled:opacity-60"
            disabled={disabled}
            onClick={onConfirm}
            type="button"
          >
            {confirmLabel}
          </button>
        </div>
      </section>
    </div>
  );
}

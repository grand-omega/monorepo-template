import type { ReactNode } from "react";
import { Button, Panel, type ButtonVariant } from "@/components/ui";

interface ConfirmDialogProps {
  children?: ReactNode;
  confirmLabel: string;
  confirmVariant?: ButtonVariant;
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
  confirmVariant = "primary",
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
      <Panel className="w-full max-w-md p-5 shadow-lg">
        <h2 className="text-lg font-semibold" id="confirm-dialog-title">
          {title}
        </h2>
        <p className="mt-2 text-sm text-zinc-600">{description}</p>
        {children ? <div className="mt-4">{children}</div> : null}
        <div className="mt-5 flex justify-end gap-2">
          <Button onClick={onCancel} variant="secondary">
            Cancel
          </Button>
          <Button disabled={disabled} onClick={onConfirm} variant={confirmVariant}>
            {confirmLabel}
          </Button>
        </div>
      </Panel>
    </div>
  );
}

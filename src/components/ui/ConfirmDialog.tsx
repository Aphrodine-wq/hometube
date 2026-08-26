import { XIcon } from "@phosphor-icons/react";
import { AlertDialog } from "radix-ui";
import type { ReactNode } from "react";

interface ConfirmDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: string;
  description: string;
  confirmLabel: string;
  onConfirm: () => void;
  busy?: boolean;
  destructive?: boolean;
  requiredPhrase?: string | null;
  phraseValue?: string;
  onPhraseChange?: (value: string) => void;
  children?: ReactNode;
}

export function ConfirmDialog({
  open,
  onOpenChange,
  title,
  description,
  confirmLabel,
  onConfirm,
  busy = false,
  destructive = false,
  requiredPhrase,
  phraseValue = "",
  onPhraseChange,
  children,
}: ConfirmDialogProps) {
  const phraseMatches = !requiredPhrase || phraseValue === requiredPhrase;

  return (
    <AlertDialog.Root open={open} onOpenChange={onOpenChange}>
      <AlertDialog.Portal>
        <AlertDialog.Overlay className="dialog-overlay" />
        <AlertDialog.Content className="dialog-content confirm-dialog">
          <div className="dialog-heading">
            <div>
              <AlertDialog.Title>{title}</AlertDialog.Title>
              <AlertDialog.Description>{description}</AlertDialog.Description>
            </div>
            <AlertDialog.Cancel className="icon-button" aria-label="Close confirmation">
              <XIcon weight="bold" />
            </AlertDialog.Cancel>
          </div>
          {children}
          {requiredPhrase ? (
            <label className="confirmation-phrase">
              <span>Type <strong>{requiredPhrase}</strong> to continue</span>
              <input
                autoComplete="off"
                spellCheck={false}
                value={phraseValue}
                onChange={(event) => onPhraseChange?.(event.target.value)}
              />
            </label>
          ) : null}
          <div className="dialog-actions">
            <AlertDialog.Cancel className="secondary-button">Cancel</AlertDialog.Cancel>
            <AlertDialog.Action
              className={destructive ? "danger-button" : "primary-button"}
              disabled={busy || !phraseMatches}
              onClick={(event) => {
                event.preventDefault();
                onConfirm();
              }}
            >
              {busy ? "Working…" : confirmLabel}
            </AlertDialog.Action>
          </div>
        </AlertDialog.Content>
      </AlertDialog.Portal>
    </AlertDialog.Root>
  );
}

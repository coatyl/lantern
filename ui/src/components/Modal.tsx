/**
 * Shared modal chrome: a dimmed backdrop that closes on click, a
 * focus-trapped dialog panel, and the standard header and close button.
 *
 * Mount a `Modal` only while it is open: unmounting releases the focus trap
 * and returns focus to the element that opened it.
 */

import type { ReactNode } from "react";
import { useFocusTrap } from "../hooks/useFocusTrap";
import { XIcon } from "./Icons";

/** Filled accent button for a dialog's primary action; add padding per use. */
export const primaryButton =
  "rounded text-xs font-medium bg-accent hover:bg-accent-hover text-on-accent " +
  "disabled:opacity-50 transition-colors focus:outline-none " +
  "focus-visible:ring-2 focus-visible:ring-accent";

interface ModalProps {
  /** Accessible name of the dialog. */
  label: string;
  /** Called on Escape and on a click on the backdrop. */
  onClose: () => void;
  /** Size and layout of the panel. */
  className: string;
  /** Stacking and vertical placement of the backdrop. */
  placement?: string;
  children: ReactNode;
}

export function Modal({
  label,
  onClose,
  className,
  placement = "z-50 items-center",
  children,
}: ModalProps) {
  const ref = useFocusTrap<HTMLDivElement>(true, onClose);
  return (
    <div
      className={`fixed inset-0 flex justify-center bg-scrim/60 backdrop-blur-sm
                  animate-fade-in ${placement}`}
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div
        ref={ref}
        role="dialog"
        aria-modal="true"
        aria-label={label}
        className={`bg-surface-1 border border-neutral-800 rounded-lg shadow-2xl
                    overflow-hidden max-w-[95vw] ${className}`}
      >
        {children}
      </div>
    </div>
  );
}

export function ModalHeader({
  title,
  subtitle,
  closeLabel,
  onClose,
}: {
  title: string;
  subtitle?: string;
  closeLabel: string;
  onClose: () => void;
}) {
  return (
    <div className="px-4 py-3 border-b border-neutral-800 flex items-center justify-between gap-3 shrink-0">
      <div>
        <h2 className="text-sm font-semibold text-neutral-100">{title}</h2>
        {subtitle && <p className="text-[10px] text-neutral-500 mt-0.5">{subtitle}</p>}
      </div>
      <ModalCloseButton label={closeLabel} onClick={onClose} />
    </div>
  );
}

export function ModalCloseButton({
  label,
  onClick,
  className = "",
}: {
  label: string;
  onClick: () => void;
  className?: string;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-label={label}
      className={`inline-flex items-center justify-center w-8 h-8 rounded
                  text-neutral-400 hover:text-neutral-100 hover:bg-neutral-800/60
                  transition-colors focus:outline-none focus-visible:ring-1
                  focus-visible:ring-accent ${className}`}
    >
      <XIcon className="w-4 h-4" />
    </button>
  );
}

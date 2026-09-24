/**
 * Asks before closing tabs that hold edits.  Lantern keeps edits in memory
 * until a copy is saved (the original file is never written), so closing an
 * edited tab would silently lose work.
 */

import { useState } from "react";

import { useFocusTrap } from "../hooks/useFocusTrap";
import { useDocuments } from "../state/documents";
import { useFileActions } from "../state/fileActions";
import { useT } from "../i18n/I18nProvider";

export default function CloseGuardDialog() {
  const t = useT();
  const { tabs, pendingClose, confirmClose, cancelClose } = useDocuments();
  const { saveCopyOf } = useFileActions();
  const [saving, setSaving] = useState(false);
  const trapRef = useFocusTrap<HTMLDivElement>(pendingClose !== null, cancelClose);

  const edited = pendingClose ? tabs.filter((tab) => pendingClose.tabIds.includes(tab.id) && tab.dirty) : [];
  if (!pendingClose || edited.length === 0) return null;

  const saveThenClose = async () => {
    setSaving(true);
    try {
      for (const tab of edited) {
        // Stop at the first cancelled or failed save; nothing is closed.
        if (!(await saveCopyOf(tab.id))) return;
      }
      await confirmClose();
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="fixed inset-0 z-[65] flex items-center justify-center bg-scrim/60 backdrop-blur-sm animate-fade-in">
      <div
        ref={trapRef}
        role="dialog"
        aria-modal="true"
        aria-label={t("closeGuard.title")}
        className="w-[440px] max-w-[95vw] bg-surface-1 border border-neutral-800 rounded-lg shadow-2xl overflow-hidden"
      >
        <h2 className="px-4 py-3 border-b border-neutral-800 text-sm font-semibold text-neutral-100">
          {t("closeGuard.title")}
        </h2>
        <div className="px-4 py-3 space-y-2 text-sm text-neutral-300">
          <p>{t("closeGuard.body", { n: edited.length })}</p>
          <ul className="list-disc pl-5 text-neutral-100">
            {edited.map((tab) => (
              <li key={tab.id} className="truncate">{tab.title}</li>
            ))}
          </ul>
        </div>
        <div className="px-4 py-3 border-t border-neutral-800 flex justify-end gap-2">
          <button
            type="button"
            onClick={cancelClose}
            disabled={saving}
            className="px-3 py-1.5 rounded text-xs text-neutral-400 hover:text-neutral-100"
          >
            {t("common.cancel")}
          </button>
          <button
            type="button"
            onClick={() => void confirmClose()}
            disabled={saving}
            className="px-3 py-1.5 rounded border border-danger/50 text-xs text-danger hover:bg-danger/10"
          >
            {t("closeGuard.discard")}
          </button>
          <button type="button" onClick={() => void saveThenClose()} disabled={saving} className="px-3 py-1.5 rounded text-xs font-medium bg-accent hover:bg-accent-hover text-on-accent disabled:opacity-50">
            {t("closeGuard.save", { n: edited.length })}
          </button>
        </div>
      </div>
    </div>
  );
}

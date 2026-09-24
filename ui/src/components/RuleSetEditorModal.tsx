/**
 * Rule-set editor modal.
 *
 * Left: the rule sets, with new / duplicate / delete. Right: the selected
 * set's drag-sortable treatment list with inline config, saved as a whole.
 * Built-in sets are read-only; duplicating one gives an editable copy.
 */

import { useState, useEffect, useMemo } from "react";
import { ipc } from "../ipc";
import type { RuleSetDetail, RuleSetSummary, TreatmentInfo } from "../ipc/types";
import { Modal, ModalCloseButton } from "./Modal";
import { RuleSetList, type NameInput } from "./rule-sets/RuleSetList";
import { TreatmentEditor } from "./rule-sets/TreatmentEditor";
import {
  CONFIGURABLE_IDS,
  configsFromDetail,
  defaultConfigFor,
  type ConfigsMap,
} from "./rule-sets/treatments";

interface RuleSetEditorModalProps {
  open: boolean;
  initialSet?: string;
  onClose: () => void;
  /** Called after any mutation so the parent can refresh its picker. */
  onRefreshList: () => void;
}

export function RuleSetEditorModal({
  open,
  initialSet,
  onClose,
  onRefreshList,
}: RuleSetEditorModalProps) {
  const [summaries, setSummaries] = useState<RuleSetSummary[]>([]);
  const [allTreatments, setAllTreatments] = useState<TreatmentInfo[]>([]);
  const [loadingData, setLoadingData] = useState(true);

  // The selected set as last loaded or saved, and the unsaved edits on top.
  const [selectedName, setSelectedName] = useState<string | null>(initialSet ?? null);
  const [detail, setDetail] = useState<RuleSetDetail | null>(null);
  const [editedIds, setEditedIds] = useState<string[]>([]);
  const [editedConfigs, setEditedConfigs] = useState<ConfigsMap>({});

  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [nameInput, setNameInput] = useState<NameInput | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);

  const savedConfigs = useMemo(() => (detail ? configsFromDetail(detail) : {}), [detail]);

  const dirty = useMemo(() => {
    if (JSON.stringify(editedIds) !== JSON.stringify(detail?.treatment_ids ?? [])) return true;
    // Only treatments still in the set count toward config changes.
    const live: ConfigsMap = {};
    const liveSaved: ConfigsMap = {};
    for (const id of editedIds) {
      if (id in editedConfigs) live[id] = editedConfigs[id];
      if (id in savedConfigs) liveSaved[id] = savedConfigs[id];
    }
    return JSON.stringify(live) !== JSON.stringify(liveSaved);
  }, [editedIds, editedConfigs, detail, savedConfigs]);

  /** Replace the loaded set and drop any edits. */
  const showDetail = (d: RuleSetDetail | null) => {
    setDetail(d);
    setEditedIds(d?.treatment_ids ?? []);
    setEditedConfigs(d ? configsFromDetail(d) : {});
  };

  useEffect(() => {
    if (!open) return;
    setLoadingData(true);
    Promise.all([ipc.listRuleSets(), ipc.listTreatments()])
      .then(([list, treatments]) => {
        setSummaries(list);
        setAllTreatments(treatments);
      })
      .catch(() => {
        // Keep whatever we already have.
      })
      .finally(() => setLoadingData(false));
  }, [open]);

  // Fall back to the first set when nothing (or a deleted set) is selected.
  useEffect(() => {
    if (!selectedName && summaries.length > 0) setSelectedName(summaries[0].name);
  }, [summaries, selectedName]);

  useEffect(() => {
    if (!selectedName) {
      showDetail(null);
      return;
    }
    let cancelled = false;
    ipc
      .getRuleSet(selectedName)
      .then((d) => {
        if (cancelled) return;
        showDetail(d);
        setError(null);
      })
      .catch((e) => {
        if (!cancelled) setError(String(e));
      });
    return () => {
      cancelled = true;
    };
  }, [selectedName]);

  /** Run a mutation, reporting failure inline; returns whether it succeeded. */
  const mutate = async (action: () => Promise<void>): Promise<boolean> => {
    setError(null);
    try {
      await action();
      onRefreshList();
      return true;
    } catch (e) {
      setError(String(e));
      return false;
    }
  };

  const handleSave = async () => {
    if (!selectedName || !detail || detail.is_builtin) return;
    setSaving(true);
    await mutate(async () => {
      const fresh = await ipc.saveRuleSetWithConfigs(
        selectedName,
        editedIds.map((id) => ({
          id,
          config: CONFIGURABLE_IDS.has(id) ? (editedConfigs[id] ?? defaultConfigFor(id)) : null,
        })),
      );
      showDetail(fresh);
      setSummaries((prev) =>
        prev.map((s) =>
          s.name === selectedName ? { ...s, treatment_count: fresh.treatment_ids.length } : s,
        ),
      );
    });
    setSaving(false);
  };

  const handleDelete = async (name: string) => {
    setConfirmDelete(null);
    await mutate(async () => {
      await ipc.deleteRuleSet(name);
      setSummaries((prev) => prev.filter((s) => s.name !== name));
      if (selectedName === name) setSelectedName(null);
    });
  };

  /** Create or duplicate a set, then select it. */
  const handleConfirmName = async (name: string) => {
    const input = nameInput;
    setNameInput(null);
    if (!input) return;
    await mutate(async () => {
      if (input.kind === "new") await ipc.saveRuleSet(name, []);
      else await ipc.duplicateRuleSet(input.source, name);
      setSummaries(await ipc.listRuleSets());
      setSelectedName(name);
    });
  };

  const confirmDiscard = (message: string) => !dirty || window.confirm(message);

  const handleSelect = (name: string) => {
    if (name === selectedName || !confirmDiscard("Discard unsaved changes?")) return;
    setSelectedName(name);
    setError(null);
  };

  // Shared by Escape, the backdrop, and the close button.
  const handleClose = () => {
    if (confirmDiscard("You have unsaved changes. Discard and close?")) onClose();
  };

  if (!open) return null;

  return (
    <Modal
      label="Rule set editor"
      onClose={handleClose}
      className="flex w-[900px] h-[620px] max-h-[90vh]"
    >
      <RuleSetList
        summaries={summaries}
        loading={loadingData}
        selectedName={selectedName}
        nameInput={nameInput}
        onSelect={handleSelect}
        onNameInput={setNameInput}
        onConfirmName={handleConfirmName}
        onRequestDelete={setConfirmDelete}
      />

      <div className="flex-1 flex flex-col min-w-0 relative">
        <ModalCloseButton
          label="Close editor"
          onClick={handleClose}
          className="absolute top-3 right-3 z-10"
        />
        {detail ? (
          <TreatmentEditor
            detail={detail}
            ids={editedIds}
            setIds={setEditedIds}
            configs={editedConfigs}
            setConfigs={setEditedConfigs}
            dirty={dirty}
            saving={saving}
            error={error}
            allTreatments={allTreatments}
            onSave={handleSave}
            onDuplicate={() => setNameInput({ kind: "duplicate", source: detail.name })}
            onDiscard={() => {
              showDetail(detail);
              setError(null);
            }}
          />
        ) : (
          <div className="flex-1 flex items-center justify-center px-6 text-xs text-neutral-600">
            {error ? (
              <span className="text-danger">{error}</span>
            ) : loadingData ? (
              "Loading…"
            ) : (
              "Select a rule set."
            )}
          </div>
        )}
      </div>

      {confirmDelete && (
        <Modal
          label="Delete rule set?"
          onClose={() => setConfirmDelete(null)}
          className="p-5 w-72"
        >
          <p className="text-sm text-neutral-200 mb-1">Delete rule set?</p>
          <p className="text-xs text-neutral-500 mb-4">
            <span className="font-medium text-neutral-300">{confirmDelete}</span> will be
            permanently removed.
          </p>
          <div className="flex justify-end gap-2">
            <button
              onClick={() => setConfirmDelete(null)}
              className="px-3 py-1.5 rounded text-xs text-neutral-400
                         hover:text-neutral-200 transition-colors
                         focus:outline-none focus-visible:ring-1 focus-visible:ring-accent"
            >
              Cancel
            </button>
            <button
              onClick={() => handleDelete(confirmDelete)}
              className="px-3 py-1.5 rounded text-xs font-medium
                         bg-danger/90 hover:bg-danger text-on-danger
                         transition-colors focus:outline-none
                         focus-visible:ring-2 focus-visible:ring-danger"
            >
              Delete
            </button>
          </div>
        </Modal>
      )}
    </Modal>
  );
}

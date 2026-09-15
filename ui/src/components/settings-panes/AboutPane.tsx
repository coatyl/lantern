/**
 * About pane: surfaces compile-time facts (version, build flavor, license).
 *
 * Backed by the `get_build_info` IPC command.
 */

import { useEffect, useState } from "react";
import { ipc } from "../../ipc";
import type { BuildInfo } from "../../ipc/types";
import { SkeletonText } from "../Skeleton";
import { useToast } from "../../hooks/useToast";
import { useT } from "../../i18n/I18nProvider";

export function AboutPane() {
  const { toast } = useToast();
  const t = useT();
  const [info, setInfo] = useState<BuildInfo | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    ipc.getBuildInfo()
      .then((b) => {
        if (!cancelled) setInfo(b);
      })
      .catch((e) => {
        if (!cancelled) {
          // Load failures route through the toast surface; the rest of the
          // pane (the section heading) still renders, the user just doesn't
          // see the build-info table yet.  v0.0.11 QoL slice 1.
          toast(String(e), "error");
        }
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [toast]);

  return (
    <section aria-label="About Lantern">
      <h3 className="text-sm font-semibold text-neutral-200 mb-3">Lantern</h3>

      {loading ? (
        <div aria-busy="true" aria-label={t("loading.generic")} className="space-y-2">
          {Array.from({ length: 6 }).map((_, i) => (
            <SkeletonText key={i} lines={1} />
          ))}
        </div>
      ) : info ? (
        <dl className="text-xs space-y-2">
          <div className="flex">
            <dt className="w-32 shrink-0 text-neutral-500">Version</dt>
            <dd className="text-neutral-200 font-mono">{info.version}</dd>
          </div>
          <div className="flex">
            <dt className="w-32 shrink-0 text-neutral-500">Build flavor</dt>
            <dd className="text-neutral-200 font-mono">{info.build_flavor}</dd>
          </div>
          <div className="flex">
            <dt className="w-32 shrink-0 text-neutral-500">Rust version</dt>
            <dd className="text-neutral-200 font-mono">{info.rust_version}</dd>
          </div>
          <div className="flex">
            <dt className="w-32 shrink-0 text-neutral-500">License</dt>
            <dd className="text-neutral-200 font-mono">{info.license}</dd>
          </div>
          <div className="flex">
            <dt className="w-32 shrink-0 text-neutral-500">Signed</dt>
            <dd className="font-mono">
              {info.signed ? (
                <span className="text-accent" aria-label="Authenticode-signed build">
                  ✓ signed
                </span>
              ) : (
                <span className="text-neutral-500" aria-label="Unsigned build">
                  unsigned
                </span>
              )}
            </dd>
          </div>
          <div className="flex">
            <dt className="w-32 shrink-0 text-neutral-500">ADR index</dt>
            <dd className="text-neutral-200 font-mono break-all">
              {info.adr_index_path}
            </dd>
          </div>
          <div className="flex">
            <dt className="w-32 shrink-0 text-neutral-500">SBOM</dt>
            <dd>
              {/* TODO: replace with the real SBOM URL once releases land */}
              <a
                href="https://github.com/<org>/lantern/releases"
                target="_blank"
                rel="noopener noreferrer"
                className="text-accent underline focus-visible:ring-1 focus-visible:ring-accent rounded"
              >
                Software bill of materials
              </a>
              <span className="ml-2 text-xs text-neutral-500">
                (generated per release)
              </span>
            </dd>
          </div>
        </dl>
      ) : null}
    </section>
  );
}

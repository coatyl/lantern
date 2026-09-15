import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";

import { I18nProvider, useT } from "./I18nProvider";

/**
 * The framework is intentionally minimal; we only verify three load-bearing
 * behaviours: known keys resolve, unknown keys fall through to the key
 * itself (so missing translations are visible but harmless), and `{name}`
 * parameter substitution works.
 */

function Probe({ k, params }: { k: string; params?: Record<string, string | number> }) {
  const t = useT();
  return <span data-testid="out">{t(k, params)}</span>;
}

describe("I18nProvider", () => {
  it("returns the literal string for a known key", () => {
    render(
      <I18nProvider locale="en">
        <Probe k="titleBar.open" />
      </I18nProvider>,
    );
    expect(screen.getByTestId("out").textContent).toBe("Open");
  });

  it("returns the key itself when the key is unknown (graceful fallback)", () => {
    render(
      <I18nProvider locale="en">
        <Probe k="this.key.does.not.exist" />
      </I18nProvider>,
    );
    expect(screen.getByTestId("out").textContent).toBe(
      "this.key.does.not.exist",
    );
  });

  it("substitutes {name}-style parameters in the resolved string", () => {
    render(
      <I18nProvider locale="en">
        <Probe k="statusBar.bookmarks" params={{ n: 42 }} />
      </I18nProvider>,
    );
    expect(screen.getByTestId("out").textContent).toBe("42 bookmarks");
  });
});

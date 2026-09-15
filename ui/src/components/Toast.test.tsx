/**
 * Toaster + useToast: v0.0.11 QoL slice 1.
 *
 * Three behaviours we care about:
 *
 *   1. A queued info toast renders.
 *   2. An info toast auto-dismisses after its 4 s timeout (verified with
 *      `vi.useFakeTimers`).
 *   3. An error toast does NOT auto-dismiss; it stays on screen until the
 *      user hits the inline × button.
 *
 * The toast store is a module-scope Zustand singleton, so we reset it
 * between test cases via the public `clear()` action.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { act, render, screen } from "@testing-library/react";

import Toaster from "./Toast";
import { useToastStore } from "../state/toasts";
import { I18nProvider } from "../i18n/I18nProvider";

function renderToaster() {
  return render(
    <I18nProvider locale="en">
      <Toaster />
    </I18nProvider>,
  );
}

beforeEach(() => {
  useToastStore.getState().clear();
});

afterEach(() => {
  vi.useRealTimers();
  useToastStore.getState().clear();
});

describe("Toaster", () => {
  it("renders an info toast that was pushed into the store", () => {
    renderToaster();
    act(() => {
      useToastStore.getState().push("Hello info", "info");
    });
    expect(screen.getByText("Hello info")).toBeInTheDocument();
  });

  it("auto-dismisses an info toast after the 4s timeout", () => {
    vi.useFakeTimers();
    renderToaster();
    act(() => {
      useToastStore.getState().push("Goodbye info", "info");
    });
    expect(screen.getByText("Goodbye info")).toBeInTheDocument();
    act(() => {
      vi.advanceTimersByTime(4_500);
    });
    expect(screen.queryByText("Goodbye info")).not.toBeInTheDocument();
  });

  it("error toasts do NOT auto-dismiss", () => {
    vi.useFakeTimers();
    renderToaster();
    act(() => {
      useToastStore.getState().push("Boom", "error");
    });
    expect(screen.getByText("Boom")).toBeInTheDocument();
    // Even after a long stretch the error sticks around.
    act(() => {
      vi.advanceTimersByTime(60_000);
    });
    expect(screen.getByText("Boom")).toBeInTheDocument();
  });
});

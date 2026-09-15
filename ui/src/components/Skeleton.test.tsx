/**
 * Skeleton: v0.0.11 QoL slice 1.
 *
 * SkeletonRow renders three placeholder shapes (icon + title + url).
 */

import { describe, it, expect } from "vitest";
import { render } from "@testing-library/react";

import { SkeletonRow } from "./Skeleton";

describe("Skeleton", () => {
  it("SkeletonRow renders three placeholder blocks", () => {
    const { container } = render(<SkeletonRow />);
    // The wrapper's three shape children: icon + title + url placeholder.
    const wrapper = container.firstElementChild!;
    expect(wrapper).not.toBeNull();
    expect(wrapper.children.length).toBe(3);
    // Each child should carry the animate-pulse class so the row visibly
    // breathes.
    Array.from(wrapper.children).forEach((child) => {
      expect(child.className).toMatch(/animate-pulse/);
    });
  });
});

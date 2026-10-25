// node --test scripts/branch-name.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import { checkBranchName } from "./branch-name.mjs";

test("accepts <type>/<topic>, release/<X.Y.Z> and bot branches", () => {
  for (const name of [
    "feat/io-firefox-places",
    "fix/core-undo-order",
    "chore/repo-templates",
    "ci/linux-first",
    "release/0.2.0",
    "dependabot/npm_and_yarn/vite-5.4.0",
    "renovate/tauri-2.x",
  ]) {
    assert.equal(checkBranchName(name), null, name);
  }
});

test("rejects personal, generated and malformed names with a reason", () => {
  for (const name of [
    "wyvern/great-brahmagupta-ert8ja",
    "wyvern-agent-branch/repo-cleanup-1a1c",
    "patch-1",
    "main",
    "feature/ui-thing",
    "feat/Bad_Name",
    "feat/ui/nested",
    "feat/",
    "release/0.2",
    "feat/an-extremely-long-topic-that-goes-on-and-on-and-on",
  ]) {
    assert.match(checkBranchName(name) ?? "", /./, name);
  }
});

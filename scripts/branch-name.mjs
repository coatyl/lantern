#!/usr/bin/env node
// Branch-naming rule (see CONTRIBUTING.md → Branches), shared by
// `npm run branch` and the CI `branch-name` check.
//
//   node scripts/branch-name.mjs <name>   exits 1 with a reason if invalid

import { pathToFileURL } from "node:url";

export const TYPES = ["feat", "fix", "perf", "refactor", "docs", "test", "build", "ci", "chore", "revert"];
const MAX_LENGTH = 50;
/** Branches opened by bots, which follow their own naming. */
const EXEMPT = [/^dependabot\//, /^renovate\//];

/** Returns null when `name` follows the convention, otherwise the reason it does not. */
export function checkBranchName(name) {
  if (EXEMPT.some((re) => re.test(name))) return null;
  if (/^release\/\d+\.\d+\.\d+$/.test(name)) return null;
  const [type, ...rest] = name.split("/");
  const topic = rest.join("/");
  if (!TYPES.includes(type) || rest.length !== 1) {
    return `expected <type>/<topic> with type one of ${TYPES.join(", ")}, or release/<X.Y.Z>`;
  }
  if (!/^[a-z0-9]+(-[a-z0-9]+)*$/.test(topic)) {
    return "the topic must be lowercase words joined with '-' (e.g. io-firefox-places)";
  }
  if (name.length > MAX_LENGTH) return `keep it under ${MAX_LENGTH} characters`;
  return null;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const name = process.argv[2] ?? "";
  const problem = checkBranchName(name);
  if (problem) {
    console.error(`Branch "${name}" does not follow the naming convention: ${problem}.`);
    console.error("See CONTRIBUTING.md → Branches.");
    process.exit(1);
  }
  console.log(`Branch "${name}" follows the naming convention.`);
}

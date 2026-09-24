#!/usr/bin/env node
// Create a correctly named branch from an up-to-date main.
//
//   npm run branch -- feat/io-firefox-places
//   npm run branch -- release/0.3.0

import { execFileSync } from "node:child_process";
import { checkBranchName } from "./branch-name.mjs";

const name = process.argv[2];
if (!name) {
  console.error("Usage: npm run branch -- <type>/<topic>   (e.g. feat/ui-review-filters)");
  process.exit(1);
}
const problem = checkBranchName(name);
if (problem) {
  console.error(`"${name}" does not follow the naming convention: ${problem}.`);
  process.exit(1);
}

const git = (...args) => execFileSync("git", args, { stdio: "inherit" });
git("fetch", "origin", "main");
git("switch", "--create", name, "origin/main");
console.log(`\nOn ${name}, branched from origin/main. Push with: git push -u origin ${name}`);

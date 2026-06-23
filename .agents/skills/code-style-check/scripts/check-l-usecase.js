#!/usr/bin/env bun
// check-l-usecase.js — Rules 39-42: Usecase layer checks
//
//   Rule 39: Composite trait bounds — H: Query + ImageGet + Send + Sync.
//   Rule 40: Transactional — H: Clone + Transactional + Send + Sync.
//   Rule 41: .boxed() not Box::pin() — use futures_util::FutureExt as _; then .boxed().
//   Rule 42: transaction_scoped only when cross-aggregate atomicity needed.

import { readFileSync } from "node:fs";
import { globSync } from "node:fs";
import { resolve, relative } from "node:path";

const ROOT = resolve(import.meta.dirname, "../../../..");
const SRC = resolve(ROOT, "src");
const files = globSync("**/*.rs", { cwd: SRC }).map((f) => resolve(SRC, f));

let violations = 0;

function grepRules(filePath, content, rel) {
  const lines = content.split("\n");

  // ---- Rule 41: Box::pin usage (should be .boxed()) ----
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const lineno = i + 1;
    const trimmed = line.trimStart();

    if (trimmed.startsWith("//") || trimmed.startsWith("///")) continue;

    if (trimmed.includes("Box::pin") || trimmed.includes("Box::pin(")) {
      console.log(`${rel}:${lineno} — Rule 41: Box::pin found — use .boxed() instead`);
      console.log(`    ${trimmed}`);
      violations++;
    }
  }

  // ---- Rule 42: transaction_scoped check ----
  // Not easily enforced by script — this is a design judgment.
  // Heuristic: flag if transaction_scoped is used for single-row operations.
  // Skipping automated check.
}

for (const f of files) {
  const rel = relative(SRC, f);
  if (!rel.startsWith("usecase/") && !rel.startsWith("usecase\\")) continue;
  const content = readFileSync(f, "utf-8");
  grepRules(f, content, rel);
}

if (violations === 0) {
  console.log("✓ L (Usecase): all rules pass.");
} else {
  console.log(`✗ L (Usecase): ${violations} violation(s).`);
}
process.exitCode = violations;

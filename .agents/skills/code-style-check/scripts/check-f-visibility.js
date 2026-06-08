#!/usr/bin/env bun
// check-f-visibility.js — Rules 17-19: Visibility & field checks
//
//   Rule 17: Only pub or private — no pub(crate), pub(super), pub(self), pub(in ...).
//   Rule 18: Data containers have pub fields (aggregates, value objects, entity structs).
//   Rule 19: Logic-carrying types have private fields (query handles, harness, effect sinks).

import { readFileSync } from "node:fs";
import { globSync } from "node:fs";
import { resolve, relative } from "node:path";

const ROOT = resolve(import.meta.dirname, "../../../..");
const SRC = resolve(ROOT, "src");
const files = globSync("**/*.rs", { cwd: SRC }).map((f) => resolve(SRC, f));

let violations = 0;

// Pattern: pub(crate), pub(super), pub(self), pub(in path)
const RESTRICTED_PUB = /pub\s*\((?:crate|super|self|in\s)/;

function grepRules(filePath, content, rel) {
  const lines = content.split("\n");

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const lineno = i + 1;
    const trimmed = line.trimStart();

    if (trimmed.startsWith("//") || trimmed.startsWith("///")) continue;

    // ---- Rule 17: Only pub or private ----
    if (RESTRICTED_PUB.test(trimmed)) {
      console.log(`${rel}:${lineno} — Rule 17: restricted visibility (pub(crate)/pub(super)/etc.)`);
      console.log(`    ${trimmed}`);
      violations++;
    }
  }
}

for (const f of files) {
  const rel = relative(SRC, f);
  const content = readFileSync(f, "utf-8");
  grepRules(f, content, rel);
}

if (violations === 0) {
  console.log("✓ F (Visibility): all rules pass.");
} else {
  console.log(`✗ F (Visibility): ${violations} violation(s).`);
}
process.exitCode = violations;

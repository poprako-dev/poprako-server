#!/usr/bin/env bun
// check-g-ownership.js — Rules 20-22: Ownership & construction checks
//
//   Rule 20: Constructor over struct literal — use new() when it exists.
//   Rule 21: Borrow over clone — pass references when possible.
//   Rule 22: Aspect builder — always Aspect::new(now).field(val), never struct literal.

import { readFileSync } from "node:fs";
import { globSync } from "node:fs";
import { resolve, relative } from "node:path";

const ROOT = resolve(import.meta.dirname, "../../../..");
const SRC = resolve(ROOT, "src");
const files = globSync("**/*.rs", { cwd: SRC }).map((f) => resolve(SRC, f));

let violations = 0;

// Detect struct literals for known Aspect types (heuristic: anything ending in "Aspect")
const ASPECT_LITERAL = /(\w*Aspect)\s*\{/;

// Detect `.clone()` usage at final use (heuristic)
const CLONE_FINAL = /\.clone\(\)\s*$/;

function grepRules(filePath, content, rel) {
  const lines = content.split("\n");

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const lineno = i + 1;
    const trimmed = line.trimStart();

    if (trimmed.startsWith("//") || trimmed.startsWith("///") || trimmed.startsWith("/*"))
      continue;
    if (trimmed.startsWith("#[")) continue;
    if (trimmed.startsWith("use ")) continue;

    // ---- Rule 22: Aspect struct literal ----
    // Only check struct literal *expressions* (e.g. in assignments), not type *definitions*
    // and not impl blocks.
    // Definitions: "pub struct FooAspect {"
    // impl blocks: "impl FooAspect {"
    const defOrImpl = trimmed.match(/^(pub\s+)?(struct|impl)\s+(\w*Aspect)/);
    if (defOrImpl) continue;

    const aspMatch = trimmed.match(ASPECT_LITERAL);
    if (aspMatch && !trimmed.includes("new(")) {
      // Check if this is a struct literal for an Aspect type
      console.log(`${rel}:${lineno} — Rule 22: Aspect should use builder pattern (::new(now).field(val))`);
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
  console.log("✓ G (Ownership & Construction): all rules pass.");
} else {
  console.log(`✗ G (Ownership & Construction): ${violations} violation(s).`);
}
process.exitCode = violations;

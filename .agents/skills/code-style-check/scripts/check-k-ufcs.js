#!/usr/bin/env bun
// check-k-ufcs.js — Rules 37-38: UFCS (Universal Function Call Syntax) checks
//
//   Rule 37: UFCS for trait calls on harness — Trait::method(harn, args) ✓, harn.method(args) ✗
//   Rule 38: UFCS inside transaction_scoped closures — Trait::method(query, args) ✓

import { readFileSync } from "node:fs";
import { globSync } from "node:fs";
import { resolve, relative } from "node:path";

const ROOT = resolve(import.meta.dirname, "../../../..");
const SRC = resolve(ROOT, "src");
const files = globSync("**/*.rs", { cwd: SRC }).map((f) => resolve(SRC, f));

let violations = 0;

// Usecase files only (rule 37 applies to src/usecase/)
// Transactional closure check applies to all files

function grepRules(filePath, content, rel) {
  const lines = content.split("\n");
  const isUsecase = rel.startsWith("usecase/") || rel.startsWith("usecase\\");

  if (!isUsecase) return;

  // ---- Rule 37: Harness method calls in usecase ----
  // In usecase business logic (not test modules), check for harn.something()
  // Test modules are excluded.
  // Use a simple heuristic: skip everything after 'mod tests {' block.
  let depth = 0;
  let inTest = false;
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmed = line.trimStart();
    const lineno = i + 1;

    // Track test module boundaries
    if (/^\s*(?:#\[cfg\(test\)\]\s*)?mod\s+tests\s*\{/.test(line)) {
      inTest = true;
      depth = 1;
      continue;
    }
    if (inTest) {
      for (const ch of line) {
        if (ch === "{") depth++;
        if (ch === "}") depth--;
      }
      if (depth <= 0) {
        inTest = false;
        depth = 0;
      }
      continue;
    }

    if (trimmed.startsWith("//") || trimmed.startsWith("///")) continue;
    if (trimmed.startsWith("#[test")) continue;

    // Look for harn.XYZ(args) — but NOT Trait::XYZ(harn, ...)
    const harnMethodCall = trimmed.match(/\bharn\.(\w+)\s*\(/);
    if (harnMethodCall) {
      // Skip test harness methods (seed_*, snapshot, events)
      const method = harnMethodCall[1];
      if (/^(seed_|snapshot|events|invited_)/.test(method)) continue;

      console.log(
        `${rel}:${lineno} — Rule 37: harn.${method}() should be Trait::${method}(harn, ...)`,
      );
      console.log(`    ${trimmed}`);
      violations++;
    }
  }

  // ---- Rule 38: UFCS inside transaction_scoped closures ----
  // Scan for transaction_scoped blocks and check internal method calls
  let inTxScoped = false;
  let txBraceDepth = 0;
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmed = line.trimStart();

    if (trimmed.includes("transaction_scoped")) {
      inTxScoped = true;
      txBraceDepth = 0;
      continue;
    }

    if (inTxScoped) {
      if (trimmed.startsWith("//") || trimmed.startsWith("///")) continue;
      if (trimmed.startsWith("use ")) continue;
      if (trimmed.startsWith(".boxed()")) {
        inTxScoped = false;
        continue;
      }

      // Check for query.something() which should be Trait::something(query, ...)
      const queryMethod = trimmed.match(/\bquery\.(\w+)\s*\(/);
      if (queryMethod) {
        const methodName = queryMethod[1];
        // Likely a violation — should use Trait::method(query, ...)
        console.log(
          `${rel}:${i + 1} — Rule 38: query.${methodName}() should use Trait::${methodName}(query, ...)`,
        );
        console.log(`    ${trimmed}`);
        violations++;
      }
    }

    // Track brace depth to find closure end
    if (inTxScoped) {
      for (const ch of line) {
        if (ch === "{") txBraceDepth++;
        if (ch === "}") {
          txBraceDepth--;
          if (txBraceDepth < 0) inTxScoped = false;
        }
      }
    }
  }
}

for (const f of files) {
  const rel = relative(SRC, f);
  const content = readFileSync(f, "utf-8");
  grepRules(f, content, rel);
}

if (violations === 0) {
  console.log("✓ K (UFCS): all rules pass.");
} else {
  console.log(`✗ K (UFCS): ${violations} violation(s).`);
}
process.exitCode = violations;

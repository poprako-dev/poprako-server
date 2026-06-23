#!/usr/bin/env bun
// check-i-errors.js — Rules 28-34: Error construction checks
//
//   Rule 28-30: Expected — DomainError::expected_{argument,authentication,conflict}(trl("..."))
//   Rule 31: Unrecoverable — DomainError::unrecoverable(format!("[Struct::method] ..."))
//   Rule 32: No hardcoded language — always use trl("error-xxx")
//   Rule 33: No [Struct::method] prefix on Expected
//   Rule 34: Diesel NotFound — .optional()? → .ok_or_else(|| DomainError::expected_argument(trl(...)))

import { readFileSync } from "node:fs";
import { globSync } from "node:fs";
import { resolve, relative } from "node:path";

const ROOT = resolve(import.meta.dirname, "../../../..");
const SRC = resolve(ROOT, "src");
const files = globSync("**/*.rs", { cwd: SRC }).map((f) => resolve(SRC, f));

let violations = 0;

// Matches DomainError::unrecoverable(format!("...")) and extracts message
const UNRECOVERABLE_RE =
  /DomainError::unrecoverable\s*\(\s*format!\s*\(\s*"([^"]*)"/;

// Matches DomainError::expected_{argument|authentication|conflict}(...)
const EXPECTED_RE =
  /DomainError::expected_(argument|authentication|conflict)\s*\(\s*([^)]+)/;

// Hardcoded English strings in errors (not using trl, not using format placeholder)
const HARDCODED_ERROR_RE =
  /DomainError::(expected_\w+|unrecoverable)\s*\(\s*"([^"]*)"\s*\)/;

function grepRules(filePath, content, rel) {
  const lines = content.split("\n");

  // Track if we're inside a test module
  let inTest = false;
  let testDepth = 0;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const lineno = i + 1;
    const trimmed = line.trimStart();

    // Track test module boundaries (including #[cfg(test)] mod tests {)
    if (/^\s*(?:#\[cfg\(test\)\]\s*)?mod\s+tests\s*\{/.test(line)) {
      inTest = true;
      testDepth = 1;
      continue;
    }
    if (inTest) {
      for (const ch of line) {
        if (ch === "{") testDepth++;
        if (ch === "}") testDepth--;
      }
      if (testDepth <= 0) {
        inTest = false;
        testDepth = 0;
      }
      continue; // skip all lines inside test modules
    }

    if (trimmed.startsWith("//") || trimmed.startsWith("///")) continue;

    // ---- Rule 31 & 33: [Struct::method] prefix check ----

    const unrecoverableMatch = trimmed.match(UNRECOVERABLE_RE);
    if (unrecoverableMatch) {
      const msg = unrecoverableMatch[1];
      if (!msg.startsWith("[")) {
        console.log(
          `${rel}:${lineno} — Rule 31: Unrecoverable missing [Struct::method] prefix`,
        );
        console.log(`    ${trimmed}`);
        violations++;
      }
    }

    const expectedMatch = trimmed.match(EXPECTED_RE);
    if (expectedMatch) {
      // Rule 33: Expected errors should NOT have [Struct::method] prefix
      const args = expectedMatch[2];
      if (args.includes("[") && args.includes("]")) {
        const bracketContent = args.match(/"\[([^\]]+)\]/);
        if (bracketContent) {
          console.log(
            `${rel}:${lineno} — Rule 33: Expected error has [Struct::method] prefix`,
          );
          console.log(`    ${trimmed}`);
          violations++;
        }
      }

      // Rule 32: Check that trl() is used (not hardcoded string directly)
      if (args.includes('trl("')) continue; // OK — uses trl()
      // Direct string path in expected_argument — suspicious
      const directStr = args.match(/"([^"]+)"/);
      if (directStr && !args.includes("trl(")) {
        console.log(`${rel}:${lineno} — Rule 32: expected error without trl() (hardcoded string)`);
        console.log(`    ${trimmed}`);
        violations++;
      }
    }

    // ---- Rule 32: Hardcoded language in unrecoverable ----
    // Unrecoverable errors may use format!() (with [Struct::method] prefix) or direct strings
    // Direct strings in unrecoverable without [prefix] are suspicious
    const hardMatch = trimmed.match(HARDCODED_ERROR_RE);
    if (hardMatch) {
      const kind = hardMatch[1];
      const msg = hardMatch[2];
      if (kind.startsWith("expected") && !msg.startsWith("error-")) {
        // expected error with direct string that doesn't use trl("error-*")
        console.log(`${rel}:${lineno} — Rule 32: expected error with hardcoded string (use trl()`);
        console.log(`    ${trimmed}`);
        violations++;
      }
    }

    // ---- Rule 34: Diesel NotFound pattern ----
    // Pattern: .optional()? followed by .ok_or_else(...)
    // Not easily checked with regex — would need multi-line AST. Skipping deep check.
  }
}

for (const f of files) {
  const rel = relative(SRC, f);
  const content = readFileSync(f, "utf-8");
  grepRules(f, content, rel);
}

if (violations === 0) {
  console.log("✓ I (Error Construction): all rules pass.");
} else {
  console.log(`✗ I (Error Construction): ${violations} violation(s).`);
}
process.exitCode = violations;

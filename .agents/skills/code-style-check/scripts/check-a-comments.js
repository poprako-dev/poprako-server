#!/usr/bin/env bun
// check-a-comments.js — Rules 1-3: Comment & language checks
//
//   Rule 1: All comments (//, /* */, ///, //!) must be in English.
//   Rule 2: No Go references (file paths, function names, patterns) in comments.
//   Rule 3: No // comments directly above #[test] / #[tokio::test].

import { readFileSync } from "node:fs";
import { globSync } from "node:fs";
import { resolve, relative } from "node:path";

const ROOT = resolve(import.meta.dirname, "../../../..");
const SRC = resolve(ROOT, "src");
const files = globSync("**/*.rs", { cwd: SRC }).map((f) => resolve(SRC, f));

let violations = 0;

// --- Rule 1: Non-English characters in comments ---
// Simple heuristic: check for CJK codepoints inside comments.
const CJK = /[\u4E00-\u9FFF\u3400-\u4DBF\uF900-\uFAFF\u3040-\u309F\u30A0-\u30FF\uAC00-\uD7AF\uFF01-\uFF60]/;

function grepRules(filePath, content) {
  const rel = relative(SRC, filePath);
  const lines = content.split("\n");

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const lineno = i + 1;
    const trimmed = line.trimStart();

    // ---- Rule 1: Chinese/Japanese/Korean in comments ----
    if (
      (trimmed.startsWith("//") || trimmed.startsWith("/*") || line.includes("//") || line.includes("///") || line.includes("//!")) &&
      CJK.test(line)
    ) {
      console.log(`${rel}:${lineno} — Rule 1: non-English characters in comment`);
      console.log(`    ${trimmed}`);
      violations++;
    }

    // ---- Rule 2: Go references in comments (heuristic) ----
    if (
      (trimmed.startsWith("//") || trimmed.startsWith("/*") || line.includes("///")) &&
      /\.go\b|poprako-s|GORM|gorm\.|internal\/domain\/model\//i.test(line) &&
      !/\.rs\b/.test(line) // skip if it's also referencing a .rs file
    ) {
      console.log(`${rel}:${lineno} — Rule 2: Go reference in comment`);
      console.log(`    ${trimmed}`);
      violations++;
    }

    // ---- Rule 3: // comment directly above #[test] or #[tokio::test] ----
    // Test-case descriptions (// name(target)(positive|negative): ...) are allowed.
    if (trimmed.startsWith("//") && i + 1 < lines.length) {
      const nextLine = lines[i + 1].trimStart();
      if (
        (nextLine === "#[test]" || nextLine.startsWith("#[tokio::test]")) &&
        !trimmed.match(/^\/\/\s+\w+\(\w+\)\(positive\|negative\):/) &&
        !trimmed.startsWith("// ──")
      ) {
        console.log(`${rel}:${lineno} — Rule 3: unsanctioned comment above #[test]`);
        console.log(`    ${trimmed}`);
        console.log(`    ${nextLine}`);
        violations++;
      }
    }
  }
}

for (const f of files) {
  const content = readFileSync(f, "utf-8");
  grepRules(f, content);
}

if (violations === 0) {
  console.log("✓ A (Comments & Language): all rules pass.");
} else {
  console.log(`✗ A (Comments & Language): ${violations} violation(s).`);
}
process.exitCode = violations;

#!/usr/bin/env bun
// check-c-imports.js — Rules 7-11: Import style checks
//
//   Rule  7: No wildcard imports (except super::* in tests, Diesel prelude, serde, schema).
//   Rule  8: No non-leaf braces (use a::b::{c, d} ✓ / use a::{b, c::d} ✗).
//   Rule  9: No bare crate::... paths in code bodies (excl. openapi.rs, forward_ref.rs, macros).
//   Rule 10: API handler uses `use crate::usecase;` with `usecase::` prefix (not enforced by script).
//   Rule 11: Import order: std/external → blank line → crate imports.

import { readFileSync } from "node:fs";
import { globSync } from "node:fs";
import { resolve, relative } from "node:path";

const ROOT = resolve(import.meta.dirname, "../../../..");
const SRC = resolve(ROOT, "src");
const files = globSync("**/*.rs", { cwd: SRC }).map((f) => resolve(SRC, f));

let violations = 0;

// Allowed wildcard import prefixes
const WILDCARD_OK_PREFIXES = [
  "use super::*",
  "use crate::infra::query::schema",
  "use crate::part_impl::repo_rdb::schema",
  "use diesel::prelude::*",
  "use serde::*",
];

function isWildcardOk(trimmed) {
  return WILDCARD_OK_PREFIXES.some((p) => trimmed.startsWith(p));
}

function grepRules(filePath, content, rel) {
  const lines = content.split("\n");

  // Collapse multi-line use statements for Rule 8
  const collapsed = [];
  let acc = "";
  let accStart = 0;
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    if (/^\s*use\s/.test(line)) {
      if (acc !== "") collapsed.push({ text: acc, lineno: accStart + 1 });
      acc = line.trimEnd();
      accStart = i;
    } else if (acc !== "") {
      acc += " " + line.trim();
    }
    if (acc.includes(";")) {
      collapsed.push({ text: acc, lineno: accStart + 1 });
      acc = "";
    }
  }

  // Collect all import lines with their line numbers (for Rule 11)
  const allImportLines = [];
  for (let i = 0; i < lines.length; i++) {
    if (lines[i].trimStart().startsWith("use ")) {
      allImportLines.push({ lineno: i + 1, text: lines[i].trim() });
    }
  }

  // ---- Rule 7: Wildcard imports ----
  for (const imp of allImportLines) {
    if (imp.text.includes("::*") && !isWildcardOk(imp.text)) {
      console.log(`${rel}:${imp.lineno} — Rule 7: disallowed wildcard import`);
      console.log(`    ${imp.text}`);
      violations++;
    }
  }

  // ---- Rule 8: Non-leaf braces ----
  for (const stmt of collapsed) {
    const t = stmt.text;
    if (!t.includes("{") || !t.includes("}")) continue;
    const start = t.indexOf("{");
    const end = t.lastIndexOf("}");
    if (start === -1 || end <= start) continue;
    const inner = t.slice(start + 1, end);
    for (const item of inner.split(",")) {
      const it = item.trim();
      if (it && it.includes("::")) {
        console.log(`${rel}:${stmt.lineno} — Rule 8: non-leaf brace in use statement`);
        console.log(`    ${t}`);
        violations++;
        break;
      }
    }
  }

  // ---- Rule 9: Bare crate:: paths in code bodies ----
  const skipRule9 = rel === "api/http/openapi.rs" || rel === "forward_ref.rs";
  if (!skipRule9) {
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      const trimmed = line.trimStart();
      if (
        trimmed.startsWith("use ") ||
        trimmed.startsWith("///") ||
        trimmed.startsWith("//") ||
        trimmed.startsWith("/*") ||
        trimmed.startsWith("#[") ||
        trimmed.startsWith("*") ||
        trimmed.includes('deserialize_with = "crate::')
      )
        continue;
      if (/crate::/.test(line) && !/\$crate::/.test(line)) {
        console.log(`${rel}:${i + 1} — Rule 9: bare crate:: path in code body`);
        console.log(`    ${line.trim()}`);
        violations++;
      }
    }
  }

  // ---- Rule 11: Import order ----
  if (allImportLines.length >= 2) {
    let sawBlank = false;
    for (let k = 1; k < allImportLines.length; k++) {
      const prev = allImportLines[k - 1];
      const curr = allImportLines[k];
      const hasBlankInBetween = curr.lineno - prev.lineno > 1;
      const isCrate = curr.text.startsWith("use crate::");

      if (hasBlankInBetween) sawBlank = true;

      // crate import found before blank separator
      if (isCrate && !sawBlank && prev.text !== "" && !prev.text.startsWith("use crate::")) {
        // Only flag if this is the first crate import and previous was not crate
        console.log(`${rel}:${curr.lineno} — Rule 11: crate import before blank-line separator`);
        console.log(`    ${curr.text}`);
        violations++;
        sawBlank = true; // don't flag subsequent crate imports
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
  console.log("✓ C (Imports): all rules pass.");
} else {
  console.log(`✗ C (Imports): ${violations} violation(s).`);
}
process.exitCode = violations;

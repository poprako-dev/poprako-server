#!/usr/bin/env bun
// check-b-abbreviations.js — Rules 4-6: Abbreviation checks
//
//   Rule 4: No abbreviations in identifiers (type/field/fn/param/var names).
//   Rule 5: Allowed: harn, conn, txn, id, qid, aggr(suffix), sadmin(field), i18n(module).
//   Rule 6: Forbidden patterns: _val suffix, _filter suffix, cnt, ws/ws_id, desc, upd, cre, resv, new_val, offset_val, limit_val.

import { readFileSync } from "node:fs";
import { globSync } from "node:fs";
import { resolve, relative } from "node:path";

const ROOT = resolve(import.meta.dirname, "../../../..");
const SRC = resolve(ROOT, "src");
const files = globSync("**/*.rs", { cwd: SRC }).map((f) => resolve(SRC, f));

let violations = 0;

// Allowed abbreviation-words in identifiers (full words are fine, these are the short forms we accept).
const ALLOWED = new Set(["harn", "conn", "txn", "id", "qid", "sadmin", "i18n"]);
// "aggr" is allowed only as a suffix (e.g. TeamAggr).
// "sadmin" is allowed only as a field name.
// "i18n" is allowed only as a module reference.

// Forbidden identifier patterns (word-level checks).
const FORBIDDEN = /\b(cnt|ws|desc|upd|cre|resv)\b/;
const FORBIDDEN_SUFFIX = /_(?:val|filter)$/;

// Excluded from being checked: comments, strings, etc. We only check code identifiers.
// Simple heuristic: match against lines that look like Rust code (not //, ///, /*, or string-only).

function grepRules(filePath, content) {
  const rel = relative(SRC, filePath);
  const lines = content.split("\n");

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const lineno = i + 1;
    const trimmed = line.trimStart();

    // Skip comments and string-only lines
    if (trimmed.startsWith("//") || trimmed.startsWith("///")) continue;
    if (trimmed.startsWith("/*") || trimmed.startsWith("*")) continue;
    if (trimmed.startsWith('"')) continue;

    // ---- Rule 4 & 6: Forbidden abbreviations in code identifiers ----
    // Skip string literals — abbreviations inside strings are fine
    if (trimmed.includes('"')) continue;
    if (trimmed.includes("'", 0)) continue;

    // Check _val suffix
    if (/_val\b/.test(trimmed) && !trimmed.startsWith("use ") && !trimmed.startsWith("#[")) {
      console.log(`${rel}:${lineno} — Rule 6: forbidden _val suffix`);
      console.log(`    ${trimmed}`);
      violations++;
    }

    // Check _filter suffix
    if (/_filter\b/.test(trimmed) && !trimmed.startsWith("use ") && !trimmed.startsWith("#[")) {
      console.log(`${rel}:${lineno} — Rule 6: forbidden _filter suffix`);
      console.log(`    ${trimmed}`);
      violations++;
    }

    // Check forbidden standalone words (cnt, ws, desc, upd, cre, resv)
    // These are tricky — "desc" could be a Diesel method call. Filter out .desc() method calls.
    const m = trimmed.match(FORBIDDEN);
    if (m && !trimmed.startsWith("use ") && !trimmed.startsWith("#[")) {
      const word = m[0];
      if (word === "desc" && /\.desc\(\)/.test(trimmed)) continue; // Diesel method call — OK
      if (word === "ws" && /\.ws\(/.test(trimmed)) continue; // method call

      console.log(`${rel}:${lineno} — Rule 6: forbidden abbreviation '${word}'`);
      console.log(`    ${trimmed}`);
      violations++;
    }
  }
}

for (const f of files) {
  const content = readFileSync(f, "utf-8");
  grepRules(f, content);
}

if (violations === 0) {
  console.log("✓ B (Abbreviations): all rules pass.");
} else {
  console.log(`✗ B (Abbreviations): ${violations} violation(s).`);
}
process.exitCode = violations;

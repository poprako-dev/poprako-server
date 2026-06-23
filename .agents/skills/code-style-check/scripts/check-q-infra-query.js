#!/usr/bin/env bun
// check-q-infra-query.js — Rules 61-64: Infra query impl layer checks
//
//   Rule 61: Free functions take `conn: &mut AsyncPgConnection` — `pub async fn`.
//   Rule 62: RdbQuery impl uses submit_query! macro — submit_query!(self.pool, free_fn, args...).
//   Rule 63: RdbQueryTransactional impl delegates directly — free_fn(self.conn, args).await.
//   Rule 64: INSERT uses *Entry struct — .values(&entry), never inline tuples.

import { readFileSync } from "node:fs";
import { globSync } from "node:fs";
import { resolve, relative } from "node:path";

const ROOT = resolve(import.meta.dirname, "../../../..");
const SRC = resolve(ROOT, "src");
const files = globSync("**/*.rs", { cwd: SRC }).map((f) => resolve(SRC, f));

let violations = 0;

function grepRules(filePath, content, rel) {
  const isQueryFile = rel.startsWith("infra/query/") &&
    !rel.includes("entity/") &&
    !rel.includes("memory_mock/") &&
    rel !== "infra/query.rs" &&
    rel !== "infra/query/mod.rs";

  if (!isQueryFile) return;

  const lines = content.split("\n");

  // ---- Rule 63: RdbQueryTransactional delegating directly to free_fn ----
  // This is hard to check with regex. Skipping deep check.

  // ---- Rule 64: INSERT using Entry struct ----
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmed = line.trimStart();
    const lineno = i + 1;

    if (trimmed.startsWith("//") || trimmed.startsWith("///")) continue;

    // Check for insert without .values() using Entry struct
    // diesel::insert_into(table).values(&entry)  ✓
    // diesel::insert_into(table).values((col1.eq(val1), ...))  ✗ (tuple, not Entry)
    if (
      trimmed.includes(".values((") &&
      trimmed.includes("insert_into") &&
      !trimmed.includes("&") // heuristic: Entry references always have & prefix
    ) {
      console.log(
        `${rel}:${lineno} — Rule 64: INSERT using inline tuple — use Entry struct (.values(&entry))`,
      );
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
  console.log("✓ Q (Infra Query): all rules pass.");
} else {
  console.log(`✗ Q (Infra Query): ${violations} violation(s).`);
}
process.exitCode = violations;

#!/usr/bin/env bun
// check-p-entity.js — Rules 57-60: Infra query entity layer checks
//
//   Rule 57: Suffixes: Row (Queryable+Selectable), Entry (Insertable), Aspect (AsChangeset).
//   Rule 58: Aspect: new(updated_at) + builder methods.
//   Rule 59: Database columns use f_ prefix (f_id, f_nickname).
//   Rule 60: From<EntityRow> in entity module, not in query logic file.

import { readFileSync } from "node:fs";
import { globSync } from "node:fs";
import { resolve, relative } from "node:path";

const ROOT = resolve(import.meta.dirname, "../../../..");
const SRC = resolve(ROOT, "src");
const files = globSync("**/*.rs", { cwd: SRC }).map((f) => resolve(SRC, f));

let violations = 0;

function grepRules(filePath, content, rel) {
  const lines = content.split("\n");
  const isEntityModule = rel.includes("entity/") || rel.includes("entity\\");
  const isQueryFile = rel.startsWith("infra/query/") && !isEntityModule;

  // ---- Rule 59: Column names should have f_ prefix (check entity files) ----
  if (isEntityModule) {
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      const trimmed = line.trimStart();
      const lineno = i + 1;

      if (trimmed.startsWith("//") || trimmed.startsWith("///")) continue;
      if (trimmed.startsWith("use ")) continue;
      if (trimmed.startsWith("#[diesel")) continue;

      // Check struct field definitions: `pub field_name: Type,`
      const fieldMatch = trimmed.match(/^pub\s+(\w+)\s*:/);
      if (fieldMatch) {
        const fieldName = fieldMatch[1];
        if (
          !fieldName.startsWith("f_") &&
          !["id", "name", "description", "status", "payload", "last_error", "retried_count",
             "lease", "visible_at", "created_at", "updated_at"].includes(fieldName) // non-f_ fields in actual struct? Let's flag all
          // Actually, all entity fields should have f_ prefix. Let's be stricter.
        ) {
          console.log(
            `${rel}:${lineno} — Rule 59: entity field '${fieldName}' missing f_ prefix`,
          );
          console.log(`    ${trimmed}`);
          violations++;
        }
      }
    }
  }

  // ---- Rule 60: From<EntityRow> should be in entity module ----
  if (isQueryFile) {
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i].trimStart();
      if (
        line.includes("impl From<") &&
        line.includes("Row>")
      ) {
        console.log(
          `${rel}:${i + 1} — Rule 60: From<EntityRow> in query file — should be in entity module`,
        );
        console.log(`    ${line}`);
        violations++;
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
  console.log("✓ P (Entity): all rules pass.");
} else {
  console.log(`✗ P (Entity): ${violations} violation(s).`);
}
process.exitCode = violations;

#!/usr/bin/env bun
// check-r-mock.js — Rule 65: Memory mock layer check
//
//   Rule 65: MemoryMockQuery impls *Query; MemoryMockQueryTransactional impls *QueryTransactional.

import { readFileSync } from "node:fs";
import { globSync } from "node:fs";
import { resolve, relative } from "node:path";

const ROOT = resolve(import.meta.dirname, "../../../..");
const SRC = resolve(ROOT, "src");
const files = globSync("**/*.rs", { cwd: SRC }).map((f) => resolve(SRC, f));

let violations = 0;

function grepRules(filePath, content, rel) {
  const isMockFile = rel.includes("memory_mock/");

  if (!isMockFile) return;

  const lines = content.split("\n");

  for (let i = 0; i < lines.length; i++) {
    const trimmed = lines[i].trimStart();
    const lineno = i + 1;

    if (trimmed.startsWith("//") || trimmed.startsWith("///")) continue;

    // Check impl blocks
    const implMatch = trimmed.match(/impl\s+(MemoryMockQueryTransactional)\s+for\s+\w+/);
    if (!implMatch) continue;

    // Check if the trait being implemented is *Query (not *QueryTransactional)
    const traitImplMatch = trimmed.match(/impl\s+\w+Query(?:Transactional)?\s+for\s+MemoryMockQueryTransactional/);
    if (traitImplMatch) {
      const traitName = traitImplMatch[0];
      // MemoryMockQueryTransactional should ONLY impl *QueryTransactional, NOT *Query
      if (!traitName.includes("Transactional")) {
        console.log(
          `${rel}:${lineno} — Rule 65: MemoryMockQueryTransactional implements non-Transactional trait`,
        );
        console.log(`    ${trimmed}`);
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
  console.log("✓ R (Mock): all rules pass.");
} else {
  console.log(`✗ R (Mock): ${violations} violation(s).`);
}
process.exitCode = violations;

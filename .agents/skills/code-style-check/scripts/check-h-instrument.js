#!/usr/bin/env bun
// check-h-instrument.js — Rules 23-27: #[instrument] placement checks
//
//   Rule 23: No #[instrument] on constructors (new, generate_id, From, Default).
//   Rule 24: No #[instrument] on domain model (src/domain/).
//   Rule 25: No #[instrument] on harness (src/harness.rs).
//   Rule 26: No #[instrument] on QueryTransactional impl (pure delegation).
//   Rule 27: Allowed on: usecase, infra query free fns, infra Query impl, infra external, API handlers.

import { readFileSync } from "node:fs";
import { globSync } from "node:fs";
import { resolve, relative } from "node:path";

const ROOT = resolve(import.meta.dirname, "../../../..");
const SRC = resolve(ROOT, "src");
const files = globSync("**/*.rs", { cwd: SRC }).map((f) => resolve(SRC, f));

let violations = 0;

const CONSTRUCTOR_NAMES = ["new", "generate_id", "from_form", "from_aggr"];

function grepRules(filePath, content, rel) {
  const lines = content.split("\n");

  // ---- Rule 24: Whole-file check for domain/ ----
  const isDomain = rel.startsWith("domain/") || rel.startsWith("domain\\");
  // ---- Rule 25: harness.rs ----
  const isHarness = rel === "harness.rs";

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const lineno = i + 1;
    const trimmed = line.trimStart();

    // Check if this line has #[instrument] — look at current line as well as
    // multi-line #[instrument(...)] attributes
    const hasInstrument = trimmed.startsWith("#[instrument");

    if (!hasInstrument) continue;

    // ---- Rule 24: domain/ folder ----
    if (isDomain) {
      console.log(`${rel}:${lineno} — Rule 24: #[instrument] on domain model code`);
      console.log(`    ${trimmed}`);
      violations++;
      continue;
    }

    // ---- Rule 25: harness.rs ----
    if (isHarness) {
      console.log(`${rel}:${lineno} — Rule 25: #[instrument] on harness`);
      console.log(`    ${trimmed}`);
      violations++;
      continue;
    }

    // ---- Rule 23: Constructor check ----
    // Look ahead to find the function name
    for (let j = i + 1; j < Math.min(i + 5, lines.length); j++) {
      const nextLine = lines[j].trimStart();
      const fnMatch = nextLine.match(/(?:pub\s+)?(?:async\s+)?fn\s+(\w+)/);
      if (fnMatch) {
        const fnName = fnMatch[1];
        if (
          CONSTRUCTOR_NAMES.includes(fnName) ||
          nextLine.includes("impl From<") ||
          nextLine.includes("impl Default")
        ) {
          console.log(
            `${rel}:${lineno} — Rule 23: #[instrument] on constructor/From/Default '${fnName}'`,
          );
          console.log(`    ${trimmed}`);
          violations++;
        }
        break;
      }
      // Also check for impl blocks
      if (nextLine.includes("impl ") && (nextLine.includes("From") || nextLine.includes("Default"))) {
        console.log(`${rel}:${lineno} — Rule 23: #[instrument] on From/Default impl`);
        console.log(`    ${trimmed}`);
        violations++;
        break;
      }
    }
  }

  // ---- Rule 26: QueryTransactional impl with instrument ----
  // Scan for impl *QueryTransactional blocks and check their methods for #[instrument]
  let inQueryTxImpl = false;
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const lineno = i + 1;
    const trimmed = line.trimStart();

    if (trimmed.startsWith("impl") && trimmed.includes("QueryTransactional")) {
      inQueryTxImpl = true;
    }
    if (inQueryTxImpl && trimmed === "}") {
      inQueryTxImpl = false;
    }

    if (
      inQueryTxImpl &&
      (trimmed.startsWith("#[instrument") || trimmed === "#[instrument]")
    ) {
      console.log(`${rel}:${lineno} — Rule 26: #[instrument] on QueryTransactional impl`);
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
  console.log("✓ H (#[instrument] Placement): all rules pass.");
} else {
  console.log(`✗ H (#[instrument] Placement): ${violations} violation(s).`);
}
process.exitCode = violations;

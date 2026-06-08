#!/usr/bin/env bun
// check-t-tests.js — Rules 70-71: Test module checks
//
//   Rule 70: use super::* must be first import in every test module (after test-case descriptions).
//   Rule 71: Test-case descriptions before imports: // name(target)(positive|negative): description.

import { readFileSync } from "node:fs";
import { globSync } from "node:fs";
import { resolve, relative } from "node:path";

const ROOT = resolve(import.meta.dirname, "../../../..");
const SRC = resolve(ROOT, "src");
const files = globSync("**/*.rs", { cwd: SRC }).map((f) => resolve(SRC, f));

let violations = 0;

function grepRules(filePath, content, rel) {
  const lines = content.split("\n");

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmed = line.trimStart();

    // Detect test module start: `mod tests {`
    const testModMatch = trimmed.match(/^(?:#\[cfg\(test\)\]\s*)?mod\s+tests\s*\{/);
    if (testModMatch) {
      // We're at a test module. Now scan forward for `use` statements.
      // `use super::*` should be the FIRST import.
      let firstUse = null;
      let firstUseLine = 0;
      let sawSuperStar = false;
      let hasAnyImport = false;

      for (let j = i + 1; j < Math.min(i + 60, lines.length); j++) {
        const tline = lines[j].trim();
        const tlineno = j + 1;

        // Skip test-case description comments (they're allowed above imports)
        if (/^\/\/\s+\w+\(\w+\)\(positive\|negative\)/.test(tline)) continue;
        // Skip blank lines and section comments
        if (tline === "" || tline.startsWith("// ──")) continue;
        if (tline === "}" || tline.startsWith("}")) break;

        if (tline.startsWith("use ")) {
          hasAnyImport = true;
          if (firstUse === null) {
            firstUse = tline;
            firstUseLine = tlineno;
          }

          // Rule 70: use super::* must be first
          if (tline === "use super::*;") {
            sawSuperStar = true;
            if (firstUse !== null && firstUse !== "use super::*;") {
              console.log(
                `${rel}:${firstUseLine} — Rule 70: use super::* should be the FIRST import in test module`,
              );
              console.log(`    ${firstUse} (found before super::*)`);
              violations++;
            }
            break;
          }
        }
      }
    }
  }

  // ---- Rule 71: Test-case descriptions ----
  // Check comments above #[test] lines match the format
  for (let i = 0; i < lines.length - 1; i++) {
    const line = lines[i];
    const nextLine = lines[i + 1].trimStart();
    const trimmed = line.trimStart();
    const lineno = i + 1;

    if (
      (nextLine === "#[test]" || nextLine.startsWith("#[tokio::test]")) &&
      trimmed.startsWith("//") &&
      !trimmed.startsWith("// ──")
    ) {
      // Check if it matches the test-case description format
      const descMatch = trimmed.match(/^\s*\/\/\s+(\w+)\((\w+)\)\((positive|negative)\)\s*:\s*/);
      if (!descMatch) {
        console.log(
          `${rel}:${lineno} — Rule 71: comment above #[test] not in test-case description format`,
        );
        console.log(`    ${trimmed}`);
        console.log(`    Expected: // name(target)(positive|negative): description`);
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
  console.log("✓ T (Tests): all rules pass.");
} else {
  console.log(`✗ T (Tests): ${violations} violation(s).`);
}
process.exitCode = violations;

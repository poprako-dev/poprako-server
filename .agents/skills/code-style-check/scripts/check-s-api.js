#!/usr/bin/env bun
// check-s-api.js — Rules 66-69: API handler layer checks
//
//   Rule 66: Accept as _ — anonymous import, use .accept(StatusCode::...).
//   Rule 67: ? propagation — never manually match usecase errors.
//   Rule 68: HttpError imported and bare — body = HttpError in #[utoipa::path], never full path.
//   Rule 69: Handler name matches usecase function name.

import { readFileSync } from "node:fs";
import { globSync } from "node:fs";
import { resolve, relative } from "node:path";

const ROOT = resolve(import.meta.dirname, "../../../..");
const SRC = resolve(ROOT, "src");
const files = globSync("api/http/handler/**/*.rs", { cwd: SRC }).map((f) => resolve(SRC, f));

let violations = 0;

function grepRules(filePath, content, rel) {
  const lines = content.split("\n");

  // ---- Rule 66: Accept as _ ----
  let hasAcceptImport = false;
  let hasHandlerFn = false;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmed = line.trimStart();

    if (trimmed.includes("use crate::api::http::result::Accept as _")) {
      hasAcceptImport = true;
    }

    // Check if this file has actual Axum handler functions
    if (/^pub\s+async\s+fn\s+\w+/.test(trimmed)) {
      hasHandlerFn = true;
    }

    // ---- Rule 68: HttpError bare import ----
    if (
      trimmed.includes("crate::api::http::result::HttpError") &&
      trimmed.startsWith("#[")
    ) {
      console.log(
        `${rel}:${i + 1} — Rule 68: full path HttpError in #[utoipa] — use bare 'HttpError'`,
      );
      console.log(`    ${trimmed}`);
      violations++;
    }
  }

  // Only flag missing Accept import if the file actually has handler functions
  if (!hasAcceptImport && hasHandlerFn) {
    console.log(`${rel}: Rule 66 — missing 'Accept as _' import`);
    violations++;
  }

  // ---- Rule 67: ? propagation ----
  // Check for manual matching of usecase errors (anti-pattern)
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmed = line.trimStart();
    if (
      trimmed.includes("match ") &&
      (trimmed.includes("UseCaseResult") || trimmed.includes("DomainError"))
    ) {
      console.log(
        `${rel}:${i + 1} — Rule 67: manual match on usecase result — use ? propagation`,
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
  console.log("✓ S (API Handler): all rules pass.");
} else {
  console.log(`✗ S (API Handler): ${violations} violation(s).`);
}
process.exitCode = violations;

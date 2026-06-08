#!/usr/bin/env bun
// check-d-macros.js — Rules 12-14: Macro usage checks
//
//   Rule 12: Attribute macros — use import + bare name (e.g. `use tracing::instrument;` → `#[instrument]`)
//   Rule 13: Derive macros  — use import + bare name (e.g. `use serde::Serialize;` → `#[derive(Serialize)]`)
//   Rule 14: tracing event macros — fully-qualified at call site (tracing::error!(...)), never bare-imported

import { readFileSync } from "node:fs";
import { globSync } from "node:fs";
import { resolve, relative } from "node:path";

const ROOT = resolve(import.meta.dirname, "../../../..");
const SRC = resolve(ROOT, "src");
const files = globSync("**/*.rs", { cwd: SRC }).map((f) => resolve(SRC, f));

let violations = 0;

// tracing event macros that must be fully-qualified
const TRACING_EVENTS = /(?<!\w)(error!\s*\(|warn!\s*\(|info!\s*\(|debug!\s*\(|trace!\s*\()/;

function grepRules(filePath, content, rel) {
  const lines = content.split("\n");

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const lineno = i + 1;
    const trimmed = line.trimStart();

    if (trimmed.startsWith("//") || trimmed.startsWith("///") || trimmed.startsWith("/*"))
      continue;

    // ---- Rule 12: #[tracing::instrument] (fully-qualified in attribute) ----
    if (trimmed.includes("#[tracing::instrument")) {
      console.log(`${rel}:${lineno} — Rule 12: #[tracing::instrument] should be #[instrument]`);
      console.log(`    ${trimmed}`);
      violations++;
    }

    // ---- Rule 13: #[derive(tracing::...)] or #[derive(serde::...)] ----
    // serde::Deserialize is explicitly allowed in derive macros.
    // utoipa::IntoParams, utoipa::ToSchema, etc. are also allowed in path attrs.
    const allowedDerive = ["serde::Serialize", "serde::Deserialize", "utoipa::IntoParams", "utoipa::ToSchema"];
    if (
      trimmed.includes("derive(") &&
      /derive\(.*::/.test(trimmed) &&
      !allowedDerive.some((ad) => trimmed.includes(ad))
    ) {
      console.log(`${rel}:${lineno} — Rule 13: #[derive] should use bare names`);
      console.log(`    ${trimmed}`);
      violations++;
    }

    // ---- Rule 14: bare tracing event macros (not tracing::) ----
    // These macros must be used as tracing::error!(), not error!()
    if (
      !trimmed.startsWith("use ") &&
      TRACING_EVENTS.test(line) &&
      !line.includes("tracing::")
    ) {
      console.log(`${rel}:${lineno} — Rule 14: bare tracing event macro (use tracing::macro_name!)`);
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
  console.log("✓ D (Macros): all rules pass.");
} else {
  console.log(`✗ D (Macros): ${violations} violation(s).`);
}
process.exitCode = violations;

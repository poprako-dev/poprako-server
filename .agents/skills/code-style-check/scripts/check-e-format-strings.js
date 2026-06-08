#!/usr/bin/env bun
// check-e-format-strings.js — Rules 15-16: Format string checks
//
//   Rule 15: No inline captures — format!("{ident}") ✗, format!("{}", ident) ✓
//   Rule 16: tracing fields — tracing::error!(key = %val, "msg") ✓, no interpolated values in msg string

import { readFileSync } from "node:fs";
import { globSync } from "node:fs";
import { resolve, relative } from "node:path";

const ROOT = resolve(import.meta.dirname, "../../../..");
const SRC = resolve(ROOT, "src");
const files = globSync("**/*.rs", { cwd: SRC }).map((f) => resolve(SRC, f));

let violations = 0;

// Inline capture: {ident} or {ident:?} etc. (not {0}, not {{escaped}})
const INLINE_CAPTURE = /\{(?!\d+\}?)([a-z_][a-z0-9_]*)(?::[?])?\}/;

// tracing event check: any interpolation in the format string (e.g. "... {value} ...")
// tracing event macros: tracing::error!, tracing::warn!, tracing::info!, tracing::debug!, tracing::trace!

function grepRules(filePath, content, rel) {
  const lines = content.split("\n");

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const lineno = i + 1;
    const trimmed = line.trimStart();

    // Skip comments, utoipa paths, route paths
    if (trimmed.startsWith("//") || trimmed.startsWith("///")) continue;
    if (trimmed.includes("utoipa")) continue;
    if (trimmed.includes(".route(")) continue;
    if (trimmed.startsWith("path =")) continue;

    // ---- Rule 15: Inline format captures ----
    // Check format!, println!, write!, panic!, format_args!, etc.
    if (/^\s*(format!|println!|print!|write!|writeln!|panic!|eprintln!|eprint!|format_args!)/.test(line)) {
      // Extract the format string portion
      const m = line.match(/(?:format!|println!|print!|write!|writeln!|panic!|eprintln!|eprint!|format_args!)\s*\(\s*"([^"]*)"/);
      if (m) {
        const fmtStr = m[1];
        if (INLINE_CAPTURE.test(fmtStr)) {
          console.log(`${rel}:${lineno} — Rule 15: inline capture in format string`);
          console.log(`    ${trimmed}`);
          violations++;
        }
      }
    }

    // ---- Rule 16: tracing fields (no interpolation in message) ----
    if (/tracing::(error|warn|info|debug|trace)!/ .test(line)) {
      // The format string should not contain {variable} interpolations — only {} placeholders
      // Get the trailing message string: it's the last positional arg after key=value pairs
      const macroMatch = line.match(/tracing::(?:error|warn|info|debug|trace)!\s*\(([^)]*)\)/);
      if (macroMatch) {
        const args = macroMatch[1];
        // Find the message string — it's the last unnamed string literal
        const msgMatch = args.match(/,\s*"([^"]*)"\s*\)?\s*$/);
        if (msgMatch) {
          const msg = msgMatch[1];
          if (INLINE_CAPTURE.test(msg)) {
            console.log(`${rel}:${lineno} — Rule 16: interpolated value in tracing message`);
            console.log(`    ${trimmed}`);
            violations++;
          }
        }
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
  console.log("✓ E (Format Strings): all rules pass.");
} else {
  console.log(`✗ E (Format Strings): ${violations} violation(s).`);
}
process.exitCode = violations;

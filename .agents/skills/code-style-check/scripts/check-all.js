#!/usr/bin/env bun
// check-all.js — Run all code style check scripts and aggregate results.
//
// Usage:
//   bun run check-all.js            # run all checks
//   bun run check-all.js --fix      # run all checks, show fix suggestions
//   bun run check-all.js A D M      # run only specific modules

import { spawnSync } from "node:child_process";
import { resolve } from "node:path";
import { argv } from "node:process";

const SCRIPT_DIR = resolve(import.meta.dirname);

const ALL_MODULES = [
  { id: "A", name: "Comments & Language", script: "check-a-comments.js" },
  { id: "B", name: "Abbreviations", script: "check-b-abbreviations.js" },
  { id: "C", name: "Imports", script: "check-c-imports.js" },
  { id: "D", name: "Macros", script: "check-d-macros.js" },
  { id: "E", name: "Format Strings", script: "check-e-format-strings.js" },
  { id: "F", name: "Visibility", script: "check-f-visibility.js" },
  { id: "G", name: "Ownership & Construction", script: "check-g-ownership.js" },
  { id: "H", name: "#[instrument] Placement", script: "check-h-instrument.js" },
  { id: "I", name: "Error Construction", script: "check-i-errors.js" },
  { id: "J", name: "i18n", script: "check-j-i18n.js" },
  { id: "K", name: "UFCS", script: "check-k-ufcs.js" },
  { id: "L", name: "Usecase Layer", script: "check-l-usecase.js" },
  { id: "M", name: "Type Annotations / Turbofish", script: "check-m-turbofish.js" },
  { id: "N", name: "Domain Aggregates", script: "check-n-aggregates.js" },
  { id: "O", name: "Domain Query Trait", script: "check-o-query-trait.js" },
  { id: "P", name: "Infra Entity", script: "check-p-entity.js" },
  { id: "Q", name: "Infra Query Impl", script: "check-q-infra-query.js" },
  { id: "R", name: "Memory Mock", script: "check-r-mock.js" },
  { id: "S", name: "API Handler", script: "check-s-api.js" },
  { id: "T", name: "Test Modules", script: "check-t-tests.js" },
];

// Filter modules by command-line args
let modules = ALL_MODULES;
const args = argv.slice(2).filter((a) => a !== "--fix");
if (args.length > 0) {
  const ids = args.map((a) => a.toUpperCase());
  modules = ALL_MODULES.filter((m) => ids.includes(m.id));
  if (modules.length === 0) {
    console.error(`Unknown module IDs: ${args.join(", ")}`);
    console.error("Available: A B C D E F G H I J K L M N O P Q R S T");
    process.exit(1);
  }
}

console.log("╔══════════════════════════════════════════╗");
console.log("║   poprako-r Code Style Check — 70 Rules  ║");
console.log("╚══════════════════════════════════════════╝");
console.log();

let totalPassed = 0;
let totalFailed = 0;
let totalViolations = 0;

for (const mod of modules) {
  const scriptPath = resolve(SCRIPT_DIR, mod.script);
  const result = spawnSync("bun", ["run", scriptPath], {
    stdio: "pipe",
    encoding: "utf-8",
    timeout: 30_000,
  });

  const output = result.stdout.trim();
  const stderr = result.stderr.trim();

  if (result.status === 0) {
    console.log(`  ✅ ${mod.id.padEnd(2)} ${mod.name}: PASS`);
    totalPassed++;
  } else {
    const violationCount = result.status;
    totalViolations += violationCount;
    console.log(`  ❌ ${mod.id.padEnd(2)} ${mod.name}: ${violationCount} violation(s)`);
    if (output) {
      // Print the first few violation lines
      const lines = output.split("\n").filter((l) => !l.startsWith("✗") && !l.startsWith("✓"));
      for (const l of lines.slice(0, 5)) {
        console.log(`     ${l}`);
      }
      if (lines.length > 5) {
        console.log(`     ... and ${lines.length - 5} more violations`);
      }
    }
    totalFailed++;
  }
}

console.log();
console.log("──────────────────────────────────────────");
console.log(
  `  Total: ${totalPassed} passed, ${totalFailed} failed, ${totalViolations} violations`,
);

if (totalFailed > 0) {
  console.log();
  console.log("  Run individual scripts for full details:");
  for (const mod of modules) {
    const scriptPath = resolve(SCRIPT_DIR, mod.script);
    console.log(`    bun run ${scriptPath}`);
  }
}

process.exit(totalFailed > 0 ? 1 : 0);

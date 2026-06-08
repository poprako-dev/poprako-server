#!/usr/bin/env bun
// check-j-i18n.js — Rules 35-36: i18n checks
//
//   Rule 35: Keys in kebab-case with error- prefix: "error-user-not-found".
//   Rule 36: Every trl("error-xxx") key must exist in BOTH locales/en-US/main.ftl and locales/zh-CN/main.ftl.

import { readFileSync, existsSync } from "node:fs";
import { globSync } from "node:fs";
import { resolve, relative, dirname } from "node:path";

const ROOT = resolve(import.meta.dirname, "../../../../");
const SRC = resolve(ROOT, "src");
const EN_FTL = resolve(ROOT, "poprako-util/locales/en-US/main.ftl");
const ZH_FTL = resolve(ROOT, "poprako-util/locales/zh-CN/main.ftl");

let violations = 0;

// Extract trl("...") keys
const TRL_KEY_RE = /trl\s*\(\s*"([^"]+)"/g;

function extractTrlKeys(filePath) {
  const content = readFileSync(filePath, "utf-8");
  const keys = [];
  let m;
  while ((m = TRL_KEY_RE.exec(content)) !== null) {
    keys.push(m[1]);
  }
  return keys;
}

function extractFtlKeys(ftlPath) {
  if (!existsSync(ftlPath)) {
    console.error(`WARNING: FTL file not found: ${ftlPath}`);
    return new Map();
  }
  const content = readFileSync(ftlPath, "utf-8");
  // FTL keys are at the start of a line followed by = sign
  const map = new Map();
  const keyRe = /^(\S+)\s*=\s*(.*)/gm;
  let m;
  while ((m = keyRe.exec(content)) !== null) {
    map.set(m[1], m[2].trim());
  }
  return map;
}

// Collect all trl keys from source
const allKeys = new Set();
const files = globSync("**/*.rs", { cwd: SRC }).map((f) => resolve(SRC, f));
for (const f of files) {
  const keys = extractTrlKeys(f);
  for (const k of keys) allKeys.add(k);
}

// ---- Rule 35: kebab-case with error- prefix ----
for (const key of allKeys) {
  // Skip keys that are clearly format patterns (may contain {})
  if (key.includes("{")) continue;

  if (!/^error-[a-z0-9]+(-[a-z0-9]+)*$/.test(key)) {
    console.log(`Rule 35: key "${key}" is not in kebab-case with error- prefix`);
    violations++;
  }
}

// ---- Rule 36: keys must exist in both locale files ----
const enKeys = extractFtlKeys(EN_FTL);
const zhKeys = extractFtlKeys(ZH_FTL);

for (const key of allKeys) {
  if (key.includes("{")) continue; // format patterns
  if (!enKeys.has(key)) {
    console.log(`Rule 36: key "${key}" missing in en-US/main.ftl`);
    violations++;
  }
  if (!zhKeys.has(key)) {
    console.log(`Rule 36: key "${key}" missing in zh-CN/main.ftl`);
    violations++;
  }
}

if (violations === 0) {
  console.log("✓ J (i18n): all rules pass.");
} else {
  console.log(`✗ J (i18n): ${violations} violation(s).`);
}
process.exitCode = violations;

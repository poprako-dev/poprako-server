#!/usr/bin/env bun
// check-m-turbofish.js — Rules 43-44: Type annotation & turbofish checks
//
//   Rule 43: let annotation over turbofish. e.g. let row: UserRow = ... ✓ ; .first::<UserRow>(conn) ✗
//   Rule 44: No turbofish anywhere in Diesel query chains — .first(), .get_result(), .load(), .execute()
//            never carry type parameters. Also catch .bind::<SqlType>(), .collect::<T>() turbofish.

import { readFileSync } from "node:fs";
import { globSync } from "node:fs";
import { resolve, relative } from "node:path";

const ROOT = resolve(import.meta.dirname, "../../../..");
const SRC = resolve(ROOT, "src");
const files = globSync("**/*.rs", { cwd: SRC }).map((f) => resolve(SRC, f));

let violations = 0;

// Turbofish on any method call in a chain: .method::<Type>(...)
// Covers: .first::<T>(), .get_result::<T>(), .load::<T>(), .execute::<T>(),
//          .bind::<SqlType, _>(), .collect::<T>(), .into::<T>(), etc.
const METHOD_TURBOFISH = /\.(\w+)::<[^>]+>\s*\(/;

// Turbofish on function calls: function_name::<Type>(...)
const FN_TURBOFISH = /(?<!\.)([a-z_]\w*)::<[^>]+>\s*\(/;

// Excluded: Into::<u32>::into, Err::<...>, transmute::<...>, ConnectionManager::<...>,
//            make_service_with_connect_info::<...>, bind::<...> (Diesel API), etc.
const EXCLUDED_NAMES = new Set([
  "into", "transmute", "bind", // Diesel/domain API
]);

function grepRules(filePath, content, rel) {
  const lines = content.split("\n");

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const lineno = i + 1;
    const trimmed = line.trimStart();

    if (trimmed.startsWith("//") || trimmed.startsWith("///") || trimmed.startsWith("/*"))
      continue;
    if (trimmed.startsWith("use ")) continue;
    if (trimmed.startsWith("#[")) continue;
    if (trimmed.startsWith("pub mod")) continue;

    // Skip: Into::<u32>::into(...), Err::<(), DomainError>(...), transmute::<...>(...),
    //       AsyncDieselConnectionManager::<...>::new(...), collect::<...>,
    //       make_service_with_connect_info::<...>()

    // ---- Rule 44: Turbofish on Diesel-specific methods ----
    const dieselMethods = [
      "first::<",
      "get_result::<",
      "load::<",
      "execute::<",
      "bind::<",
    ];
    for (const dm of dieselMethods) {
      if (trimmed.includes(`.${dm}`)) {
        console.log(
          `${rel}:${lineno} — Rule 44: turbofish on Diesel method (.${dm})`,
        );
        console.log(`    ${trimmed}`);
        violations++;
      }
    }

    // ---- Rule 43: General turbofish — collect::<T>(), from_value::<T>(), etc. ----
    const mtMatch = trimmed.match(METHOD_TURBOFISH);
    if (mtMatch) {
      const method = mtMatch[1];
      // Skip Diesel methods already checked above
      if (["first", "get_result", "load", "execute", "bind"].includes(method))
        continue;
      // Skip Into::<u32>::into — it's a turbofish on the trait, not method
      if (method === "into" && trimmed.includes("Into::"))
        continue;
      // Skip Axum framework API requirement
      if (method === "into_make_service_with_connect_info")
        continue;

      console.log(
        `${rel}:${lineno} — Rule 43: method turbofish .${method}::<...>() — use let annotation`,
      );
      console.log(`    ${trimmed}`);
      violations++;
    }

    // ---- Rule 43: Function-level turbofish (e.g. serde_json::from_value::<T>(...)) ----
    const ftMatch = trimmed.match(FN_TURBOFISH);
    if (ftMatch) {
      const fnName = ftMatch[1];
      // Skip known allowed patterns
      if (EXCLUDED_NAMES.has(fnName)) continue;
      if (fnName === "collect") continue; // method
      if (fnName === "new") continue; // Constructor — e.g. ConnManager::new
      if (fnName === "into") continue;
      if (fnName === "transmute") continue;
      if (fnName === "from_value") {
        console.log(
          `${rel}:${lineno} — Rule 43: fn turbofish ${fnName}::<...>() — use let annotation`,
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
  console.log("✓ M (Type Annotations / Turbofish): all rules pass.");
} else {
  console.log(`✗ M (Type Annotations / Turbofish): ${violations} violation(s).`);
}
process.exitCode = violations;

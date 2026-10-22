# Rust linter migration assessment

Date: 2026-10-22. Scope: equivalent libraries and migration feasibility; no implementation or Rust performance claim.

## Recommendation

A Rust migration is technically viable. Preserve Tree-sitter first, then move rule traversal and indexing into Rust, and parallelize independent files. This is an engineering recommendation, not a measured speedup. The strongest compatibility path is the official Rust `tree-sitter` binding with the existing `tree-sitter-rust` grammar, rather than replacing the syntax model at the same time.

## Local scope and evidence

Static inspection during this assessment found 16 shared checker modules, 15 enabled by the current configuration; extra has 18 `check.py` paths representing 16 implementations after two alias wrappers. The shared package contains 23 Python files / 10,483 lines; extra contains 25 Python files / 5,377 lines. These counts include embedded self-tests and supporting code, and are not an estimate of exclusive production logic. [Shared package](../linters/rust_style_lint/), [extra](../linters-extra/)

The largest shared modules are `spacing_style.py` (1,741 lines), `use_style.py` (1,548), `forbidden_identifiers.py` (1,388), and `module_dependency.py` (1,136). This is a substantial rule migration rather than a parser-binding swap. [Shared checkers](../linters/rust_style_lint/checkers/)

The previously observed full invocation took 14.50 seconds; prior shared timing was 9.578 seconds and extra rule timings totaled approximately 4.6 seconds. These measurements were not a synchronized benchmark of a stable working tree: business code changed concurrently, and the full run reported an import-merge diagnostic. Treat them as evidence of remaining cost, not an exact baseline or a Rust result. No new full-suite performance run or Rust build was performed for this assessment.

Cross-file work includes the shared module dependency graph/SCC analysis and extra module aliases, use-case rules, and associated-function checks. They require indexing or graph resolution before independent checks can finish. Existing original-source and production-masked parse views must remain distinct; the target is one parse per distinct required view, not one tree shared across incompatible rule inputs. [Module dependency checker](../linters/rust_style_lint/checkers/module_dependency.py), [extra analysis session](../linters-extra/analysis_session.py)

Regex migration has concrete blockers: shared `module_dependency.py:26` uses negative lookbehind, and extra `oper-inline/check.py:177` uses negative lookahead. Rewrite these predicates using boundary checks or syntax nodes. The `\\1_\\2` spelling in `forbidden_identifiers.py:122` is a replacement-template reference, not evidence of pattern backreferences; translate replacement syntax separately. [Module dependency checker](../linters/rust_style_lint/checkers/module_dependency.py), [oper-inline checker](../linters-extra/oper-inline/check.py), [identifier checker](../linters/rust_style_lint/checkers/forbidden_identifiers.py)

## Equivalent libraries

| Current responsibility | Rust candidate | Compatibility assessment |
| --- | --- | --- |
| Python Tree-sitter parser and nodes | `tree-sitter` | Direct binding to the same parser family; exposes fields, children, siblings, byte ranges, cursors, and error nodes. Start with engine version `0.25.0` and verify the Python distribution's bundled engine revision before claiming exact identity. [Pinned Rust source](https://raw.githubusercontent.com/tree-sitter/tree-sitter/v0.25.0/lib/binding_rust/lib.rs) |
| Rust grammar `tree-sitter-rust==0.23.3` | `tree-sitter-rust = "=0.23.3"` | The same version exists as a Rust crate, exposing `LANGUAGE` and `NODE_TYPES`. Preserve the grammar version and compare generated grammar/node definitions before upgrading. [Pinned grammar source](https://docs.rs/tree-sitter-rust/0.23.3/src/tree_sitter_rust/lib.rs.html) |
| CPU worker scheduling | `rayon` | Parallel iterators and job-local state fit file analysis. `map_init` creates state as needed for job groups; it does not promise one initialization per OS worker. [ParallelIterator](https://docs.rs/rayon/latest/rayon/iter/trait.ParallelIterator.html#method.map_init) |
| Recursive file discovery | `walkdir` | Supports pruning before descending and explicit symlink policy. Preserve existing exclusions and traversal errors. [walkdir](https://docs.rs/walkdir/latest/walkdir/) |
| Git-style ignore handling, if required | `ignore` | Supports parallel traversal, but defaults include `.ignore`, Git ignores, and global ignores. These defaults must not silently change the checked file set. [WalkBuilder](https://docs.rs/ignore/latest/ignore/struct.WalkBuilder.html) |
| TOML parsing | `toml` with `serde` | Provides TOML parsing/deserialization; preserve existing configuration defaults, accepted syntax, and error policy separately. [toml](https://docs.rs/toml/latest/toml/) |
| Python `re` | `regex`, plus explicit scanner logic where necessary | Not a drop-in replacement: lookaround/backreferences are unsupported. Unicode `\w` includes marks and join controls, whereas Python uses alphanumeric characters plus underscore; consequently `\b` can also differ. Audit patterns rather than mechanically translating them. [regex](https://docs.rs/regex/latest/regex/), [Python re](https://docs.python.org/3.13/library/re.html) |

## What Rust changes

The current Python binding already calls the native C Tree-sitter parser. In the pinned `v0.25.0` buffer path, `parser_parse` calls `ts_parser_parse_string_encoding` directly; there is no GIL-release guard around that call. Moving languages therefore targets Python traversal, wrapper access, allocations, indexing, and orchestration as well as enabling parallel analysis. It does not replace a parser written in Python. [Pinned Python binding source](https://raw.githubusercontent.com/tree-sitter/py-tree-sitter/v0.25.0/tree_sitter/binding/parser.c)

Ordinary CPython threads do not run Python bytecode simultaneously because of the GIL. Python 3.13 free-threaded builds exist but are not the default; this is not grounds to assume the installed interpreter or extension is free-threaded. [Python threading documentation](https://docs.python.org/3.13/library/threading.html#gil-and-performance-considerations)

In Tree-sitter `0.25.0`, `Parser`, `Tree`, and `Node` implement `Send` and `Sync`, but parsing takes `&mut Parser`, and `Node<'tree>` borrows its tree. Recommended design: independent parser state per Rayon job, immutable trees after parsing, and owned compact facts for cross-file analysis. Avoid storing borrowed nodes inside a struct that also owns their tree. Preserve byte offsets, named versus unnamed nodes, and error/missing-node handling. [Pinned Rust binding source](https://raw.githubusercontent.com/tree-sitter/tree-sitter/v0.25.0/lib/binding_rust/lib.rs)

Use one configurable thread pool across shared and extra rules. Rayon supports an explicit thread count. Start performance comparison with one, two, and four workers; choose a default from measurements on the target Mac. More active cores may increase instantaneous power; shorter elapsed time alone does not prove lower energy or temperature. The latter is an assessment limitation, not a measured thermal outcome. [Rayon ThreadPoolBuilder](https://docs.rs/rayon/latest/rayon/struct.ThreadPoolBuilder.html#method.num_threads)

## Why not change parsers during the migration?

- `syn` parses Rust tokens into a typed syntax tree and is oriented toward procedural macros. It is a viable choice for new token/AST analyses, but adopting its different tree model would require rewriting existing Tree-sitter rule contracts. It is not a compatibility shortcut. [syn](https://docs.rs/syn/latest/syn/)
- `ra_ap_syntax` offers full-fidelity syntax trees, error recovery, and typed AST wrappers. It is a credible alternative when that representation is desired, but its API and syntax kinds differ from Tree-sitter. The syntax crate itself should not be mistaken for the complete rust-analyzer semantic pipeline. [ra_ap_syntax](https://docs.rs/ra_ap_syntax/latest/ra_ap_syntax/)
- `rustc_private` accesses unstable compiler internals and requires compiler/LLVM development components. This introduces compiler coupling and distribution work that a syntax-based compatibility migration does not need. [Rust Unstable Book](https://doc.rust-lang.org/unstable-book/language-features/rustc-private.html)

These are design judgments based on each library's documented scope. A parser change should be justified by a required capability, not assumed to be necessary for multithreading.

## Migration and validation proposal

1. Freeze the current file-selection, CLI, configuration, diagnostic, and explicit-fix behavior as the oracle. Pin parser and grammar versions independently.
2. Build one reusable Rust analysis library and a CLI. Discover files once, retain original source bytes, and parse each distinct required source view once per run (raw and production-masked views may differ). Keep PopRaKo-specific rules in their owning repository, consuming the shared library rather than copying the engine.
3. Port `trait-use-anonymous` for API/Unicode parity and `module-dependency` for representative cross-file cost. Compare the Rust single-worker version against Python before adding concurrency, to distinguish implementation improvement from parallelism. The large spacing checker is another useful candidate after establishing macro/token behavior.
4. Run independent file extraction in parallel, merge owned symbol/import facts, then resolve cross-file aliases and run global rules. Preserve convergence and invalidation behavior; adding threads does not fix repeated global scans.
5. Collect diagnostics locally and sort them deterministically. Keep fixes explicit, detect overlapping edits, apply edits in byte-offset-safe order, and reparse changed files as needed.
6. Differentially compare existing fixtures and the complete repository: file set, rule IDs, path/line/column, messages, exit status, and fix output. Include Unicode, CRLF, comments, raw strings, attributes, macro token trees, incomplete syntax, aliases, globs, and cycles.
7. Measure the same binary and input with one/two/four workers, reporting wall time, total CPU time, peak memory, and diagnostic equality. Separate compilation and tool installation from recurring execution; lint entry points should execute the already-built binary. Use a bounded number of ordinary runs; no stress workload is needed.

No speed multiplier, final duration, or thermal improvement is established until a representative port is measured. Keeping Python checks as an oracle during migration reduces rule-loss risk, while full replacement requires preserving all custom rule semantics and maintenance workflows.

# Rust macro line width

`WID001` rejects macro-related lines wider than `max_width` in the workspace
root's `rustfmt.toml` or `.rustfmt.toml` (in that order). The limit is required
and must be a positive integer. `tab_spaces` is read from the same file and
defaults to rustfmt's four columns when absent. Invalid or missing configuration
fails the check.

The rule parses original Rust with Tree-sitter and checks macro invocations,
macro definitions, and procedural/helper attributes such as `#[instrument]`.
`cfg_attr` wrappers around macro attributes are included. Nested macro ranges
are merged so each physical line is reported once.

Ordinary Rust code, imports, strings outside macros, and standalone comments
are outside the rule's scope. Built-in attributes such as `allow`, `expect`,
`derive`, and `cfg` are excluded, including inside opaque macro token trees.
String arguments and code inside macro bodies are checked.

The workspace is scanned once, including tests, examples, benches, `build.rs`,
and generated Rust files. Build output, dependencies, tooling, references, and
worktrees are excluded. The checker is read-only and has no suppression markers.

Widths count wide/fullwidth Unicode characters as two columns, combining marks
and format characters as zero, and other characters as one. Tabs advance to the
next configured tab stop. Line endings do not count; trailing spaces do.

Every offending line is printed as `path:line:column: WID001`, with its width,
limit, and excess. A final summary reports the total lines and affected files.
Exit status is 1 for width violations, 2 for configuration or read failures,
and 0 otherwise. Existing violations must be resolved before CI can pass.

```sh
linters-extra/.venv/bin/python linters-extra/line-width/check.py --self-test
linters-extra/.venv/bin/python linters-extra/line-width/check.py --root .
sh linters-extra/run-check.sh
```

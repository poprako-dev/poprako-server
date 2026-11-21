# Project lint runner

Run `sh linters-extra/run-check.sh` from the repository root. It prepares the
Python environment, runs the existing self-test gate, then checks the shared
suite and every project rule. Every failure contributes to the final exit code.
Project rule output includes elapsed time.

`line-width` enforces the workspace rustfmt `max_width` on macro invocations,
definitions, and procedural/helper attributes, including `instrument`. Ordinary
code, imports, built-in attributes, and standalone comments are outside its
scope. It runs once across the workspace and reports every violation. See
[line-width/FORMAT.md](line-width/FORMAT.md) for column counting and exclusions.

`minimum-pub` requires every plain `pub` to be referenced outside its defining
module. It checks modules, items, re-exports, struct fields, and inherent
associated items across the whole workspace, including references from tests,
benches, examples, macro bodies, and attribute string paths. Struct fields are
judged as one unit. It runs once for the workspace. See
[minimum-pub/FORMAT.md](minimum-pub/FORMAT.md) for the diagnostic codes and
known source-analysis limits.

`struct-field-visibility` requires a struct to expose either every field or
none. Mixed visibility means the struct is neither a data shape nor a
behaviour object. See
[struct-field-visibility/FORMAT.md](struct-field-visibility/FORMAT.md) for the
data/behaviour split and its interaction with `minimum-pub`.

`serde-rename-ban` rejects manual `serde(rename = "...")` attributes. Rust
fields and variants must carry their wire names directly; `rename_all` casing
conversions remain allowed. Pica wire modules and enum values containing dots
or hyphens are the only exceptions. See
[serde-rename-ban/FORMAT.md](serde-rename-ban/FORMAT.md).

`serde-scope` keeps `serde`, `serde_json`, and `utoipa` out of the domain
layers and requires HTTP wire types to live in `src/api/http/object`. Its
migration baseline records the violations that remain while domains are moved
one at a time. See [serde-scope/FORMAT.md](serde-scope/FORMAT.md).

Project rules run in one Python process. During that read-only run, masked
production sources are reused across rules, and an LRU cache retains up to
1024 Rust syntax trees keyed by their exact source bytes. Each caller receives
its own tree copy. The caches are released on exit, including exceptional exit;
there is no persistent cache and every invocation checks the current files.

Self-tests that edit fixtures run in isolated processes before the read-only
session. Standalone checker commands remain available, and do not retain
analysis caches. `direct-struct-import` and its compatibility entry points only
edit files when `--fix` is passed explicitly.

Run runner and analysis regression tests with:

```sh
linters-extra/.venv/bin/python -m unittest discover -s linters-extra/tests
```

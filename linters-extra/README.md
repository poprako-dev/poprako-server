# Project lint runner

Run `sh linters-extra/run-check.sh` from the repository root. It prepares the
Python environment, runs the existing self-test gate, then checks the shared
suite and every project rule. Every failure contributes to the final exit code.
Project rule output includes elapsed time.

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

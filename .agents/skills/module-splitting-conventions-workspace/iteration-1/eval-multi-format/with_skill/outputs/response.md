The unsplit import module is 780 lines, so it exceeds the strict 600-line limit and warrants extraction.

Proposed split: extract exactly one business-named child module, `labelplus`. LabelPlus is the only format with a complete, stable workflow large enough to form its own implementation responsibility. Move the LabelPlus parser, its workflow-specific types, validation/conversion functions, and private workflow utilities into that child. Keep the import entry point, public exports, format dispatch/selection, shared import types, and the short JSON and CSV parsing branches in the parent.

Projected sizes:

- Parent import module: approximately 420 lines, plus a small `mod labelplus;` declaration and any necessary private imports.
- `labelplus` child: approximately 360 lines.

Both files remain strictly below 600 lines. No `json` or `csv` children should be created: those branches are short, and extracting them would add classification modules merely for symmetry. Public API, behavior, errors, transactions, and tracing must remain unchanged.

Please confirm this exact one-child split before any production code is moved or edited. After confirmation, the concrete symbol inventory should be checked against the source during extraction, followed by the split audit and the repository’s formatting, compile, test, and lint checks.

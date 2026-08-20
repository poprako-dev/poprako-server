Split only the LabelPlus workflow into a private child module. Keep the short JSON and CSV branches, format dispatch, and shared import types in the existing module.

```text
import.rs
import/
└── labelplus.rs
```

`import.rs` should declare `mod labelplus;` and delegate the LabelPlus branch through one narrow function, while preserving the module's existing public API. Move LabelPlus-specific parsing, validation, intermediate state, helpers, and focused tests into `import/labelplus.rs`. Keep genuinely cross-format types and helpers in `import.rs`; do not create a generic shared module for code used only by LabelPlus.

This extraction follows the actual responsibility boundary: LabelPlus is a cohesive 360-line workflow, whereas JSON and CSV are too small to justify separate files. The parent should fall from roughly 780 lines to about 420 lines, with both resulting files comfortably below the 600-line limit. Avoid splitting LabelPlus further unless its extracted file independently approaches the limit or reveals another strong cohesive boundary.

# Model and Shape storage

Server-only structural ownership checks complement the dependency checker.
The owning design guidance is
[model-shape-boundaries](../../.agents/skills/model-shape-boundaries/SKILL.md).

- `OWN001`: Model structs, enums, and aliases cannot carry lifetimes,
  borrowed storage (including nested references and `Cow`), or Shapes.
  Local aliases and transitive local fields are resolved. Borrow-returning
  methods and function-pointer signatures do not represent stored borrows.
- `SHP001`: Shapes belong in `shape::{read,write,shared}::<domain>`.
- `SHP002`: Shape declarations use a domain-qualified `Shape` suffix.
- `SHP003`: Each Shape has named lifetime parameters used in stored data.
  `PhantomData` and function signatures do not constitute borrowing owners.
- `OWN002`: The crate manifest is missing. Malformed production Rust retains
  the shared scanner's `LYR002` diagnostic.

The checker uses the layer-dependency scanner's production module discovery
and alias resolution. Test-only modules are excluded; production feature
branches remain checked. It inspects source declarations rather than expanded
macros or external crate implementations. Generic ownership and allocation
behavior still require review. Dependency direction is enforced separately by
`layer-dependency`; direct type imports also apply to Shapes.

Run `sh linters-extra/run-check.sh` from the repository root for the full
suite, including fixture self-tests. The standalone checker accepts `--root`
and `--self-test` and does not modify Rust files.

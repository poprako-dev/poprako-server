# Data struct naming

Public data types must carry their domain in the type name, and their role is
encoded by the `src/data` submodule:

```rust
use crate::data::instr::team::CreateTeamInstr;
use crate::data::view::team::TeamInfoView;
use crate::data::view::team::TeamMemberView;

fn create(instr: CreateTeamInstr) -> TeamInfoView { /* ... */ }
```

In `src/data/instr/team.rs`, write `CreateTeamInstr`, not `CreateInstr`. Every
public data type has one boundary role:

- Usecase and handler inputs end in `Instr`.
- Direct response DTOs that are not model `Info` projections end in `Val`.
- Every direct projection of a model `*Info` is named `*InfoView`, even when
  the endpoint returns it directly or wraps it in `Vec`, `Option`, or another
  response DTO.
- Other response-only nested structures end in `View`.

Legacy role suffixes are forbidden. A persisted creation input belongs in the
model layer as a domain-qualified `Entry`.

Run the standalone checker from the repository root:

```bash
uv run fmt/data-struct-naming/check.py
uv run fmt/data-struct-naming/check.py --self-test
```

The script contains its pinned Tree-sitter dependencies, reports violations
only, and never modifies a Rust file.

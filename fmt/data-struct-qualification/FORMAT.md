# Data struct imports

Every data role contains public domain submodules. Import the concrete type
from its role and domain module, then use the type bare:

```rust
use crate::data::instr::team::CreateTeamInstr;
use crate::data::val::team::TeamInfoVal;
```

Legacy wrappers, root data re-exports, and module-qualified uses such as
`team::CreateTeamInstr` are forbidden. The checker
delegates to the shared Tree-sitter rule:

```bash
uv run fmt/data-struct-qualification/check.py
uv run fmt/data-struct-qualification/check.py --fix
```

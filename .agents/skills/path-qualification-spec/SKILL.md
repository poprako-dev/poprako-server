---
name: path-qualification-spec
description: "Call-site path rules: crate:: prefix forbidden at call sites; intra-layer calls use bare submodule names; cross-layer calls add layer qualifier (usecase::, complex::); third-party exceptions: serde_json, jwt, diesel allow qualified paths."
---

# Call-Site Path Qualification Specification

## 引言

调用处的路径有两条独立的规则：
1. **内部代码** — 区分层内调用 vs 跨层调用
2. **三方库** — 默认 import 裸名，例外允许 qualified

它们互不冲突，各自覆盖不同的调用场景。

---

## 规则 1：内部代码 — `crate::` 禁止 + 层间/层内区分

### 1a. `crate::` 禁止出现在调用处

函数、类型、常量等的调用处**不得使用 `crate::` 前缀**。所有项必须通过
`use crate::...` 导入后再以**裸名**或**层限定短路径**调用。

`use` 导入语句中必须用 `crate::` 完整路径，但调用处不能再出现 `crate::`。

### 1b. 层内调用 vs 跨层调用

| 场景 | 调用处写法 | 示例 |
|------|-----------|------|
| **同层内不同子模块** | 子模块名直接调用 | `usecase/user.rs` 中调 `member::create(...)` |
| **跨层调用** | 加层 qualifier | `usecase/team.rs` 中调 `complex::team::delete_cascade(...)` |
| `crate::` 前缀 | ❌ 永远禁止 | `crate::domain::complex::team::delete_cascade(...)` |

**层 qualifier 列表**（跨层时必须加的顶层模块名）：
- `usecase::` — usecase 层
- `complex::` — domain/complex 层
- `domain::` — domain 层
- `infra::` — infra 层
- `api::` — API 层

**层内不允许加层 qualifier**——同层调用带层前缀属于冗余。
例如 `usecase/team.rs` 中调 `workset::create(...)` → ✅
`usecase/team.rs` 中调 `usecase::workset::create(...)` → ❌（层内冗余）

### 1c. 深层路径必须简化为裸名

超过 2-3 层的嵌套路径（非 top-level 子模块）必须通过 `use` 导入后
以裸名使用，不得在调用处保留全路径。

**正确：**
```rust
use crate::domain::model::aggr::team::TeamAggr;
use crate::domain::model::value::role::RoleFlag;
use crate::usecase::data_object::member::MemberCreateParams;

// 调用处：
harn.seed_team(TeamAggr { ... });
u32::from(RoleFlag::Admin);
let params = MemberCreateParams { ... };
```

**错误：**
```rust
// ❌ 调用处出现深层 crate:: 路径
harn.seed_team(crate::domain::model::aggr::team::TeamAggr { ... });
u32::from(crate::domain::model::value::role::RoleFlag::Admin);
let params = crate::usecase::data_object::member::MemberCreateParams { ... };
```

### 1d. 判断方法速查表

| 调用处写法 | 判定 | 理由 |
|-----------|------|------|
| `member::create(...)` | ✅ 合法 | 同层内调用子模块 |
| `workset::get_by_id(...)` | ✅ 合法 | 同层内调用子模块 |
| `complex::team::delete_cascade(...)` | ✅ 合法 | 跨层调用，加层 qualifier |
| `complex::user::delete_cascade(...)` | ✅ 合法 | 跨层调用 |
| `usecase::member::create(...)` | ❌ 非法 | 同层内冗余的层前缀 |
| `domain::complex::team::delete_cascade(...)` | ❌ 非法 | 同层内（domain 内跨 complex 不需要再写 domain::） |
| `crate::usecase::member::create(...)` | ❌ 非法 | `crate::` 禁止 |
| `crate::domain::complex::team::delete_cascade(...)` | ❌ 非法 | `crate::` 禁止 |

> 注：`complex` 本身已是 domain 下的一个子模块，所以 `domain::complex` 是冗余的。
> 在 domain 内部跨到 complex 只需要 `complex::`。

---

## 规则 2：三方库 — 默认 import 裸名

除例外列表外，所有三方库的**类型、函数、常量**必须通过 `use` 导入后
以裸名调用。不得在调用处写 `CrateName::item`。

### 例外列表（允许 qualified 路径）

| Crate | 允许形式 |
|-------|---------|
| `serde_json` | `serde_json::from_value(...)`, `serde_json::to_value(...)`, `serde_json::Value::Null`, 等 |
| `jsonwebtoken` / `jwt` | `jwt::encode_header(...)`, `jwt::Token::...` 等 |
| `diesel` | `diesel::update(...)`, `diesel::insert_into(...)`, 等 |

```rust
// ✅ 例外允许：qualified 路径
serde_json::from_value(payload)?;
serde_json::to_value(&message)?;

// ✅ 如果愿意也可以 import 裸名（不强制）
use serde_json::from_value;
from_value(payload)?;
```

### 其余三方库必须 import 裸名

```rust
use time::OffsetDateTime;
use serde::Deserialize;

// ✅ 正确
let now = OffsetDateTime::now_utc();

// ❌ 错误
let now = time::OffsetDateTime::now_utc();

// ✅ 正确（derive macro 也必须 import 后裸名）
#[derive(Debug, Deserialize)]
pub struct Foo { ... }
```

---

## 规则 3：Derive/Attribute Macro

所有三方库的 derive/attribute macro 必须 `use` import + 裸名。
不得在 `#[derive(...)]` 或 `#[...]` 内写 qualified 路径。

```rust
// ❌ 错误
#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]

// ✅ 正确
use serde::Deserialize;
#[derive(Debug, Deserialize, utoipa::IntoParams)]
```

> `serde` 和 `serde_json` 是两个不同的 crate。`serde` 不在例外列表里，
> 所以 `serde::Deserialize` 必须 import 为 `Deserialize`。

---

## 规则 4：Doc Comment 例外

`///` 注释中的 `crate::` intra-doc 链接（`[name](crate::...)` 格式）不受此规则限制。

```rust
/// 实现见 [`UserForm`](crate::domain::model::aggr::user::UserForm)
//                    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ ✅ 文档链接，允许
```

---

## 规则 5：`utoipa::OpenApi` proc macro

`#[openapi(components(schemas(...)))]` 中的路径可能需要 qualified 形式
因为 proc macro 的解析环境特殊。短路径能编译则优先用短路径，
否则 qualified 路径可接受。

```rust
#[derive(OpenApi)]
#[openapi(
    components(schemas(
        // ✅ 尽量先用短路径
        usecase::data_object::user::UserBase,
        // ✅ 如果短路径编译不过，回退到 qualified
        crate::usecase::data_object::user::UserBase,
    ))
)]
pub struct ApiDoc;
```

---

## 检查命令

```bash
# 1. crate:: 在调用处（非 use 导入、非 doc comment）
rg 'crate::' -g '*.rs' src/ \
  | grep -v '^.*use crate::' \
  | grep -v '^[[:space:]]*///' \
  | grep -v '\$crate' \
  | grep -v 'macro_rules!' \
  | grep -v 'forward_ref'

# 2. 三方库不在例外列表中却使用 qualified 路径
rg '\b(time|uuid|tokio|axum|anyhow|thiserror|utoipa|tower|futures|serde)::' -g '*.rs' src/ \
  | grep -v '^.*use .*::' \
  | grep -v '^[[:space:]]*///'

# 3. derive macro 中的 qualified 路径
rg '#\[derive\([^)]*::' -g '*.rs' src/ --no-heading -n
```

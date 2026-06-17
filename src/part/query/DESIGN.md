# Query 层设计意图

> **本文目的**：记录 `part/query/` 模块的设计决策与意图，防止后续 Agent 理解偏差。
> **格式**：每条独立，包含 **设计** 和 **动机** 两部分。

---

## 1. `Query<H>` = Repository，非"只读接口"

**设计**：
- `Query<H>` trait 是 Repository 层本身。它可以包含读操作（如 `get_info`）也可以包含写操作（如单条 `UPDATE`），这些方法都不需要事务包裹。
- `Query` 的 impl 内部持有连接池（Pool），非事务方法每次从池中取连接即可，走 `&self`。
- `H` 泛型参数不出现在 `Query` 自己的任何方法签名中，它**仅**作为关联类型的约束锚点出现。

**动机**：
- `H` 的唯一存在意义是 `type Transactional: XXXQueryTransactional<H>` —— 通过 Rust 类型系统编译期强制 `Query` 和它 `generate_transactional()` 产出的 `Transactional` 操作的是同一套资源（同一个 Handle 类型）。防止把 RdbQuery 产出的 Transactional 错接到 Memory 的 Handle 上。
- 非事务操作不需要 `Handle` —— 它们每次从 Pool 取连接，用完即还，不与事务闭包共享 `&mut Handle`。
- 在一个 usecase 函数作用域内，只有一种 `H` 实现（要么真实，要么 mock），类型系统自己就能把路径推对。

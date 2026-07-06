## 一、业务场景

一个漫画页由若干 `Unit` 构成。每个 Unit 包含：

- 稳定身份；
- `coord`；
- `text`；
- 在页面数组中的顺序位置。

多个 Translator 可以基于不同页面版本并发编辑。例如：

`b → c`

另一个 Translator 基于旧版本 `b` 产生提交 `d`。当 `d` 到达服务器时，服务器当前版本已经是 `c`，但仍要求：

> 服务器不得因为版本过旧而拒绝提交，必须把它确定性地合并到当前版本上。

这里不追求严格保持双方业务意图，只要求：

1. 每个合法提交都能应用；
2. 合并结果始终唯一、合法；
3. 页面中的 Unit 最终形成严格全序；
4. 不需要人工处理冲突。

因此，这套机制本质上不是对称的 Git 三方合并，而是：

> 服务器将所有提交按页面级锁串行化，然后把基于旧版本产生的操作，确定性地重放到最新页面状态上。

---

# 二、核心状态模型

逻辑上将页面建模为一个有序序列：

```rust
struct Page {
    revision: Revision,
    units: Vec<Unit>,
}

struct Unit {
    id: ServerUnitId,
    coord: Coord,
    text: String,
}
```

顺序不作为可信的独立标量参与合并，而由 Unit 在 `units` 序列中的位置决定。

对外不返回 `index`、`order`、`cand_order` 或任何等价的顺序标量。

`index` 只属于 DB 层，用于持久化 Unit ID 的先后关系。

前端看到的顺序只有一种表达：

> `list units` 返回的数组顺序就是页面 Unit 顺序。

客户端不能通过提交整数 `index`、整数 `order`、`candidate_order`、`cand_order`
或独立 ID 排列来决定最终顺序；需要改变顺序时，只能提交相对位置操作。

---

# 三、身份模型

## 1. 已存在 Unit

服务器中已经存在的 Unit 使用稳定的：

`ServerUnitId`

同一个服务器 ID 永远表示同一个逻辑 Unit。服务器 ID 一旦生成，不再变化。

## 2. 客户端新建 Unit

客户端在本地创建 Unit 时生成：

`LocalUnitId`

例如本地 UUID。

提交到服务器后，由服务器生成新的：

`ServerUnitId`

并返回映射：

`LocalUnitId → ServerUnitId`

不同的本地 ID 永远视为不同 Unit。即使它们的 `coord`、`text` 和位置完全相同，服务器也不进行语义去重。

---

# 四、提交模型

每次客户端提交一个完整的 Submission：

```rust
struct Submission {
    id: SubmissionId,
    page_id: PageId,
    base_revision: Revision,
    operations: Vec<Operation>,
}

enum Operation {
    Update {
        unit: ServerUnitSnapshot,
    },

    MoveBefore {
        unit: ServerUnitSnapshot,
        before: Option<ServerUnitId>,
    },

    InsertBefore {
        unit: LocalUnitSnapshot,
        before: Option<ServerUnitId>,
    },

    Delete {
        unit_id: ServerUnitId,
    },
}
```

其中：

```rust
struct ServerUnitSnapshot {
    id: ServerUnitId,
    coord: Coord,
    text: String,
}

struct LocalUnitSnapshot {
    id: LocalUnitId,
    coord: Coord,
    text: String,
}
```

任何会写入 Unit 的操作都必须携带完整 Unit 内容，而不是字段 patch。

也就是说，`Update` 和 `MoveBefore` 都携带完整的 `coord` 与 `text`。后执行的写操作可以完整恢复或覆盖这个 Unit。

当前 save API 的传输层是 tagged event enum；详细请求/响应格式见
`docs/unit-save-api.md`。语义必须一一映射到上面的操作：

- `oper=create + local_id + 完整 unit payload`：新建 Unit，缺省插入尾部；
- `oper=create + local_id + 完整 unit payload + move_before`：新建 Unit，并插入到目标 Unit 前；
- `oper=save + id + 完整 unit payload`：保存/恢复已有 Unit 内容，位置不变；若当前已被删除，则按完整快照恢复到尾部；
- `oper=move_before + id + 完整 unit payload + move_before`：保存/恢复已有 Unit 内容，并移动到目标 Unit 前；`move_before: null` 表示尾部；
- `oper=delete + id`：删除 Unit；

不存在无 payload 的 move。移动属于 save 语义，因为并发重放时 subject Unit 可能已经被删除，必须靠完整 Unit 快照恢复。

`base_revision` 表示客户端编辑时所基于的页面版本，但它不是乐观锁条件。即使：

`base_revision < current_revision`

服务器仍然继续合并。

---

# 五、Submission 去重

每次提交必须具有唯一的：

`SubmissionId`

服务器在有限时间窗口内缓存已处理的 Submission ID。

在缓存窗口内收到相同 Submission：

- 不重复执行；
- 不重新生成 Unit ID；
- 返回第一次执行的结果。

去重判断必须与页面修改串行化，避免两个相同 Submission 同时通过检查。

缓存过期后，旧 Submission 再次到达，可以被视为一个全新的提交。极端情况下可能导致：

- 重复插入 Unit；
- 同一个操作被再次执行。

这是系统明确接受的有限幂等语义，不要求无限期保存 Submission ID。

---

# 六、服务器串行化模型

服务器对同一个页面的提交使用页面级锁。

所有提交被转换为唯一的服务器顺序：

`S1 < S2 < S3 < ...`

这里的“后者”指：

> 在页面锁内后执行并成功提交的 Submission。

它不取决于：

- 客户端时间；
- 请求发送时间；
- 客户端基于哪个 revision；
- 网络到达的物理时间戳。

同一个 Submission 内，Operation 按列表顺序执行。

因此整个系统存在两层全序：

1. Submission 之间的服务器执行顺序；
2. Submission 内 Operation 的列表顺序。

任何两个冲突操作都可以判断谁是后者。

---

# 七、操作语义

## 1. Update

`Update(unit)` 表示：

> 确保该 Unit 存在，并将其完整内容更新为提交值。

如果当前页面中仍存在该 Unit：

- 保留当前位置；
- 覆盖 `coord`；
- 覆盖 `text`。

如果当前页面中已经不存在该 Unit：

- 恢复该 Unit；
- 使用提交携带的完整内容；
- 将其放到尾节点。

因此：

`Delete(U) → Update(U)`

最终 `U` 存在。

而：

`Update(U) → Delete(U)`

最终 `U` 不存在。

删除与更新之间不再产生无法处理的冲突，后操作决定最终存在状态。

---

## 2. MoveBefore

移动统一使用：

`MoveBefore(unit, before)`

语义为：

> 写入完整 Unit 内容，并把该 Unit 移动到 `before` 对应 Unit 的前面。

执行过程：

1. 若 subject Unit 不存在，则根据完整快照恢复；
2. 将 subject 从当前序列中摘除；
3. 在摘除后的当前序列中解析 `before`；
4. 插入到解析结果之前。

定位规则：

- `before` 当前仍存在：插入其前面；
- `before == null`：插入尾部；
- `before` 已被删除：视为 `null`；
- `before == subject.id`：视为 `null`。

也就是说，`null` 是一个逻辑上的 dummy 尾节点。

例如当前状态：

`A, B, C`

执行：

`MoveBefore(A, C)`

得到：

`B, A, C`

如果 `C` 已经不存在，则得到：

`B, A`

即 `A` 被放到尾部。

---

## 3. InsertBefore

新建统一使用：

`InsertBefore(local_unit, before)`

执行过程：

1. 根据 LocalUnitId 生成新的 ServerUnitId；
2. 将本地 Unit 归一化为服务器 Unit；
3. 解析 `before`；
4. 插入到目标 Unit 前面；
5. 目标不存在时插入尾部。

其定位规则与 `MoveBefore` 完全相同。

如果多个 Submission 都在同一个 Unit 前插入：

初始：

`A, B`

先执行：

`InsertBefore(X, B)`

得到：

`A, X, B`

再执行：

`InsertBefore(Y, B)`

得到：

`A, X, Y, B`

最终顺序由服务器串行执行顺序唯一决定。

---

## 4. Delete

`Delete(unit_id)` 表示：

> 从当前页面中移除该 Unit。

如果 Unit 当前存在，则删除。

如果 Unit 当前已经不存在，则 no-op。

Delete 不需要携带完整 Unit，因为它不会写入 Unit 内容。

---

# 八、冲突裁决规则

## 内容冲突

同一个 Unit 的 `coord` 或 `text` 被不同提交修改：

> 后执行的完整 Unit 快照生效。

例如：

- `S1` 将 `U.text` 改为 `A`；
- `S2` 将 `U.text` 改为 `B`；

最终为 `B`。

由于写操作携带完整 Unit，因此后一次写入同时决定完整的 `coord` 和 `text`。

---

## 存在性冲突

一方删除 Unit，另一方写入 Unit：

> 后操作决定 Unit 是否存在。

- `Delete → Update/Move`：Unit 被恢复；
- `Update/Move → Delete`：Unit 被删除；
- `Delete → Delete`：仍然不存在。

---

## 顺序冲突

同一个 Unit 被移动到不同位置：

> 后一次 `MoveBefore` 决定最终位置。

不同 Unit 的移动也不会形成无法解决的顺序约束环，因为服务器不是合并两组静态排序约束，而是在当前序列上逐条执行摘除和插入。

每一步产生的都是一个合法线性序列。

---

## 定位点消失

移动或插入所引用的 `before` Unit 已经被删除：

> 自动降级为尾节点。

不会拒绝提交，也不会继续沿着历史版本寻找替代定位点。

---

## 并发插入

多个新 Unit 插入同一位置：

> 按服务器串行顺序依次执行。

不会发生对外顺序字段重复，因为系统没有对外顺序字段。

DB `index` 由最终序列重新派生，只用于之后的列表读取排序。

---

## 本地 Unit 重复提交

相同 Submission ID 在去重窗口内：

> 复用第一次结果。

超出去重窗口：

> 允许被当作新的插入，可能产生重复 Unit。

---

# 九、完整应用流程

服务器收到 Submission 后，逻辑流程为：

1. 检查 Submission ID 是否已经处理；
2. 获取页面级锁；
3. 再次进行去重确认；
4. 读取当前页面状态 `curr`；
5. 不要求 `base_revision == curr.revision`；
6. 按顺序执行每一个 Operation；
7. 为本地新建 Unit 分配服务器 ID；
8. 将所有缺失定位点解析为尾节点；
9. 得到新的 Unit 有序序列；
10. 根据序列位置重新生成 DB 内部 `index`；
11. 推进页面 revision；
12. 提交结果；
13. 缓存 Submission 的处理结果；
14. 返回新 revision、LocalId 映射及最终页面状态。

核心状态转移可以表示为：

`Page(n+1) = Apply(Page(n), Submission)`

而不是：

`Page(n+1) = ThreeWayMerge(base, current, clientSnapshot)`

---

# 十、最终建模结论

整个系统可以归纳为三个核心对象。

## Page

页面是一个具有 revision 的 Unit 有序序列。

DB `index` 是序列位置的内部持久化结果，不是 API 字段，也不是独立冲突字段。

## Submission

Submission 是客户端基于某个旧 revision 产生的一组有序操作，并携带用于有限时间去重的 UUID。

Submission 即使基于旧版本，也允许应用到当前页面。

## Operation

操作只需要四种：

- `Update`
- `MoveBefore`
- `InsertBefore`
- `Delete`

其中所有写操作携带完整 Unit 快照；所有位置表达统一采用 `before` 关系；所有无效定位统一降级到尾节点。

---

# 十一、该方案提供的保证

该模型保证：

- 任意旧版本提交都可以应用；
- 同一页面的所有操作具有唯一顺序；
- 内容冲突始终可以裁决；
- 删除与写入冲突始终可以裁决；
- 移动冲突始终可以裁决；
- 定位点消失不会导致失败；
- 并发插入始终产生确定结果；
- 最终页面数组不缺乏全序；
- DB 内部 `index` 可重新压紧且不向前端暴露；
- 不需要冲突返回或人工合并。

它不保证：

- 最终文本一定符合 Translator 的原始业务意图；
- 删除后的旧修改不会重新恢复 Unit；
- 旧移动不会覆盖较新的内容；
- 相似 Unit 不会重复；
- 去重窗口过期后不会重复执行；
- 提交到达顺序不同仍得到相同结果。

因此它是一套：

> 以服务器线性顺序为最终裁决权、以可用性和必然合并为目标的确定性操作重放模型。

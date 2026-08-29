# ObjDept 与通用 Prom 最终修正计划

## 1. 计划状态

本文是本轮开发修正的唯一实施依据，覆盖 ObjDept、通用 Prom、PageImage
上传后自动推进以及生产启动组装。

本文只定义内部架构修正，不授权改变任何外部行为。实现前必须先恢复当前
工作树中未闭合的错误修改，再按本文分阶段开发。

以下内容不属于可选优化：

- 不改变 HTTP、OpenAPI、请求和响应。
- 不改变任何 migration、table、column、index、status 字符串或 JSON
  wire format。
- 不改变 page reserve 的事务、payload、task id、20 分钟 delay 或投递次数。
- 不改变通用 Prom 的 poll、claim、lease、retry、wait、dead、reset、purge
  和 shutdown 行为。
- 不改变固定四个 worker、FNV 分片、同 shard 串行和不同 shard 并行的行为。
- 不改变 Chapter 推进事务、锁顺序、workflow record 或 event 派发时机。
- 不改变 Invitation 当前直接 `run_on` 的事务形状。
- 不改善现有 Develop 满队列、关闭以及并发 shutdown 时可能丢 event 的行为；
  该缺口可另立任务处理，本次必须保持现状。
- 不改变 ObjDept 私有 prom 的任何行为。

## 2. 已确认的错误

### 2.1 空 Sched

当前 `Sched` 没有注册任何 job，只创建 cancellation token 和空 receiver
集合。它不是通用 Prom 或 ObjDept 的依赖，也不产生外部行为。

最终处理：

- 删除 `src/extra/sched.rs`。
- 删除只为它存在的 `src/extra.rs`。
- 删除 `src/lib.rs` 中对应 module、注释和导出。
- 删除 `src/main.rs` 中对应 import、构造和 shutdown 调用。

不得创建替代 scheduler、空壳类型或兼容别名。

### 2.2 RdbProm 直接知道业务实现

当前通用 Prom 的 RDB 实现直接持有具体 ObjDept 和 Develop，并通过
`RdbPromRepo<R>` 同时持有 queue repo 与业务 repo。这使 RDB queue
mechanics、业务 usecase 和生产组合混在同一层。

最终处理不是删除业务能力，而是移动所有权：

- `RdbProm` 必须持有 `RdbCore`。
- `RdbProm` 继续拥有 actor 生命周期。
- RDB actor 只调用一个静态 callback，不出现 ObjDept、Develop 或具体业务
  repo 的类型约束。
- 业务 callback 的长期 owner 持有 ObjDept 和 Develop。
- Chapter 成功后仍通过同一份 Develop 派发 event。

### 2.3 错误的 unit struct 修改

把 `RdbProm` 改为 unit struct、删除 `RdbCore`、删除 actor 启动及 `close`
属于错误修改，必须首先撤销。不得在这个错误状态上继续补丁开发。

### 2.4 业务编排位于 adapter

现有 Chapter handler 拥有完整业务编排：事务、Chapter→Page 锁顺序、
PageImage 完整性检查、阶段更新、workflow record 和事务后 event。它应属于
usecase，而不是 RDB adapter。

Invitation 的过期清理同样是业务 task，不属于 queue mechanics。

## 3. 最终依赖方向

```text
usecase::prom
    ├── 依赖 part::repo
    ├── 依赖 part::obj_dept
    ├── 依赖 part::effect
    └── 不依赖 part_impl

harn
    ├── 组合 usecase::prom
    ├── 组合 part_impl::prom::rdb_impl
    └── 创建静态 local-message callback

part_impl::prom::rdb_impl
    ├── 依赖 RdbCore
    ├── 依赖 t_local_message typed Diesel schema
    ├── 依赖通用 Prom payload
    ├── 不依赖 ObjDept
    ├── 不依赖 Develop
    ├── 不依赖业务 repo
    ├── 不依赖 usecase
    └── 不依赖 Sched

part_impl::obj_dept
    ├── 私有 RdbObjProm
    ├── 私有 R2 ObjPool
    ├── 单一 ObjActor
    └── 与通用 Prom 零依赖
```

`part` 仍只放 usecase 通过泛型注入的能力。local-message callback、queue
repo、actor 和组合胶水都不是 part。

## 4. RdbProm 的固定职责

目标字段必须保持非泛型：

```text
RdbProm
├── core: RdbCore
├── token: CancellationToken
└── done: watch::Receiver<bool>
```

硬约束：

- `RdbProm` 不实现 `Clone`。
- `core` 不允许删除。
- `token` 只用于停止 supervisor。
- `done` 必须继续使用可 clone 的 `watch::Receiver<bool>`。
- `close(&self)` 必须可重复、可并发调用。
- `Drop` 只 cancel，不等待。
- `done_send.send_replace(true)` 只能发生在四个 worker 全部 join 之后。
- callback 的泛型类型只存在于构造函数和 spawned future，不进入
  `RdbProm` 字段、Harn 类型或 HTTP state 类型。

RdbProm 的两条数据库路径必须严格隔离：

### 4.1 Producer 路径

`Step<Defer>` 与 `Step<DeferBatch>`：

- 永远只使用调用者传入的 `RdbContext` 和 `context.conn()`。
- 绝不使用 `self.core` 获取新连接。
- 与 page、ObjDept row 和其他业务写保持同一 caller transaction。
- 任一写入失败必须使整个 caller transaction 回滚。
- isolation capability 继续是 `AtLeast<ReptRead>`。

### 4.2 Consumer 路径

actor 通过 `self.core.clone()` 创建自己的：

```text
queue_nucl = RdbNucl<ReptRead>
queue_repo = RdbPromRepo
```

`RdbCore::clone()` 必须只共享同一个 pool。禁止重新调用
`RdbCore::from_env()`，禁止缓存 pooled connection，禁止让 queue context
跨越业务 callback。

## 5. Queue repo 拆分

`RdbPromRepo<R>` 改为无业务泛型的 `RdbPromRepo`。它不持有 inner repo、
core 或 connection，只实现 `t_local_message` 的 typed Diesel operations。

必须一次性删除：

- 业务 repo 泛型 `R`。
- inner 字段。
- `inner()` accessor。
- actor 为 queue mechanics 声明的业务 repo bounds。

以下 operations 的 SQL、返回值和事务必须原样迁移：

- `PollPending`
- `ClaimPending`
- `CompleteMessage`
- `FailMessage`
- `RetryMessage`
- `ResetStuck`
- `PurgeCompleted`

禁止：

- `diesel::sql_query`
- `QueryableByName`
- 手写 SQL
- runtime table/column name
- untyped row
- 修改 `LocalMessageEntryRow` 或 `LocalMessageRow` 的列合同
- 修改 migration 或 generated schema

## 6. RDB actor 的静态 callback

RDB actor 泛型只表示 callback：

```rust
H: Fn(TaskPayload) -> F + Send + Sync + 'static
F: Future<Output = TaskFlow> + Send + 'static
```

不得使用：

- `dyn`
- boxed future
- `async_trait`
- registry
- `TypeId`
- runtime handler lookup

callback 输入使用 owned `TaskPayload`，不使用 borrowed topic/value，也不额外
clone topic。

`TaskFlow` 的可见性固定为：

- 定义在 `part_impl::prom::task_flow::TaskFlow`。
- `task_flow` 与 `TaskFlow` 使用普通 `pub`，但其 ancestor `part_impl` 保持
  private，因此 library 内的 `harn` 与 RDB actor 都可访问，外部 crate
  无法命名。
- 创建 actor 的泛型入口是 `part_impl::prom::rdb_impl` 内部未 root
  re-export 的 free function，不是 `RdbProm` 的 public associated method。
- 该内部 free function 才声明 `F: Future<Output = TaskFlow>`；root 公开 API
  不得泄漏 `TaskFlow`。
- binary main 只调用 root re-export 的生产组合入口，并且看不到 callback、
  `TaskFlow` 或 handler 的具体类型。

每条 row 的顺序必须是：

1. supervisor poll row。
2. 使用持久化的原始 `f_topic` 计算 FNV shard。
3. 以 `id + Pending + observed lease` claim。
4. claim transaction 提交。
5. row 发送到对应 worker。
6. worker 内 clone 当前已有的 JSON value 并反序列化 `TaskPayload`。
7. worker 内核对 `payload.topic() == row.f_topic`。
8. malformed JSON 或 topic mismatch 直接产生 `Dead`，不调用业务 callback。
9. 合法 payload 才调用 callback。
10. callback 完成后才执行 complete、retry 或 fail 的独立 transaction。

callback 运行期间不得持有 queue connection。

## 7. 业务 usecase

新增 `src/usecase/prom.rs`，接收通用 Prom payload 并执行业务编排。

该模块定义自己的业务输出，例如：

```text
PromTaskAction
├── Complete
├── Retry { message }
└── Wait { message }
```

`Dead` 不属于业务 usecase 输出；malformed JSON 和 topic mismatch 由 RDB
actor 在进入 usecase 前处理。

组合层把 `PromTaskAction` 一对一映射为 actor 私有的 `TaskFlow`。usecase
不得依赖 `TaskFlow` 或任何 part_impl 类型。

### 7.1 Chapter task

必须从现有 handler 原样迁移以下行为：

1. usecase 只接收泛型注入的 `N`，并要求
   `N: Nucl<Context = RdbContext, Error = BaseError>`；通过 `nucl.coord`
   开启业务事务。usecase 不引用 concrete `RdbNucl`。
2. 先锁 Chapter，再读取 Page；锁顺序仍是 Chapter→Page。
3. 对每个 page 通过 `ObjDept<PageImage, RdbContext>` 读取最新 ObjMeta。
4. 任一 page 缺失或 `f_is_uploaded == false`，事务正常结束并返回 Wait。
5. 全部完成后执行 `CompleteChapterRawProvide`。
6. 实际发生推进时，在同一事务写入 workflow record。
7. 业务事务提交后，同步 await `ChapterWorkflowCompleted` 的 Develop 调用。
8. Develop 调用返回后，业务 task 才返回 Complete。
9. `Ok(Some(false))` 与 `BaseError::Expected` 仍返回 Complete。
10. 其他错误仍返回 Retry，并保持原错误文本格式。

不得把 event 放进事务，不得在 event 外另行 spawn，不得在 event 前 complete
local message。

### 7.2 Invitation task

Invitation 必须继续直接对独立业务 repo 调用 `run_on`：

- 不包进 `RdbNucl::coord`。
- 不复用 queue context。
- 成功返回 Complete。
- 失败返回 Retry，并保持原错误文本格式。

## 8. 生产组合与 Develop 生命周期

生产组合位于 `harn`，因为它是同时看见 usecase 与 part_impl 的 composition
root。不得让 RDB adapter 反向依赖 usecase。`harn` 中的生产组合入口使用
普通 `pub` 并由 crate root re-export 给 binary main；handler、callback 和
RDB actor 构造 free function 均不做 root re-export。

组合时创建：

```text
handler_nucl = RdbNucl<ReptRead>::new(core.clone())
handler_repo = HybRepo::new(core.clone())
handler_obj_dept = obj_dept.clone()
handler_develop = develop.clone()
```

`handler_repo` 必须使用 `HybRepo::new(core.clone())`，不得 clone Harn 中的
repo；现有 actor 使用独立 process-local map，该行为必须保持。

上述四项放进一个长期存活的 `Arc<LocalMessageHandler<N, R, O, D>>`。
每次 callback：

- 只执行 `Arc::clone(&handler)`。
- 绝不 clone `LocalMessageHandler`。
- 绝不按 task clone Develop。
- 绝不按 task clone ObjDept。

原因：`AsyncEffectDevelop` 的任意 clone 在 Drop 时都会 cancel 共享 token。
按 task clone 会在第一条任务结束时关闭全局 Develop。长期 `Arc` 必须一直
存活到四个 worker 全部 drain。

ObjDept 的共享必须恢复既有 `Clone` 语义：只 clone core、pool、prom 和同一
actor descriptor，不创建第二个 ObjActor。不得重新调用 production factory
构造第二个 ObjDept。

Harn 仍只持有 usecase 所需的 `RdbProm`、ObjDept 和 Develop。callback 的
具体类型不进入 Harn 泛型。

`main` 的启动顺序与 shutdown 关系保持：

- 在原通用 Prom 构造位置立即启动 consumer。
- serve 返回后，继续使用同一个 `tokio::join!` 并发关闭 ObjDept、RdbProm
  和 Develop。
- 不改成顺序关闭。

当前并发关闭可能使 Develop 在 Chapter handler 派发前被 cancel。该行为是
现状，本次不得偷偷改善，也不得在文档中声称 event 已实现可靠投递。

## 9. Actor 状态机冻结

以下全部是代码级不变量：

- 固定四个 worker。
- 使用现有 FNV offset basis、prime 和取模方式。
- 同一 shard 串行，不宣称严格同 topic 独占 worker。
- 不同 shard 可并行。
- poll 只选 visible Pending。
- 每个 topic 只选一条，并受同 topic Processing row 阻塞。
- claim 条件为 id、Pending 和 observed lease。
- claim 不增加 lease。
- claim 更新零行返回 `Ok(false)`。
- complete、retry 和 fail 使用 Processing + exact lease fencing。
- stale finalize 更新零行仍是成功 no-op。
- Retry 的 retried count 增量为 1。
- Wait 的 retried count 增量为 0。
- `retried_count >= 3` 只把 Retry 转成 Dead，不影响 Wait。
- retry visible time 为 handler 返回后的当前时间加五分钟。
- reset 以十五分钟 Processing timeout 为界。
- reset 的 Pending 与 Dead 两个分支都增加 lease。
- reset 阈值继续使用 `< 3` 与 `>= 3`。
- reset 内两次 UPDATE 继续处于同一 transaction。
- poll interval 一分钟。
- reset interval 一分钟，启动后立即执行一次。
- purge interval 一小时，启动后立即执行一次。
- completed retention 七天。
- dead retention 三十天。
- worker channel send 失败时不主动补偿，保持 Processing 等 reset。
- queue operation 错误只按现有位置记录，不终止 supervisor。
- reschedule 时间计算或 DB 更新失败只记录，保持 Processing。

## 10. Shutdown 冻结

cancellation 只允许终止 supervisor：

1. supervisor 看见 token 后退出。
2. drop 全部 worker senders。
3. workers 继续处理已经 claim 或已经入队的 row。
4. callback、Develop 和 final queue mutation 外层不得添加 cancellation
   select、timeout 或 abort。
5. 逐个 await 四个 worker handles。
6. 所有 worker 完成后才设置 done 为 true。
7. `RdbProm::close()` 等待 done。

这保证不会新增“Chapter 已提交但 handler 被取消，重跑又因阶段已推进而不再
派发 event”的丢失路径。

## 11. ObjDept 私有 prom 的隔离

通用 Prom 不得读取或写入 `t_obj_prom_task`。

ObjDept 私有 prom 不得读取或写入 `t_local_message`，也不得投递 Chapter
业务 task 或派发 Develop event。

完整业务流保持：

```text
page reserve usecase
├── 在 caller transaction 中通过 ObjDept 创建 slot
├── ObjDept 在同一 transaction 记录私有 Check 债务
└── 在同一 transaction 向通用 Prom 投递 Chapter task

ObjActor
└── 独立消费 t_obj_prom_task，更新 PageImage f_is_uploaded

通用 Prom actor
└── 延迟消费 Chapter task
    ├── 未全部上传：Wait
    └── 全部上传：推进阶段 → workflow record → Develop event → Complete
```

两套 prom 没有互相调用、类型依赖或 task 转发。

## 12. 当前错误工作树的恢复顺序

实施开始后必须按以下顺序恢复，禁止顺手处理其他文件：

1. 检查并清除 ObjDept 文件中的 conflict marker，依据最后通过 review 的
   单字母泛型版本恢复，不使用 git reset 或 checkout 覆盖用户修改。
2. 恢复 `NormObjDept` 的共享 Clone，只共享同一 actor descriptor。
3. 恢复 `RdbProm` 的 `core`、token、done、构造即启动、close 和 Drop。
4. 删除错误的 actor/descriptor re-export；RdbProm 继续自行拥有生命周期。
5. 保留已经确认的空 Sched 删除。
6. 运行 formatter auto-fix 和最小 `cargo check --all-features`，确认恢复
   后再开始架构移动。

恢复阶段不得改业务逻辑。

## 13. 实施阶段

### 阶段 A：冻结行为测试

先补 characterization tests，不改生产行为：

- 单条 Defer 与业务写 commit/rollback 同生共死。
- DeferBatch 的 commit、rollback、空 batch 和重复 id 全事务回滚。
- malformed JSON → Dead。
- topic mismatch → Dead。
- Retry 增量 1、Wait 增量 0。
- 第四次 Retry → Dead，Wait 不受影响。
- stale complete/retry/fail 全字段不变。
- reset 两分支、lease 增量和同事务原子性。
- 两个 actor/core 并发 claim 只有一个 winner。
- 同 shard 串行、不同 shard 并行，FNV 分片不变。
- actor 启动后立即 reset/purge，各时间常量不变。
- close 可重复、可并发、actor 已完成后仍可调用。
- close 在已 claim handler 被 barrier 阻塞时持续等待；释放后 callback 与
  final mutation 完成才返回。
- Drop cancel。
- 第一条 callback 完成后 Develop 仍未被 per-task Drop cancel。
- Chapter commit → Develop → local-message Complete 的顺序。
- Invitation 不新增外层 coord。

时间相关测试使用 paused time；RDB transaction 测试必须通过
`RdbNucl::coord`，不得只构造无 begin 的 `RdbContext` 冒充事务证明。

### 阶段 B：拆 queue repo

- 只移除 `RdbPromRepo` 的业务泛型和 inner。
- 保持全部 typed Diesel query 与 transaction 调用点不变。
- 运行 repo 与 actor targeted tests。

### 阶段 C：建立静态 callback seam

- actor 泛型化为 `H/F`。
- 保持 actor pool、shard、状态机和生命周期原实现。
- 在 callback 前保留 deserialize/topic validation。
- callback 后保留原 final mutation。
- 运行全部 actor characterization tests。

### 阶段 D：迁移业务 usecase

- 将 Chapter 与 Invitation 业务编排移到 `usecase::prom`。
- Chapter 保持 coord，Invitation 保持 run。
- composition root 创建长期 `Arc<LocalMessageHandler<...>>`。
- 映射 usecase action 到 actor TaskFlow。
- 运行 page reserve、chapter workflow、invitation 和 effect tests。

### 阶段 E：生产组装

- main 使用最终生产组合入口。
- Harn 泛型与 AppHarn 仍以非泛型 `RdbProm` 作为 P。
- shutdown 仍并发关闭三项。
- 删除 Sched 全部残留。

### 阶段 F：格式与完整验证

机械问题先使用 formatter/linter auto-fix，不手工逐项修格式。

最低命令：

```text
cargo fmt --all
cargo fmt --all --check
cargo check --all-features
cargo test -p poprako-server
```

最终按用户要求运行完整 CI entry。任何失败都必须先定位到本次改动或既有
工作树，不得改 linter，不得扩大任务范围。

## 14. 明确禁止的实现

- 不把 RdbProm 改成 unit struct。
- 不删除 RdbCore。
- 不删除、跳过或异步 fire-and-forget Develop event。
- 不让 RdbProm 直接出现 ObjDept、Develop 或具体业务 repo 字段。
- 不把业务 handler 留在 rdb_impl actor 子树。
- 不创建第二个 ObjDept。
- 不按 task clone Develop。
- 不让任一临时 owner 的 Drop cancel 全局 actor/effect。
- 不让 producer 使用 RdbProm 自己的 core 开新 transaction。
- 不把 claim、handler、complete 合并成一个 transaction。
- 不让 queue connection 跨 callback。
- 不把 Invitation 包进新的 coord。
- 不改变 worker 数、分片、时间常量或状态机错误处理。
- 不引入 dyn、boxed future、async_trait 或 runtime registry。
- 不引入非类型安全 Diesel。
- 不新增数据库约束。
- 不修改 migration 或 schema。
- 不创建 Sched 替代品。
- 不使用非单字母泛型参数。
- 不新增被禁止的命名、oper constructor、`if ... else` 或 scoped
  visibility。
- 不借本任务修改无关代码、linter 或文档。

## 15. Review 意见闭环

四个 reviewer 的两轮否决已落实为以下修正：

| 被否决内容 | 最终修正 |
|---|---|
| RdbProm 变成 unit struct | 固定持有 RdbCore、token、watch receiver |
| actor descriptor 移到 main | actor 生命周期继续由 RdbProm 拥有 |
| 删除 Develop | Develop 由长期业务 callback owner 持有 |
| 每次任务 clone Develop | 每次只 clone Arc，Develop clone 活到 worker drain |
| 私有 handler 由 binary main 命名 | handler 在 library composition root 内构造 |
| queue repo 继续包装业务 repo | 改为无业务泛型的 queue-only repo |
| handler 继续位于 part_impl | 业务编排迁入 usecase，composition 只做映射 |
| 所有 handler 统一 transaction | Chapter 保持 coord，Invitation 保持 run |
| 单 actor 被误解为单 worker | 保留一个 supervisor 与固定四 worker |
| shutdown 直接取消 handler | 只停 supervisor，继续 drain workers |
| producer 误用 self.core | producer 永远只写 caller context.conn |
| 仅用快照声称等价 | 增加 transaction、并发、时间和 barrier characterization tests |

实施 review 必须逐条对照本文，不允许用“逻辑基本不变”替代代码级证明。

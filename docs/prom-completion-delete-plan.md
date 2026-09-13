# Prom 成功即删除与 UUID 执行凭据修改计划

状态：已实施并通过验证。2026-09-13。

后续生产升级：用户已明确允许停服后的手工 schema 更新。现有库应使用
[Prom 手工升级说明](prom-manual-upgrade.md)中的独立脚本保留未完成任务，
不再采用本计划实施阶段所述的基线回滚重建。下文保留当时的实施记录。

## 目标和范围

通用 Prom 与 ObjDept Prom 都在成功确认时直接删除任务。每次领取生成新的 UUID 执行凭据，替换递增 lease，防止超时旧执行修改或删除后来领取、乃至删除后重建的任务。

本计划只涉及队列基础设施、迁移、测试和相关规范。对象版本 watermark、物理对象 key、业务 use case 和 HTTP 契约保持现有语义。UUID 碰撞不在本次设计范围内。

已有未提交的 Prom payload、topic、actor、entity 和 use case 改动必须保留；实施时基于最新工作区 diff 修改，不能恢复旧文件或整文件覆盖。

## 确定的执行契约

| 操作 | 任务状态与执行凭据 |
| --- | --- |
| 入队 | `pending`，`f_claim_token = NULL` |
| 领取 | 原子锁定候选任务、改为 `processing`、生成新 token，并返回该 token |
| 成功确认 | 匹配 `id + processing + token` 后删除记录 |
| 重试或 Wait | 匹配同一组条件，改回 `pending` 并清空 token，按原策略更新可见时间和重试次数 |
| Dead 或 Operator | 匹配同一组条件，写入失败状态并清空 token |
| 超时回收 | 按既有超时及重试策略迁移状态，同时清空 token |
| 旧执行回写 | 条件不匹配，影响 0 行，不得重建任务或改用只匹配 ID 的兜底操作 |

使用 PostgreSQL 原生 `uuid` 列 `f_claim_token`，Rust 使用 `uuid::Uuid`。领取 SQL 在每个被领取行上调用 `gen_random_uuid()`，使批量领取中的每个任务也获得独立凭据。生成与领取属于同一条语句，不能领取成功后再单独注册 token。

PostgreSQL 18 提供 UUID v4 生成函数，见 [UUID Functions](https://www.postgresql.org/docs/18/functions-uuid.html)。复用 Cargo.lock 已解析的 UUID 版本，并在需要的 Diesel 依赖上启用 UUID 类型支持。生成宏通过基础设施 crate 暴露的必要类型或助手引用依赖，避免给宏调用方增加隐式依赖。

持久化列允许空值；领取结果转换为带非空 token 的执行模型。约束保证 `processing` 必须带 token，其他合法状态必须没有 token。任务表主键继续承担定位职责，不为 token 增加无用途的独立索引。

接口仍使用 Complete / `complete_task` 表示成功确认，持久化状态移除 Completed。执行结果与数据库记录状态分开处理。

## 实施顺序

### 1. 数据迁移与类型支持

- 按用户最新指示，直接修改两张任务表的现有建表 SQL，将 `f_lease` 替换为 nullable UUID `f_claim_token`，添加状态与 token 一致性约束。
- 修改现有 ObjDept 超时索引 SQL，移除已经删除的 `f_lease` 索引字段。
- 不新增增量迁移或独立升级脚本。现有数据库需要回滚并重新运行基线，`CREATE TABLE IF NOT EXISTS` 不会升级旧表。
- 本次只在已授权的 `db_poprako_ci` 上运行 `just mgr-run` → `just mgr-reset` → `just mgr-run`，然后执行 `just mgr-schema`；禁止手改生成 schema。
- 回滚会删除对应表及其数据，不能用于未经确认的非 disposable 数据库；本次不操作开发业务库或生产库。

### 2. ObjDept Prom

涉及 `poprako-obj-dept/src/model/task.rs`、`prom.rs`、`actor.rs`，以及 `poprako-obj-dept-macro/src/rdb_obj_dept_prom.rs` 和 `rdb_obj_dept_prom/defer.rs`。

- 用 `claim_token: Uuid` 替换已领取任务上的 `lease: i64`，同步模型校验、宏生成行类型和转换。
- claim 保持全局候选排序、行锁及 `SKIP LOCKED`，在同一条更新中生成并返回 token。
- 完成改为带 token 的 DELETE；retry/operator 改为带 token 的 UPDATE，并清空 token。
- reset 清空旧 token；移除 lease 溢出判断、溢出 operator 分支以及相关错误文本。
- 移除 `COMPLETED` 常量、完成状态白名单和其他历史状态分支。
- 保留现有确定性任务 ID 与 obligation generation，本次不混入另一套任务 ID 重构。相同义务仅在任务仍存在时去重；完成删除后可以再次入队。
- `operator` 仍要求修复，不能因重复 defer 自动覆盖。
- 复核 bulk insert 后 identity/status 查询与并发删除的交互；支持的事务隔离下不得把合法的完成删除误报为身份损坏，也不得静默吞掉真正的身份冲突。
- actor 对旧 token 的 0 行结果保留明确诊断；超时、取消、远端错误分类与现有重试策略保持一致。

### 3. 通用 Prom

涉及 `src/part_impl/prom/rdb_impl/entity.rs`、`repo.rs`、`actor/pool.rs`、相关 actor 类型和 `task_flow.rs`。

- 已领取行、完成/重试/失败操作描述符和 actor 调用链统一传递 UUID token。
- claim 保留每个空闲 topic 最多领取一个任务的规则、现有锁与事务隔离，为返回的每一行生成 token。
- 完成改为条件 DELETE；retry、Wait、Dead 和 reset 按上述契约处理 token。
- 移除 `LocalMessageStatus::Completed`、7 天成功记录保留窗口和成功记录清理分支。
- 将 `PurgeCompleted` 及维护命名改为准确的 Dead 清理命名，保留既有 Dead 保留周期和清理调度。
- 保留等待不消耗失败次数、超时回收消耗既有预算、最大重试限制和 topic 调度语义。

### 4. 文档与残留清理

- 更新 `specs/obj-dept.md`：任务成功后删除、去重范围、每次领取的 UUID token 和旧执行回写规则。
- 同步 Prom port、TaskFlow、actor 文档和测试说明，删除把数值 lease 或 Completed 当成现行协议的描述。
- 检查所有 schema 投影、宏展开、测试夹具和原始 SQL；保留业务阶段 Completed 等无关概念。
- Rust 文件严格低于 600 行；若新增测试导致达到限制，先读取 module-splitting-conventions，再按现有边界拆分。

## 必须通过的回归场景

这些场景使用真实 PostgreSQL 验证条件删除、行锁和原子领取；不只验证 mock 的调用次数。

1. **成功即删除**：两套 Prom 成功确认后行不存在；ObjDept 连续创建并完成不同对象版本后不积累成功历史。
2. **重建同 ID**：保存旧执行 token A，完成删除，重新入队同 ID 并领取 token B；A 的完成、重试、Dead/Operator 操作均不能改变 B 的记录，B 能正常完成。通过直接保留旧执行描述符确定性覆盖这个条件，无需依赖真实超时竞态。
3. **超时接管**：回收后旧 token 立即失效；重新领取得到新 token，旧执行各类回写均影响 0 行。
4. **重复确认**：同一 token 第二次完成影响 0 行；完成后旧 retry/operator 不得复活任务。
5. **领取并发**：竞争领取不会交付同一条 processing 记录；通用 Prom 的 topic 隔离与 ObjDept 的排序保持；批量领取每行 token 独立。
6. **失败策略**：Wait 不耗预算、Retry 和超时预算正确、Dead 清理继续生效、ObjDept operator 不被普通 defer 覆盖。
7. **去重及事务**：pending/processing 重复 defer 仍去重；完成后可重新 defer；批量任务身份冲突仍报错；事务回滚不留下任务。
8. **对象业务回归**：同一不可用版本重新申请上传后，已完成的旧 Check 不再阻止新的 Check；旧版本不得影响新版本，watermark 行为保持。
9. **基线重建及重放**：通过 just 完成 apply → revert-all → apply；重复运行基线 SQL 时，新 processing token、状态和重试次数保持不变。

## 验证命令和审查

实施时先运行针对性测试，再完成仓库要求的检查：

```sh
cargo test -p poprako-obj-dept --all-features
cargo test -p poprako-obj-dept-macro
cargo test -p poprako-server --all-features part_impl::prom
cargo test -p poprako-server --all-features part_impl::repo::rdb_impl::tests
just fmt
just fmt-check
just check
just clippy
sh scripts/ci-test.sh
```

ObjDept 的服务端 RDB 测试当前由 `part_impl::repo::rdb_impl::tests` 调用；实施后检查入口，确保新场景实际运行。默认 `cargo test --workspace` 不足以证明 feature-gated RDB 测试已执行。

专用 CI 数据库的回滚和运行必须使用 just 配方。不能用直接 Diesel 调用或 CI 脚本代替用户指定的 `just mgr-reset` / `just mgr-run`。基线 SQL 重放属于额外验证，不能代替上述完整回滚重建。

最终审查逐项核对所有终结路径是否使用 token、成功记录是否还有写入路径、历史迁移是否能安全重放，以及工作区已有修改是否完整保留。本计划不创建 commit；后续若要求提交，必须通过完整 pre-commit CI 链，不绕过 hooks。


## 实施记录

- 两套队列已经用 UUID claim token 替换数值 lease；成功确认直接删除，重试、失败和超时回收清空 token。
- ObjDept 重复入队通过冲突行锁保护身份查询，真实 PostgreSQL 测试验证事务提交前完成操作等待、提交后可完成删除。
- 现有两张建表 SQL 和 ObjDept 超时索引已直接修改；CI 数据库已通过 `just mgr-run` → `just mgr-reset` → `just mgr-run`，并通过 `just mgr-schema` 生成 schema。
- schema 重建同时反映了原基线中四张表的 `f_deleted_at` 字段顺序；没有手工编辑生成文件或修改这些表的 SQL。
- 全工作区 `cargo test --workspace --all-features` 已通过 534 个测试，包含 feature-gated PostgreSQL 测试；覆盖范围包含计划中的分 crate 和过滤测试。
- 最终 `just fmt-check`、`just check`、`just clippy` 和完整 `sh linters-extra/run-check.sh` 全部通过；未修改 linter、未提交或部署。

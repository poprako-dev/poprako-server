# P1 修复实施计划

## 摘要

本轮实施 Comic 阶段过滤、Term 批量更新、对象任务轮询、连接池有界等待。管理员保留检查（上一版第 3 项）明确暂缓。

不修改数据库结构、不降低事务隔离级别、不限制 HTTP 在途请求数。前端可同时发起超过 4 个请求，由连接池排队，等待超过 5 秒才返回 503。

## 实现变更

1. **Comic 阶段过滤**

   - 将全库 Chapter ID 查询替换为 Comic 查询内的 typed correlated `EXISTS`。
   - 子查询关联 Comic ID，并保留 pinned、未删除及完整阶段掩码语义。
   - 无阶段过滤时不添加谓词；单阶段不支持 `Active` 时直接返回空结果。
   - 删除中间 `Vec<String>` 和第二次 ID 过滤，不改变排序、分页、include 或 HTTP 接口。
   - 不新增索引；仅在代表性数据的 `EXPLAIN (ANALYZE, BUFFERS)` 证明需要时另行处理。

2. **Term 批量更新**

   - 给 `UpsertTerms` 增加 `termbase_id`，RDB 和 mock 使用同一作用域。
   - 保留 complex 层现有 targets 合并规则。
   - 使用与 Unit 批量更新一致的 Diesel typed `CASE` 模式，一条 UPDATE 同时写入 source、targets、comment 和统一的 updated_at。
   - UPDATE 同时按 Termbase ID 和 Term ID 集合限定；重复 ID 在执行前视为内部不变量错误。
   - 校验批量 INSERT、UPDATE 的 affected rows；数量不符时返回 `Unrecoverable` 并回滚整个导入。
   - 不改变导入返回值和最多 200 个 Term 的业务限制。

3. **对象任务：30 秒轮询**

   - 将 actor 拆成 claim 循环和 maintenance 循环，由同一 supervisor 管理和统一取消。
   - 两个循环启动时立即执行一次，之后独立以 30 秒为周期。
   - claim 为空后等待 30 秒；发现任务时继续立即排空现有任务，不人为给每个任务增加 30 秒延迟。
   - maintenance 保留现有三类显式修复操作，不引入复杂 CASE；30 秒节奏已经把空闲维护降至 6 条 UPDATE/分钟。
   - 将 claim 的 SELECT + CAS UPDATE 改成 typed `UPDATE ... RETURNING`，候选子查询按现有顺序使用 `FOR UPDATE SKIP LOCKED`。
   - 保留 pending、visible_at、lease 递增、lease 溢出转 operator、attempt timeout 和 fencing 语义。
   - `ObjActor` 的 Prom 泛型增加 `Clone` 约束，用于两个受控后台循环。
   - processing timeout 仍为 3 分钟，因此最迟恢复窗口变为约 3 分 30 秒。

4. **连接池有界等待**

   - 保持 pool 最大连接数为 4，不添加 HTTP concurrency middleware。
   - Deadpool 配置 Tokio runtime 和 5 秒 wait timeout；其他连接创建或回收错误仍按基础设施故障处理。
   - 增加结构化 `RdbError::PoolWaitTimeout`，并映射为新的 `BaseError::Unavailable`。
   - HTTP 将 `Unavailable` 映射为 `503 Service Unavailable`、错误码 `9` 和通用本地化消息；不暴露驱动错误。
   - 对象任务侧仍将连接等待超时视为 retryable，后台任务不会进入 operator。
   - 在 RDB 错误产生边界记录一次驱动错误，移除上层重复日志。
   - 记录连接获取耗时 histogram 和按有限原因分类的获取失败 counter。
   - 保留现有请求速率限制。该方案只保证等待有上限，不承诺为后台任务预留连接。

5. **审计文档**

   - 更新 `docs/diesel-usage-audit.md`：将完成项改为 Resolved。
   - 管理员保留检查继续列为 deferred P1。
   - 并发项明确记录“5 秒有界等待、无 HTTP 并发限制、无后台容量预留”的剩余风险。

## 接口影响

- 成功响应、请求 DTO、分页和数据库 schema 均不变化。
- 新增 HTTP 失败语义：数据库连接等待超时返回 `503`，响应错误码为 `9`。
- 内部接口变化：`UpsertTerms` 增加 Termbase ID；`BaseError` 增加 `Unavailable`；`ObjActor` 要求 Prom 可克隆。

## 测试与验收

- Comic RDB：覆盖跨 Workset 隔离、组合阶段、所有合法 phase、单阶段非法 Active、无阶段过滤，确认结果与旧语义一致。
- Term RDB：覆盖混合批量插入/更新、200 条更新、错误 Termbase 作用域导致整批回滚、affected-row 不一致。
- Object actor：使用暂停时间测试立即首轮、30 秒间隔、繁忙时不重复 maintenance、取消与 join；RDB 测试两个 claim 竞争不重复领取、顺序稳定、lease 溢出处理。
- Pool/error：验证 max-size=4、wait-timeout=5 秒、wait timeout 分类、503/code 9 映射以及其他池错误仍为 500。
- 执行 `cargo fmt --all --check`、相关 package 定向测试、`cargo check --all-features` 和 `cargo test -p poprako-server`。

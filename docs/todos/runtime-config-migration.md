# 剩余运行时配置迁移清单

状态：暂缓，当前不实施迁移。

核对日期：2026-10-18。

本文记录仍写死在代码中、后续可以考虑通过 `AppConfig` 注入的参数。
表中的数值是核对时的代码现值，不代表已经存在对应配置项。
环境变量不在本轮范围内；协议标识、角色位、单位换算、派生探测行和缓存锁分片等内部实现常量不列入迁移清单。

已迁移的缓存预算、在线状态有效期、数据库批处理大小和邀请有效期，以
[app_config.toml](../../app_config.toml) 及 [配置类型](../../src/config.rs) 为准。

## 数据库连接池

来源：[poprako-rdb-core/src/rdb.rs](../../poprako-rdb-core/src/rdb.rs)。

| 参数 | 当前值 | 代码标识 |
|---|---|---|
| 最大连接数 | 4 | `RDB_POOL_MAX_SIZE` |
| 等待连接超时 | 5 秒 | `RDB_POOL_WAIT_TIMEOUT` |

## Prom 调度、重试与退出

| 参数 | 当前值 | 代码位置 |
|---|---|---|
| 轮询间隔 | 1 分钟 | [actor/pool.rs](../../poprako-prom/src/general/actor/pool.rs)，`POLL_INTERVAL` |
| 过期任务检查间隔 | 1 分钟 | [actor/pool.rs](../../poprako-prom/src/general/actor/pool.rs)，`STUCK_RESET_INTERVAL` |
| 重试间隔 | 5 分钟 | [actor/pool.rs](../../poprako-prom/src/general/actor/pool.rs)，`RETRY_DELAY` |
| 处理中超时 | 15 分钟 | [actor/pool.rs](../../poprako-prom/src/general/actor/pool.rs)，`PROCESSING_TIMEOUT` |
| 单次执行超时 | 14 分钟 | [actor/pool.rs](../../poprako-prom/src/general/actor/pool.rs)，`EXECUTION_TIMEOUT` |
| 死信保留时长 | 30 天 | [actor/pool.rs](../../poprako-prom/src/general/actor/pool.rs)，`DEAD_RETENTION` |
| 死信清理间隔 | 1 小时 | [actor/pool.rs](../../poprako-prom/src/general/actor/pool.rs)，`DEAD_PURGE_INTERVAL` |
| 退出宽限 | 30 秒 | [actor/pool.rs](../../poprako-prom/src/general/actor/pool.rs)，`SHUTDOWN_GRACE` |
| 等待外部状态的最长时间 | 1 小时 | [dispatch_flow.rs](../../poprako-prom/src/general/dispatch_flow.rs)，`WAIT_TIMEOUT` |
| 最大重试次数 | 3 | [dispatch_flow.rs](../../poprako-prom/src/general/dispatch_flow.rs)，`enforce_retry_limit` |
| 中止工作任务后的额外等待 | 1 秒 | [actor/worker.rs](../../poprako-prom/src/general/actor/worker.rs)，`workers.shutdown()` 的超时 |

## 对象存储任务与上传签名

| 参数 | 当前值 | 代码位置 |
|---|---|---|
| 轮询间隔 | 30 秒 | [actor.rs](../../poprako-obj-dept/src/actor.rs)，`POLL_INTERVAL` |
| 单次尝试超时 | 60 秒 | [actor.rs](../../poprako-obj-dept/src/actor.rs)，`ATTEMPT_TIMEOUT` |
| 远程操作超时 | 45 秒 | [actor/rdb_impl.rs](../../poprako-obj-dept/src/actor/rdb_impl.rs)，`REMOTE_TIMEOUT` |
| 重试间隔 | 5 分钟 | [rdb_obj_dept_prom.rs](../../poprako-obj-dept-macro/src/rdb_obj_dept_prom.rs)，`RETRY_DELAY` |
| 处理中超时 | 3 分钟 | [rdb_obj_dept_prom.rs](../../poprako-obj-dept-macro/src/rdb_obj_dept_prom.rs)，`PROCESSING_TIMEOUT` |
| 检查宽限 | 1 分钟 | [rdb_obj_dept_prom/defer.rs](../../poprako-obj-dept-macro/src/rdb_obj_dept_prom/defer.rs)，`Duration::minutes(1)` |
| URL 生成并发 | 20 | [pool.rs](../../poprako-obj-dept/src/pool.rs)，`CONCURRENCY` |
| 上传凭证生成并发 | 20 | [impl_obj_dept/lifecycle.rs](../../poprako-obj-dept-macro/src/impl_obj_dept/lifecycle.rs)，`POOL_CONCURRENCY` |
| PUT 签名有效期 | 10 分钟 | [r2_impl.rs](../../src/part_impl/obj_dept/r2_impl.rs)，`PUT_SIGNED_EXPIRATION` |

## 章节自动推进与 Scheduler 退出

| 参数 | 当前值 | 代码位置 |
|---|---|---|
| 章节自动推进延迟 | 20 分钟，两处 | [page/alloc.rs](../../src/usecase/page_source/alloc.rs)、[page/alloc/manifest.rs](../../src/usecase/page_source/alloc/manifest.rs) 中的延迟任务 |
| Scheduler 中止工作任务后的额外等待 | 1 秒 | [sched/supervisor.rs](../../src/extra/sched/supervisor.rs)，`workers.shutdown()` 的超时 |

Scheduler 的常规退出宽限已经配置化；这里仅记录中止后的额外等待。

## 业务数量上限

| 参数 | 当前值 | 代码位置 |
|---|---|---|
| 公共列表单次返回上限 | 200 条 | [pagination.rs](../../src/value/pagination.rs)，`MAX_PUB_LIST_LIMIT` |
| 每章页数上限 | 200 页 | [page_source.rs](../../src/value/page_source.rs)，`MAX_CHAPTER_PAGE_SOURCE_COUNT` |
| 每页 Unit 数量上限 | 100 个 | [unit.rs](../../src/value/unit.rs)，`MAX_PAGE_UNIT_COUNT` |
| 单次 Unit 编辑数量上限 | 100 个 | [unit.rs](../../src/value/unit.rs)，`MAX_UNIT_EDIT_COUNT` |
| 单次 Unit 变换目标数量上限 | 100 个 | [unit.rs](../../src/value/unit.rs)，`MAX_UNIT_TRANSFORM_COUNT` |
| Unit 搜索命中数量上限 | 100 个 | [unit.rs](../../src/value/unit.rs)，`MAX_UNIT_SEARCH_MATCH_COUNT` |
| 单 Unit 变换操作数量上限 | 20 个 | [instr/unit.rs](../../src/data/instr/unit.rs)，`transforms.len()` 校验 |
| 单术语库术语数量上限 | 200 条 | [termbase.rs](../../src/value/termbase.rs)，`TERMBASE_TERM_LIMIT` |
| 归档导出跨度上限 | 12 个月 | [comic_archive.rs](../../src/value/comic_archive.rs)，`MAX_EXPORT_MONTHS` |

这些上限涉及业务校验和 API 契约。后续若实施迁移，需要同时检查 DTO 校验、
用例约束、数据库查询边界、派生探测行、OpenAPI 和对应测试，不能只替换常量引用。

## 展示与指标

| 参数 | 当前值 | 代码位置 |
|---|---|---|
| 图片预览宽度 | 1080px | [r2_impl.rs](../../src/part_impl/obj_dept/r2_impl.rs)，预览 URL 生成 |
| 图片缩略图宽度 | 300px | [r2_impl.rs](../../src/part_impl/obj_dept/r2_impl.rs)，缩略图 URL 生成 |
| 图片质量 | 80 | [r2_impl.rs](../../src/part_impl/obj_dept/r2_impl.rs)，`build_image_url` |
| 系统邮件标题截断长度 | 15 | [system_mail/chapter.rs](../../src/usecase/system_mail/chapter.rs)，`TITLE_LIMIT` |
| 指标桶数量 | 60 个 | [metric.rs](../../src/api/http/shared/metric.rs)，`BUCKET_COUNT` |
| 每个指标桶覆盖时间 | 60 秒 | [metric.rs](../../src/api/http/shared/metric.rs)，`SECONDS_PER_BUCKET` |
| 指标展示窗口 | 最近 30 分钟 | [metric.rs](../../src/api/http/shared/metric.rs)，`RECENT_MINUTE_COUNT` |

## 后续实施约定

- 重新启动迁移前，先核对代码现值和调用链，再确定当次范围。
- 每项配置提供默认值；缺文件、缺分组或缺字段时使用默认值，显式非法值在启动时拒绝。
- 每项配置添加英文注释，说明用途、单位和默认值；本地或部署覆盖值需注明。
- 配置键统一标注单位：时长使用 `_secs`，容量使用 `_mib`，速率使用 `_per_second`；其他量纲也应统一命名。
- 不添加 alias 或旧字段兼容名称；未知配置键直接拒绝。
- 保持依赖显式注入，并用非默认值验证实际行为；环境变量继续单独处理。

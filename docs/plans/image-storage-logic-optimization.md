# 图片上传 Safety Contract 与 Prom 串行化修正

## Summary

实施时首先完整填写 docs/image-consistency.md，将以下状态机、锁顺序、失败语义和职责边界作为唯一规范，再按文档修改代码。

- Page、UserAvatar、TeamAvatar、ComicCover 使用同一安全协议：内容绑定 reservation、服务端验证后落账、15 分钟兜底校正。
- mark-uploaded 只确认上传，不推进 RawProvide。
- 每个待上传 Page 创建一个 15 分钟 CheckUpload；每次 Page 批量 reservation 创建一个 20 分钟 AdvanceRawProvide，即使没有新 slot。
- 单 Page replacement 在产生上传 slot 时同样创建 15/20 分钟任务。
- Prom 不回退人工完成的阶段；任务最终 Dead 后允许人工推进。
- 同时修复 prom worker 缺少 lease fencing 的队列竞态。

## Public Contract 与数据模型

- 所有 reserve 请求统一使用 image_hash + byte_length + ext；所有 mark 请求统一使用 image_version。
- image_hash 是唯一内容身份；ext 持久化并决定 key/content-type。
- byte_length 只用于 1–20 MiB reserve 校验和预签名 PUT 的 content-length 约束，不持久化、不返回、不参与 HEAD 比对；移除 Page 现有长度字段。
- 四类上传全部使用 checksum/content-length 绑定的预签名接口。
- 定义共享 upload-slot DTO：put_url + image_version + headers。
  - Page 保留批量页面响应中的 slot: Option<\_>。
  - Avatar/Cover reserve 响应也使用 slot: Option<\_>，但只是单次上传 capability。

- 相同 hash：
  - ext 不同返回 422。
  - 已 uploaded：返回 slot=None，不改状态、不重建任务。
  - pending：复用 key/version，重新签发 URL，并重新创建对应延迟任务。

- hash 改变：version 递增、生成版本化 key、置 uploaded=false，并原子创建旧 key 的专用 Delete task。
- User/Team/Comic 表新增非空 hash/ext；Page 移除 byte-length。无需兼容旧数据，直接修改对应建表 migration、down migration 并重新生成 Diesel schema；无需额外复合索
  引。

## 状态机、锁与 Prom

- Page aggregate 统一使用 Chapter FOR UPDATE → Page FOR UPDATE：
  - 覆盖 single/batch reserve、mark 落账、prom 双向校正及章节自动推进。
  - HEAD 永远在事务外；事务内重新核对完整 (id, version, key, hash, ext) 后才写入。
  - 在相关代码添加英文 NOTE:，说明统一锁顺序用于同时防止死锁与汇总竞态。

- mark：
  - 先鉴权并读取当前身份；相同 identity 已 uploaded 时幂等成功且不再 HEAD。
  - false→true 前执行 HEAD 并校验 SHA-256，随后精确 CAS。
  - 不推进或回退 RawProvide，不删除异常对象。
  - 对象缺失、hash 不符或 stale 返回 422；R2/SDK/缺 checksum 返回 500；失败不改状态。

- 15 分钟 CheckUpload：
  - 当前对象 hash 正确则精确置 true；缺失或不符则置 false。
  - 当前对象 hash 不符时，在 URL 失效后的本任务中删除异常对象。
  - stale version 或资源已删除时仅 Complete，不删除 payload key；清理由专用 Delete task 独占。加入英文 SAFETY: 注释固定此边界。
  - 相同 version 但 key 不同视为内部不变量破坏并 Dead，不猜测删除目标。
  - 不再调用 Complete/Reset RawProvide。

- 20 分钟 AdvanceRawProvide：
  - 是唯一自动阶段推进者；事务内先锁 Chapter，再检查所有 Page 的 uploaded。
  - 全 true 时完成 RawProvide；仍有 false 时直接 Complete，不重试逻辑负结果。
  - 仅基础设施错误按 5 分钟间隔有限重试 3 次，随后 Dead。
  - 仅在本次真正发生 Pending→Completed 时发送与人工推进相同的 ChapterWorkflowCompleted(RawProvide) effect。

- Page batch 即使全部内容已 uploaded，也移除同步 Complete，始终创建一个 20 分钟推进任务。
- 外部存储在成功验证后的持续丢失不属于这套一次性上传协议的保证范围。

## Worker 与运行时

- 将一次 claim 表示为 (message_id, lease)：
  - Claim 同时匹配 Pending 与 expected lease。
  - Complete/Retry/Fail 必须匹配 Processing + expected lease。
  - affected=0 表示 attempt 已过期，静默忽略其收尾，不允许覆盖新 lease 或 Dead 状态。

- 保留 stuck reset；reset 递增现有 f_lease，业务 handler 继续依赖幂等操作和 identity CAS。
- Prom handler 共享 effect developer；生产组合使用可共享引用。
- 关机顺序调整为先 drain/close prom，再 close effect developer，确保自动阶段事件不会在关机时被丢弃。

## Test Plan 与验收

- Usecase/mock：
  - 四类 reserve 的 new/reuse-pending/already-uploaded/changed-hash/ext-conflict 分支。
  - mark 的有效、缺失、hash mismatch、R2 failure、stale、重复幂等和权限分支。
  - mark 绝不推进 RawProvide；CheckUpload 绝不回退阶段。
  - single Page 与 batch 的 15/20 分钟任务数量和 delay。

- Prom handler：
  - 四类对象的 valid/missing/mismatch/stale/resource-missing/key-mismatch。
  - stale/missing 不删除，current mismatch 才删除。
  - 20 分钟任务仅在所有 Page true 时推进，并仅对真实转换发送一次 effect。

- RDB 并发测试：
  - reserve/mark/CheckUpload/AdvanceRawProvide 各种交错均遵循 Chapter→Page，无死锁、无 stale 落账。
  - HEAD 后 identity 被替换时 CAS 必须失败。
  - 旧 lease 的 Complete/Retry/Fail 无法覆盖新 attempt 或 Dead。

- HTTP 集成：
  - 更新 avatar/cover/page 客户端为实际执行 checksum-bound PUT 后再 mark。
  - 更新 it_03、it_08 及 TESTCASES.md 的契约说明与负例。

- 验证顺序：目标模块测试、RDB 并发测试、cargo fmt --check、cargo check --all-features、完整 Rust 测试、integration TypeScript typecheck。

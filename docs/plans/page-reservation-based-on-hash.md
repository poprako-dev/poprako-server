# Chapter Page 哈希清单预留、补传与重排

## 1. 文档定位

本文是本功能的最终实现规范，供后续实现 Agent 直接执行。

实现者不得自行改变本文已经锁定的接口、事务语义、匹配优先级、Published 冻结边界、Prom 重试上限或错误状态。若代码现状与本文存在无法兼容的冲突，应停止实现并向用户报告，不得自行选择替代产品语义。

本次变更直接替换现有 `/api/v1` Page reserve 请求和响应，不保留旧 `page_count + file_ext` 协议，也不新增 `/api/v2`。

### 1.1 审计结论

方案可以在当前 `Oper` / `Run` / `Step` / `Nucl::coord` 架构内实现，不要求引入新的事务模型。数据库行锁、现有 Prom 本地消息表和 R2 S3-compatible API 足以承载本文流程。

按本文完整实现后不存在因业务未完成而永久增长的资源泄漏：

- Chapter 兜底任务遇到 pending Page 会进入 Completed，不再永久 Retry。
- 普通基础设施失败最多重试 3 次，随后进入 Dead。
- Completed 和 Dead 分别在 7 天、30 天后物理清理。
- 被替换、被删除、发布后失效以及孤立上传的对象都有明确 Delete 收敛路径。
- presign 失败留下的是可由同一 manifest 幂等续签的 pending Page，不会持续生成新 key/version。

Prom 记录数和 pending Page 数仍会随真实请求量增长；这是有终态、有保留上限的业务数据，不属于泄漏。本文的“无泄漏”结论依赖第 8 节的 Retry 上限和 purge 同时落地，禁止只实现 manifest 而保留当前无限业务 Retry。

---

## 2. 最终需求与不变量

客户端提交一个 Chapter 的完整、有序 Page 图片清单。请求数组顺序就是最终 Page `index`，服务端负责：

1. 根据显式 `page_id` 或 SHA-256 哈希复用已有 Page。
2. 为新图片、被替换图片和仍未上传图片返回 presigned PUT URL。
3. 保留被复用 Page 的 ID、Units 和统计数据。
4. 删除最终清单中不存在的旧 Page、其 Units 及图片对象。
5. 原子更新 Page 顺序和 Chapter 的反规范化统计。
6. 使用对象存储 checksum 强制验证上传内容与声明的 SHA-256 一致。
7. 通过 Page 上传确认驱动 Chapter RawProvide 完成，不允许业务未完成导致 Prom 永久 Retry。
8. 不引入 Upload Session、manifest revision 或其他跨接口会话状态。

必须保持以下不变量：

- 一个 Chapter 的最终 Page 数为 `1..=200`。
- Page `index` 在 Chapter 内严格为连续的 `0..page_count-1`。
- `(chapter_id, index)` 保持唯一。
- `image_hash` 在 Chapter 内不唯一；相同图片可以出现多次。
- 不做跨 Chapter 或全站图片复用。
- SHA-256 的所有外部表示均使用 RFC 4648 标准 Base64，必须带 `=` padding。
- SHA-256 Base64 解码后必须恰好为 32 字节，规范字符串长度为 44。
- 单张图片长度必须为 `1..=20,971,520` 字节，即最多 20 MiB。
- Published Chapter 禁止用户新增或更新相关信息，但允许删除、Archive、父级级联删除和系统幂等清理。
- 除 Published 外，不因 Translate、Proofread、TypesetRedraw 或 Review 阶段状态限制 Page 清单修改；实现者不得擅自增加其他阶段 gate。
- 所有业务参数和冻结错误继续使用现有 `ExpectedVariant::Args`，映射 HTTP 422；不新增 409 Conflict。

---

## 3. 公共接口

### 3.1 批量 Page manifest

路由保持：

```text
POST /api/v1/chapters/{chapter_id}/pages/reserve
```

请求继续在 Path 和 body 中同时携带 `chapter_id`，必须沿用现有 path/body 一致性校验。

请求类型必须改为：

```rust
struct ReserveChapterPagesParams {
    chapter_id: String,
    pages: Vec<PageImageInput>,
}

struct PageImageInput {
    page_id: Option<String>,
    image_hash: ImageHash,
    byte_length: u64,
    extension: ImageExtension,
}
```

字段语义：

- `pages` 是客户端希望事务提交后的完整 Page 集合，不是增量列表。
- `page_id = Some(...)` 表示客户端明确指定要保留和重排的已有 Page 身份。
- `page_id = None` 表示由服务端在尚未被显式占用的 Page 中自动按 hash 匹配；没有候选时创建新 Page。
- `image_hash` 是图片原始字节的 SHA-256 标准 Base64。
- `byte_length` 是将直接 PUT 到 R2 的准确字节数。
- `extension` 每项必填，即使该项最终复用已上传对象也不能省略。

`pages` 为空、超过 200、包含重复显式 `page_id`、包含非法 hash、非法长度或不支持扩展名时，必须在进入事务前返回 422。

### 3.2 批量响应

响应必须与请求保持相同顺序：

```rust
struct ReserveChapterPagesPayload {
    pages: Vec<ReservedPagePayload>,
}

struct ReservedPagePayload {
    page_id: String,
    index: u32,
    image_hash: ImageHash,
    byte_length: u64,
    extension: ImageExtension,
    upload: Option<PageImageUploadPayload>,
}

struct PageImageUploadPayload {
    put_url: String,
    image_version: u32,
    headers: BTreeMap<String, String>,
}
```

JSON 中必须始终序列化 `upload`：

- `upload: null`：当前对象已经上传，不需要再次 PUT。
- `upload: {...}`：客户端必须向 URL PUT 图片。

`headers` 至少包含：

```json
{
  "content-type": "image/png",
  "x-amz-checksum-sha256": "标准 Base64 SHA-256"
}
```

客户端必须原样发送 `headers` 中的字段。精确 `Content-Length` 由服务端绑定进签名；浏览器上传原始 Blob 时由 Fetch 自动生成，客户端不得对图片字节做二次编码或转换。

### 3.3 单 Page reserve

路由保持：

```text
POST /api/v1/pages/{page_id}/image/reserve
```

请求类型改为：

```rust
struct ReservePageImageParams {
    image_hash: ImageHash,
    byte_length: u64,
    extension: ImageExtension,
}
```

响应直接使用 `ReservedPagePayload`，语义与批量接口一致。

单 Page reserve 必须按 hash 幂等：

- hash、长度和扩展名相同且已上传：不修改版本，`upload = null`。
- hash、长度和扩展名相同但未上传：复用现有 key/version，重签 URL，重新创建 CheckUpload。
- hash 不同：保留 Page 和 Units，提升 version、生成新 key、删除旧 key、返回新上传信息。
- hash 相同但长度或扩展名不同：视为客户端元数据矛盾，返回 422，不得静默换版本。

### 3.4 Page list

现有 `PageInfoVal` 必须增加：

```rust
image_hash: ImageHash,
byte_length: u64,
extension: ImageExtension,
```

客户端必须能够只依赖 Page list 返回值重建下一次完整 manifest，而不要求保留原始本地文件或上次 reserve 响应。

### 3.5 mark-uploaded

路由和请求中的 `image_version` 保持不变，但不能继续无条件信任客户端。

公开 mark-uploaded 必须：

1. 取得当前 Page 图片身份。
2. 对当前 key 执行 HEAD。
3. 验证对象存在、字节长度匹配、checksum 匹配当前 `image_hash`。
4. 在事务中再次通过 Page ID + key + version 锁定并确认身份未变化。
5. 标记上传完成并尝试完成 Chapter RawProvide。

对象不存在、checksum/长度不符或图片身份已经 stale 时返回 422；基础设施错误保持 Unrecoverable。

---

## 4. 值类型、格式与存储

### 4.1 ImageHash

在 `value` 层新增专用 `ImageHash([u8; 32])`：

- 实现 `Debug`、`Clone`、`Eq`、`PartialEq` 和 `Hash`。
- serde 序列化为标准 RFC 4648 Base64，带 padding。
- 反序列化必须严格校验规范编码：使用标准字母表、padding 正确、解码后 32 字节，并且重新编码必须与原字符串完全相同。
- 不接受 URL-safe Base64、无 padding Base64、hex、大写/小写等其他替代表示。
- 提供返回 32 字节和标准 Base64 的方法，供 Diesel 与 R2 适配器使用。

需要增加直接 `base64` 依赖，不依赖 AWS SDK 的内部 Base64 实现。

### 4.2 ImageExtension

新增受控 `ImageExtension` enum，并固定支持现有全部格式：

```text
jpg, jpeg, png, gif, webp, svg, avif, bmp, tif, tiff
```

该类型必须提供：

- lowercase serde 表示。
- object key 后缀。
- 对应 Content-Type。

不得继续从任意字符串或 object key 临时猜测 Page 的文件类型。

### 4.3 Page model

`PageInfo` 和 `PageEntry` 增加：

```rust
image_hash: ImageHash,
image_byte_length: u64,
image_extension: ImageExtension,
```

图片身份定义为：

```text
Page ID + image_version + image_key + image_hash + image_byte_length + image_extension
```

所有 version gating 必须覆盖 key；需要验证对象内容时还必须覆盖 hash 和 byte length。

### 4.4 数据库

本次明确只支持新数据库，不提供旧 Page 回填或兼容路径。直接修改 Page 建表 migration，增加：

```sql
f_image_hash        BYTEA  NOT NULL,
f_image_byte_length BIGINT NOT NULL,
f_image_extension   TEXT   NOT NULL,

CHECK (octet_length(f_image_hash) = 32),
CHECK (f_image_byte_length BETWEEN 1 AND 20971520)
```

要求：

- 不给 `(chapter_id, image_hash)` 建唯一索引。
- 保留 `(chapter_id, index)` 唯一索引。
- 通过 `just mgr-schema` 重新生成 schema，禁止直接编辑生成文件。
- Diesel entity 必须对 `u64 <-> BIGINT` 做检查转换，不能使用无保护 `as`。

---

## 5. Repository 合约

项目当前使用 `Oper`、`Run`、`Step` 和 `Nucl::coord`。实现者必须沿用当前架构，不得套用旧的 Execute/Advance 命名。

Page repository 至少新增或扩展以下 operation：

```text
ListPageInfosExcluded { chapter_id }
    -> Vec<PageInfo>
    按 index ASC, id ASC 查询并 FOR UPDATE。

ShiftPageIndexesTemporary { chapter_id }
    -> ()
    将所有现有非负 index 原子映射为 -index-1。

UpdatePageManifest { update: PageManifestUpdate }
    -> PageInfo
    更新最终 index；需要换图时同时更新 hash/length/extension/key/version/uploaded。

DeletePages::Ids { ids }
    -> ()
    先删除这些 Page 的 Units，再删除 Page。
```

`CreatePages` 必须接收包含完整图片元数据的 `PageEntry`。

Chapter repository 必须：

- 为现有 `CompleteChapterRawProvide` 增加事务内 `Step` 实现，供 Page mark 与 manifest 原子调用。
- 新增只清空 `f_uploaded_at` 的 `ResetChapterRawProvide`，不得通过写入完整 `StageMask` 顺带修改其他阶段。
- 所有自动 `StartChapterStage` 的 RDB 更新增加 `f_published_at IS NULL` 条件。

Publish 图片清理增加 repository operation，将 Chapter 下所有 Page：

- 返回旧的非空 object keys 供 Prom Delete 使用。
- `image_key = NULL`。
- `image_uploaded = false`。
- `image_version` 安全提升一次。
- 保留 `image_hash`、长度和扩展名。

所有新增 operation 必须同时实现 RDB 和 mock 适配器。依赖 PostgreSQL 唯一约束、行锁和两阶段 index 的行为必须增加 RDB 测试，不能只靠 mock。

---

## 6. 批量 manifest 精确算法

### 6.1 事务前校验

按以下顺序执行：

1. 校验 path/body Chapter ID。
2. 校验 Page 数为 `1..=200`。
3. 完成所有 hash、byte length 和 extension 解析。
4. 使用 `HashSet` 拒绝重复显式 `page_id`。
5. 执行现有 Page reserve 权限检查。

签名 URL 不能在事务前生成，因为 Page ID、版本和是否需要上传尚未确定。

### 6.2 锁顺序

所有会同时访问 Chapter 和 Page 的写流程统一使用：

```text
Chapter FOR UPDATE -> Page rows FOR UPDATE
```

批量事务必须先 `GetChapterInfoExcluded`，再 `ListPageInfosExcluded`。

单 Page reserve、公开 mark-uploaded、Unit save 及其他会更新 Page/Units 的流程也必须调整为相同顺序。不得保留 Page -> Chapter 的反向锁顺序。

### 6.3 Published gate

锁定 Chapter 后、执行任何 manifest 变更前，调用统一的 Published 写保护。Published 时返回 422 并完整回滚。

不得因为 Translate、Proofread、TypesetRedraw 或 Review 已开始而拒绝请求。

### 6.4 显式 page_id 预占

必须先处理请求中所有 `page_id = Some(...)` 项，再处理任何自动 hash 匹配。

每个显式 Page：

1. 必须存在且属于目标 Chapter，否则 422。
2. 从自动匹配候选池移除，保证不会被消费两次。
3. 当前 hash 相同但 length/extension 不同：422。
4. 当前 hash、length、extension 全部相同：
   - uploaded：原样复用。
   - pending：复用 key/version，加入待重签集合并创建新 CheckUpload。
5. 当前 hash 不同：
   - 保留 Page ID、Units 和 counters。
   - `image_version = next_version(current)`。
   - 使用新 hash、length、extension 生成新 key。
   - `image_uploaded = false`。
   - 旧 key 只要非空就创建幂等 Delete，不依赖旧 `image_uploaded`。
   - 新身份加入待签名和 CheckUpload 集合。

### 6.5 自动 hash 匹配

只使用尚未被显式 page_id 占用的旧 Page。

对每个 hash 建立多重集合。相同 hash 的候选固定按以下优先级排序：

1. `total_unit_count > 0` 优先。
2. `image_uploaded = true` 优先。
3. 旧 `index` 升序。
4. Page ID 升序。

该顺序是已锁定的产品语义，不得改回“已上传优先”，也不得使用 HashMap 的非稳定迭代顺序。

处理 `page_id = None` 项时：

- 有候选：消费队首一个 Page。
- 候选 hash 相同但 length/extension 不同：返回 422；不得把它当成新 Page 绕过矛盾。
- 候选已上传：复用并返回 `upload = null`。
- 候选未上传：复用 key/version，重签并创建 CheckUpload。
- 无候选：创建新 Page，version 从 1 开始，counters 全为 0。

### 6.6 删除未匹配旧 Page

请求 manifest 是最终权威状态。所有处理结束后仍未被消费的旧 Page 必须删除，不得返回冲突。

删除前：

- 对任何非空 image key 创建 Delete Prom，不依赖 `image_uploaded`，因为对象可能已 PUT 但尚未确认。
- 收集 Page IDs。

随后在同一事务中先删除这些 Page 的 Units，再删除 Page。

响应不需要额外返回 deleted IDs；最终 `pages` 已完整描述提交后的状态。

### 6.7 Index 两阶段更新

由于存在 `(chapter_id, index)` 唯一索引，必须使用固定两阶段算法：

1. 在所有旧 Page 已锁定后，对 Chapter 内旧 Page 执行 `index = -index - 1`。
2. 完成新 Page 创建、旧 Page 删除和 Page 身份更新。
3. 按请求数组顺序逐项写入最终 `0..N-1`。

新数据库中所有正常 index 都由应用生成且非负，因此负数区是事务内专用临时区。不得使用可能溢出的正数 offset，也不得删除唯一索引。

### 6.8 Chapter counters 与状态

不得只更新 `page_count` 或仅按删除项做增量修补。

必须根据最终保留/创建的 Page 集合重新求和：

```text
page_count
total_unit_count
translated_unit_count
proofread_unit_count
```

新 Page counters 为 0；换图 Page 保留原 counters。

最终状态：

- 存在任意 `image_uploaded = false`：只清空 `f_uploaded_at`，创建一个延迟 20 分钟的 Chapter 单次兜底任务。
- 所有 Page 已上传：事务内立即调用 `CompleteChapterRawProvide`；不等待 Prom。

其他工作流阶段保持原值。

### 6.9 Prom 与事务输出

事务内原子写入：

- Page 创建、更新、删除和 index。
- Units 删除。
- Chapter counters/RawProvide 状态。
- 旧对象 Delete tasks。
- 新或重新签发对象的 CheckUpload tasks。
- 必要的 Chapter 20 分钟兜底。
- Comic last-active。

事务输出必须绑定为一个内部结果，包含最终 Page 描述与所有需要签名的图片身份。事务提交后再并行生成 presigned URLs。

任一签名失败时整个 HTTP 请求返回错误，但已经提交的数据库状态不回滚。客户端原样重试完整 manifest 时，服务端必须复用 pending key/version 并重新签名，不得再次 bump version 或产生额外对象 key。

---

## 7. R2 checksum 与对象验证

Page 使用新的受约束上传签名接口；不要改变 User/Team/Comic 现有无 checksum reserve 行为，除非为复用内部结构所需且不改变其公共语义。

Image port 应增加通用约束类型，而不是在 R2 适配器中依赖 Page model，例如：

```rust
struct ImageUploadSpec<'a> {
    object_key: &'a str,
    content_type: &'static str,
    checksum_sha256: &'a ImageHash,
    content_length: u64,
}

struct ImageUploadTarget {
    url: Url,
    headers: BTreeMap<String, String>,
}
```

R2 `PutObject` presign 必须设置：

- object key。
- Content-Type。
- `checksum_sha256`，值为与 API 相同的标准 Base64。
- 精确 Content-Length。
- 继续使用现有 10 分钟有效期。

R2 `HeadObject` 必须启用 checksum 返回模式（AWS SDK 中设置 `ChecksumMode::Enabled`），并把响应中的 SHA-256 checksum 解析为 `ImageHash`。若 R2 未返回 SHA-256 checksum，不得降级为只校验 ETag 或长度，必须按基础设施/对象验证失败处理。

R2 bucket CORS 是部署前置条件，必须允许前端 Origin、`Content-Type` 和 `x-amz-checksum-sha256`。

Image manager 的 HEAD 结果不能再只有 bool。增加对象身份结果，至少包含：

```rust
struct ImageObjectInfo {
    byte_length: u64,
    checksum_sha256: ImageHash,
}
```

Page 手动确认和 Prom 自动确认都必须比较持久化 hash/length 与 HEAD 结果。其他图片资源只需要存在性时可以忽略额外字段。

---

## 8. 上传确认与无泄漏 Prom 生命周期

### 8.1 Page CheckUpload

继续在 Reserve 后延迟 15 分钟执行。

精确分支：

- 对象不存在：Complete，Page 保持 pending。
- 当前 Page 身份和对象 checksum/length 匹配：标记 uploaded，并在同一事务语义中尝试完成 Chapter。
- version stale：Complete，不修改当前 Page。
- Page 已删除：尝试删除该 object key；删除成功后 Complete。
- 当前 version/key 相同但对象 checksum/length 不匹配：删除错误对象；删除成功后 Complete，Page 保持 pending。
- HEAD、Delete 或数据库基础设施错误：Retry，受全局三次重试上限约束。

### 8.2 Chapter 单次兜底

延迟 20 分钟执行 `CompleteChapterRawProvide`：

- 全部 Page 已上传：设置 `f_uploaded_at`，Complete。
- 仍有 pending Page：直接 Complete，不得 Retry。
- Chapter 已完成或已删除：幂等 Complete。
- 数据库基础设施错误：Retry，受三次重试上限约束。

Chapter 的正常完成路径是每次 Page 成功确认后立即尝试完成；20 分钟任务只用于兜底，不是轮询器。

### 8.3 全局 Retry 上限

当前 `f_lease >= 3` 只处理 Processing 卡死，不能替代正常 Retry 上限。

正常 `TaskFlow::Retry` 的固定规则：

- 初次执行失败后最多再 Retry 3 次。
- `f_retried_count` 为 0、1、2 时继续 Retry 并加一。
- 已为 3 的任务再次失败时转 Dead，不再 Pending。
- 最多执行次数因此为 4 次。

Processing timeout 的 lease 恢复规则保持独立，不与 `f_retried_count` 合并。

### 8.4 终态清理

Prom purge 必须同时处理：

- Completed：`updated_at` 超过 7 天后删除。
- Dead：`updated_at` 超过 30 天后删除。

任务转 Dead 时只增加现有风格的结构化 `tracing::error!`，字段至少包含 ID、topic 和最后错误；不增加 Prometheus 指标或管理查询 API。

Delete 任务进入 Dead 后依赖结构化日志进行人工补偿。实现者不得为了“保证删除”恢复无限 Retry，否则重新引入数据库和后台负载泄漏。

---

## 9. Published Chapter 冻结

### 9.1 冻结定义

冻结范围仅是该 Chapter 聚合，不扩展到同 Comic 的其他 Chapter、Comic 元数据或 Workset。

Published 后禁止用户执行新增或更新，包括：

- Chapter metadata update。
- Chapter stage update。
- 批量 manifest reserve。
- 单 Page image reserve。
- 手动 mark-uploaded。
- Unit save/import。
- Chapter translation import。
- Assignment join 和 role update。
- Assignment invitation create/join。

这些检查必须在实际写事务中锁定 Chapter 后完成，不能只做事务外预检查。

### 9.2 明确允许的操作

Published 后仍允许：

- 所有读取和导出。
- Chapter、Page 集合、Assignment、Invitation 等现有明确 Delete 操作。
- Comic Archive；本次不新增 Chapter Archive。
- Comic/Workset 删除导致的父级级联删除。
- 发布事务自身的图片清理。
- Prom 对象删除、状态终结、过期 invitation 清理等幂等维护。

混合 save/update 接口即使 payload 只包含内部 delete oper，也仍按更新接口处理并禁止；只有现有明确的 Delete/Archive 流程获得例外。

### 9.3 统一检查与竞态

在 `complex::chapter` 增加纯检查，例如：

```rust
ChapterComplex::ensure_user_write_allowed(&chapter_info)
```

Published 时返回新的 i18n Args 错误，例如 `error-chapter-published-frozen`。

所有相关 usecase 必须遵循 Chapter 先锁的顺序。对 detached `StartChapterStage`，还必须在 RDB UPDATE predicate 中加入 `f_published_at IS NULL`，防止事务提交后的异步 stage start 穿透冻结。

### 9.4 Publish 图片终态

Publish 成功事务中：

1. 收集所有非空 Page image keys 并创建 Delete Prom。
2. 清空 Page `image_key`。
3. 设置 `image_uploaded = false`。
4. 安全提升 `image_version`，使旧 CheckUpload 成为 stale。
5. 保留 `image_hash`、byte length 和 extension 作为历史内容指纹。

因此 Published 后 Page list 不再返回指向已删除对象的失效 URL。

---

## 10. 分层实施顺序

实现必须按以下顺序推进，不得先改 HTTP 再倒推 repository：

1. **Value / Model**
   - `ImageHash`、`ImageExtension`。
   - Page image hash、长度、扩展名字段。
   - manifest update/reservation 内部模型。
2. **Migration / Entity**
   - 修改 Page 建表 migration。
   - 运行 migration/schema 生成流程。
   - 更新 Diesel entity 转换。
3. **Repository operations**
   - Page 锁定列表、临时 index、manifest update、按 IDs 删除。
   - Chapter RawProvide reset/transactional complete。
   - Publish image metadata cleanup。
   - RDB 与 mock 同步实现。
4. **Image ports/adapters**
   - checksum + length presign。
   - HEAD 对象身份。
   - mock 能注入不存在、checksum mismatch、length mismatch 和基础设施失败。
5. **Pure complex logic**
   - Published 写保护。
   - 稳定多重集合匹配与优先级；纯算法应与 I/O 分离并可单测。
6. **Usecases**
   - 批量 manifest。
   - 单页幂等 reserve。
   - 手动/Prom 上传确认驱动 Chapter 完成。
   - 删除与 Published 例外。
7. **Prom worker**
   - Chapter 单次兜底。
   - 三次 Retry 上限。
   - Dead 30 天 purge。
8. **HTTP / OpenAPI**
   - 直接替换现有 v1 DTO。
   - 同步 handler metadata、OpenAPI schemas 和错误说明。
9. **Tests / integration docs**
   - 更新 Rust 测试、TypeScript fixtures、集成套件和 `TESTCASES.md`。

---

## 11. 必须覆盖的测试

### 11.1 Value 与 DTO

- 标准 Base64 hash 正常 round-trip。
- 拒绝 hex、Base64URL、无 padding、错误 padding、非 32 字节和非规范编码。
- byte length 接受 1 和 20 MiB，拒绝 0 和 20 MiB + 1。
- 所有允许扩展名映射到正确 Content-Type。
- `upload = None` 序列化为显式 `null`。

### 11.2 Manifest 纯匹配

- 首次创建全部 Page。
- 在中间新增 Page 并保持最终顺序。
- 显式 page_id 重排不改变 Page/Units。
- 显式 page_id 换 hash 时保留 Units 并 bump version。
- 相同 hash 的重复 Page 按“有 Units -> uploaded -> index -> ID”稳定消费。
- 显式占用先于自动匹配。
- 删除所有未匹配旧 Page 及 Units。
- 拒绝重复、跨 Chapter 和不存在的显式 page_id。
- 拒绝相同 hash 但 length/extension 矛盾。
- 拒绝空 manifest 和超过 200 项。

### 11.3 Repository / RDB

- `ListPageInfosExcluded` 确实按稳定顺序加锁。
- 两个并发 manifest 被 Chapter lock 串行化，后写生效。
- Page 顺序互换不触发唯一索引临时冲突。
- 临时负 index 在提交前全部恢复为连续非负 index。
- 删除 Page IDs 时 Units 先删除。
- 最终 Chapter 四个 counters 等于 Page 聚合。
- 任一中途错误回滚 Page、Units、Chapter counters 和 Prom records。

### 11.4 单页 reserve 与上传

- 相同已上传 hash 返回 `upload = null` 且不 bump version。
- 相同 pending hash 重签同一 key/version。
- 不同 hash 保留 Units、bump version 并创建旧 key Delete。
- presigned PUT 包含 checksum、Content-Type 和 Content-Length 约束。
- 手动 mark 对缺失对象、checksum mismatch、length mismatch 和 stale version 返回 422。
- 正确 mark 后立即完成最后一个 pending Page 所属 Chapter。
- 签名部分失败时 HTTP 整体失败；相同 manifest 重试不生成新 key/version。

### 11.5 Prom 生命周期

- CheckUpload 成功后标记 Page 并完成 Chapter。
- 对象不存在时 CheckUpload Complete 且 Page 保持 pending。
- 错误对象被删除且 Page 保持 pending。
- Chapter 20 分钟兜底遇到 pending Page 直接 Complete，不 Retry。
- 正常失败初次加三次 Retry 后转 Dead。
- Processing lease 规则不受正常 retry count 影响。
- Completed 7 天、Dead 30 天 purge 边界。

### 11.6 Published 冻结

- 上述所有新增/更新入口在 Published 后返回 422 且无 mutation。
- 明确 Delete 操作在 Published 后仍成功。
- Comic Archive 和父级级联删除仍成功。
- 系统 Prom 清理仍能收敛。
- detached stage start 在 Published 后数据库层不更新。
- Publish 清理后 hash/length/extension 保留，key 清空、uploaded false、version 提升、Page list 无 image URL。

### 11.7 HTTP 集成

- 更新现有 Page reserve 集成用例为新 manifest body 和响应。
- 覆盖补页、重排、重复 hash、显式 Page 换图、隐式删除、checksum headers 和 422 分支。
- 更新所有调用旧 `reserveChapterPages(page_count, file_ext)` fixture 的集成套件。
- 同步 `tests/integration-tests/TESTCASES.md`。

---

## 12. 验证顺序

每次 Rust 编辑后运行 `cargo fmt`。最终按以下顺序验证：

```text
cargo fmt --check
cargo test -p poprako-server <ImageHash/ImageExtension filters>
cargo test -p poprako-server <page usecase filters>
cargo test -p poprako-server <prom filters>
cargo test -p poprako-server <published freeze filters>
cargo test -p poprako-server <RDB page/chapter filters>
cargo check
cargo test -p poprako-server
cd tests/integration-tests && pnpm typecheck
```

API integration suite 只有在专用 integration database 已配置时运行，并必须通过现有脚本执行。

---

## 13. 明确非目标与禁止偏离项

本次不实现：

- Upload Session。
- manifest revision 或乐观版本冲突；并发采用行锁串行化、后写生效。
- 跨 Chapter 图片复用或全站去重。
- hash 唯一索引。
- 旧数据库 Page hash 回填。
- 部分签名成功响应。
- HTTP 409 Conflict。
- 新 `/api/v2` 或兼容旧 reserve body。
- Chapter Archive。
- Published 后禁止删除或禁止 Comic Archive。
- Dead Prom 管理 API 或新 Prometheus 指标。
- 无限业务 Retry 或无限基础设施 Retry。

实现者不得把未匹配旧 Page 改回冲突，不得把自动匹配优先级改回“uploaded first”，不得省略 R2 checksum，也不得只在 HTTP handler 做 Published 检查。

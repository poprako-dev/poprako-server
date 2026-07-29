# Unit v2 的逻辑与存储

## 存储模型

一个 Page 的全部 Unit 通过 `f_next_id` 组成单链表。链中同时包含 visible
Unit 与 tombstone；`f_hidden_at IS NULL` 表示 visible，否则表示已软删除。
Tombstone 永久保留，本版本不执行回收。

`t_unit.f_id` 是全局主键。普通查询只为 `f_page_id` 建索引，不维护
`(f_page_id, f_id)`、visible 部分索引或数据库层面的唯一前驱索引。
数据库只保留 `f_next_id -> t_unit.f_id` 的延迟外键，不安装 Unit
function、trigger、CHECK 或唯一前驱约束。

跨页、自指、分叉、环、多头、断链、重复 ID 和遍历覆盖全部由业务层在
锁定 Page 并读取完整链后统一校验。持久化链损坏属于 `Unrecoverable`，
请求产生的非法链编辑属于 `Args`。

## 读语义

`GET /api/v1/pages/{page_id}/units` 读取该 Page 的完整链，在内存中按
`next_id` 重建顺序，再过滤 tombstone。返回数组顺序就是最终顺序；响应
不暴露 `index`、`next_id` 或 `hidden_at`。

Page 响应同时返回：

- `total_unit_count`：visible Unit 数；
- `translated_unit_count`：translation 或 revision 任一包含非空文本的
  visible Unit 数；
- `proofread_unit_count`：`is_proofread` 为真的 visible Unit 数。

导出、LabelPlus 与 Comic archive 使用同一有序 visible 列表。外部格式
需要 `unit_index` 时，仅在输出阶段通过 `enumerate` 临时生成。

## 写语义

`POST /api/v1/pages/{page_id}/units/save` 的请求体就是 edit 数组，每批
接受 1–100 个 edit。成功返回 `204 No Content`；客户端随后重新 list。

Edit 使用 `edit` 标签：

- `create`：包含批内 `local_id`、必填 `is_bubble` 与 `coord`；
  `next_id` 缺失表示尾插；translation/revision 可缺失。
- `patch`：包含永久 `id`；`is_bubble`、`coord` 缺失或 null 表示不修改；
  `next_id`、`translation`、`revision` 缺失或 null 表示 Skip，使用
  `{ "type": "clear" }` 表示 Clear，使用 `{ "type": "assign", "value": ... }`
  表示 Assign。
- `delete`：只包含永久 `id`，将 Unit 转为 tombstone。

Create 先为全部 `local_id` 生成永久 ID，再解析同批次主体与 `next_id`
引用。`local_id` 只在当前请求内有效，不返回映射。

同一批 edits 会先规范化：Delete 稳定前置、重复 Delete 去重、同 ID 的
Save 按字段合并，Delete 与 Save 同时出现时收敛为 Save。Patch 会恢复
hidden Unit。移动时先从包含 tombstone 的完整链摘除主体，再插入
`next_id` 前方；显式 Clear 表示尾部，Skip 保持位置。

最终 visible Unit 不得超过 100；tombstone 不占容量。

## 权限与署名

translator 只能修改 translation，proofreader 只能修改 revision；任一
角色均可创建、删除、移动以及修改结构字段。双角色可同时修改两组内容。
越权字段返回 403，未知或非法引用返回 422。

`last_translator_id` 与 `last_proofreader_id` 始终来自登录 token，客户端
不能指定。清空 revision 会同时复位 proofread 状态、文本与署名。

## 事务与并发

保存事务按 Chapter、Page、完整 Unit 链的顺序加锁。Page 锁将同页写入
串行化，因此后提交事务读取前一事务的最终链并继续合并：

- 后提交的 Patch 可以恢复先提交 Delete 产生的 tombstone；
- 同锚点插入按事务提交顺序进入链；
- 不同 Page 仍可并行写入。

仓储只写入新节点、字段变化、hide/unhide 以及发生变化的 `next_id`。
Apply 后在同一事务内重新计算 counters，更新 Page 与 Chapter，并触碰
Comic；事务提交后再触发 translation/proofread stage。

## 旧库现场切换

维护窗口执行顺序：

1. 停止旧服务写入并完成数据库备份；
2. 在单事务内锁定 `t_unit`；
3. 添加 `f_next_id`、`f_hidden_at`，使用
   `LEAD(f_id) OVER (PARTITION BY f_page_id ORDER BY f_index, f_id)`
   回填链；
4. 校验节点数、同页引用、唯一前驱、单头单尾、无环和完整覆盖；
5. 添加 `f_next_id` 外键，保留 `f_page_id` 索引并删除 `f_index`；
6. 提交后启动 v2，执行 list → save → list 冒烟验证。

现场脚本位于 `scripts/migrate-unit-v2-storage.sql`。提交前任一校验失败会
回滚整个事务；提交后的回退依赖切换前备份。

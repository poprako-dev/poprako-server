# Prom 停服手工升级

适用来源：主分支 `b514b4bf` 的两张数值 lease 队列表。目标是本分支
`a4b957cb` 已实现的固定 Topic、当前 payload、成功即删除和 UUID claim token。
脚本还支持对已完成升级的目标库重复校验；遇到两张表版本不一致会拒绝执行。
执行时使用包含这份手册和脚本的同一份分支代码。

本次只更新数据库，不运行生产部署脚本、不修改业务服务器。正常顺序是：
**停止所有业务进程 → 备份 → 预检与必要的数据核查 → 升级 → 校验 → 交回 CI/CD。**
PostgreSQL 自身保持运行。停服必须覆盖 API、通用 Prom、ObjDept actor、scheduler
及其他会访问这两张队列表的旧进程。

## 更新范围

| 对象 | 处理 |
| --- | --- |
| `t_local_message` | 移除 `f_lease`，增加 nullable UUID `f_claim_token` 及状态一致性 CHECK |
| `t_obj_prom_task` | 同上；对象 ID、版本、key、generation 和任务 ID 均不改 |
| 通用 Prom 待执行数据 | 转换三种旧 payload，topic 改为 `chapter` / `invitation` |
| `Completed` | 从两张任务表删除已经确认成功的记录；原记录仍在停服备份中 |
| `Dead` / `Operator` | 保留原始内容、诊断和重试次数，不重新投递；Dead 原始 payload 可能仍是旧格式 |
| `Processing` | 原库有未核实执行结果的任务就阻止升级，不自动重跑或判定成功 |
| 索引 | 删除旧通用 Prom 条件索引，保留两个普通联合索引；重建 ObjDept 超时索引，移除 lease 字段 |

待执行任务的 `f_id`、`f_created_at`、`f_updated_at`、`f_visible_at`、重试次数和错误内容
全部保留。两个邀请操作共用 `invitation` topic。不会创建永久兼容函数、兼容表或运行时解码分支。

生成的 `schema.rs` 中四张业务表的 `f_deleted_at` 顺序变化只是列展示顺序，
不需要重建这些业务表。本次脚本不改任何业务表。

## 1. 连接与备份

需要 `psql`，备份建议使用 PostgreSQL 18 的 `pg_dump`。
在执行机器上设置 `DATABASE_URL` 为你已有的生产数据库连接，确认数据库名为
`db_poprako_server_prod`。以下命令均在这份代码的根目录执行。

```sh
psql "$DATABASE_URL" -X -v ON_ERROR_STOP=1 -c 'SELECT current_database(), version();'
umask 077
prom_backup="prom-before-upgrade-$(date +%Y%m%d-%H%M%S).dump"
pg_dump "$DATABASE_URL" --format=custom --file="$prom_backup"
pg_restore --list "$prom_backup" > "$prom_backup.list"
```

确认备份命令成功并保留备份文件。`pg_restore --list` 只校验归档可读取，
不能替代在隔离数据库中恢复演练。不要在生产执行 `diesel migration revert --all`、
`just mgr-reset` 或测试脚本。

## 2. 预检

```sh
psql "$DATABASE_URL" -X -v ON_ERROR_STOP=1 \
  -f scripts/prom-upgrade/preflight.sql > prom-preflight.log 2>&1
```

查看日志和退出码。预检不修改表或业务数据，会输出：

- 两张表的 lease/token 列及各状态数量。
- 每条遗留 Processing 的队列、ID、topic 和更新时间。
- 无法转换的待执行 payload，包括缺少 `actor_user_id` 的章节任务。
- 创建超过一小时的待执行章节任务及全部队列索引。

有 Processing 或无法转换的待执行 payload 时，命令以非零状态退出。
修复后重新执行预检。升级脚本还会在取得表锁后再次检查，预检成功不能代替升级校验。

### 遗留 Processing 的处理

按照 [Prom 重放约束](../src/part_impl/prom/NOTE.md)逐条核实完整业务结果，
包括事务提交和副作用。不能仅凭“服务器停了”“超时了”或一个错误名称判定安全重跑。

| 已核实的结果 | 升级前可以作出的明确处理 |
| --- | --- |
| 完整成功，任务确认未完成 | 仅删除该任务行 |
| 未执行，或完整回滚且没有副作用，允许重新执行 | 仅将该任务改为 Pending |
| 结果不明，需要继续核查 | 保持 Processing 并暂停升级；或明确将通用任务转为 Dead、对象任务转为 Operator，保留诊断以便人工处理 |

例如，以下仅适用于**已证明允许重新执行**的那一条通用 Prom 任务：

```sql
UPDATE public.t_local_message
SET f_status = 'local_message_status:pending'
WHERE f_id = '已经核实的任务ID'
  AND f_status = 'local_message_status:processing';
```

对象任务对应 `t_obj_prom_task`，状态前缀为 `obj_prom_status:`。
这些处置在旧 schema 上进行；记录依据及影响行数。脚本不会代替你作出业务结果判断。
如果任何任务转入 Dead/Operator，必须记录后续核查责任，不能把它算作“已经完成”。

### 缺少操作者的章节任务

从可信日志等来源恢复实际操作者，不能自动采用章节创建者、管理员或空字符串。
原主分支 payload 的单条修复示例：

```sql
UPDATE public.t_local_message
SET f_payload = jsonb_set(
    f_payload,
    '{AdvanceRawProvide,actor_user_id}',
    to_jsonb('核实后的实际用户ID'::text)
)
WHERE f_id = '已经核实的任务ID'
  AND f_topic = 'advance_raw_provide'
  AND f_status = 'local_message_status:pending';
```

无法恢复时停止这条任务的自动迁移，先明确业务处理结果。脚本不会伪造操作者。

### 超过一小时的章节任务

新版仍有一小时 Wait 上限：这些任务执行后如果仍返回 Wait，会进入 Dead；
成功或已无需推进的结果不受此限制。迁移保留原始时间，不延长 deadline。
预检列出它们供你决定是否接受现行策略；schema 更新本身不会修正这项业务策略。

## 3. 执行升级

保持所有业务进程停止：

```sh
psql "$DATABASE_URL" -X -v ON_ERROR_STOP=1 \
  -f scripts/prom-upgrade/apply.sql > prom-upgrade.log 2>&1
```

只有命令退出码为 0 且事务 COMMIT 成功，才进入下一步。
脚本锁定两张任务表；五秒内拿不到锁会失败，语句超时上限为五分钟。
任何数据、schema 或索引校验失败，都回滚这次事务中的转换、删除、DDL。
不会部分升级一张表后继续部署。

重复执行已升级的目标库只验证契约，不修改 payload、任务状态、时间或 claim token。
连接在提交时中断、无法判断结果时，先执行下一节校验；不要直接启动旧服务器。

## 4. 校验并交回 CI/CD

```sh
psql "$DATABASE_URL" -X -v ON_ERROR_STOP=1 \
  -f scripts/prom-upgrade/verify.sql > prom-verify.log 2>&1
```

命令必须成功，输出应满足：

- 两张表都有 nullable UUID `f_claim_token`，都没有 `f_lease`。
- 两条 claim-token CHECK 存在且已验证。
- 活跃通用任务均可匹配当前 payload 与固定 topic；已确认 Completed 行已删除。
- 通用 Prom 仅使用两个新的普通联合索引；旧条件索引不存在。
- ObjDept 的 `i_obj_prom_task_stuck` 只有 `(f_status, f_updated_at)`。

正常迁移后的非完成任务总数等于迁移前总数减 Completed 数量，脚本在提交前强制检查。
如你在预检阶段人工处置过任务，以处置后的数量为基准。

验证通过后才触发新版本部署。已演练本分支全部 `up.sql` 在升级库上连续重放两次，
不会再次转换数据、重置状态或替换新领取的 UUID token。

**当前仓库的 CI/CD 会自行启动新容器，并在失败时尝试启动旧容器。**
它不是“部署完等待手动启动”的模式。本次未修改该流程。
升级提交后旧程序仍会访问已删除的 `f_lease`、旧 topic 和旧 payload，不能再启动旧版。
因此本次发布的容器恢复必须禁止退回旧版；CI/CD 失败时应保持停机处理，不能依赖其现有自动回滚。

升级提交后、任何新业务进程都尚未启动且没有新写入时，可以使用停服备份恢复旧库再恢复旧版。
新业务已经运行后，不能直接覆盖为停服前快照，也不能只切换旧镜像；需要核对新产生的业务结果。

## PostgreSQL 仅在 Docker 容器内可访问时

将整个目录复制到数据库容器，避免 `\ir contract.sql` 找不到配套文件：

```sh
prom_db_container='你的PostgreSQL容器名'
docker cp scripts/prom-upgrade "$prom_db_container":/tmp/prom-upgrade
```

备份示例：

```sh
umask 077
docker exec "$prom_db_container" sh -eu -c \
  'exec pg_dump -U "$POSTGRES_USER" -d "${POSTGRES_DB:-$POSTGRES_USER}" -Fc' \
  > prom-before-upgrade.dump
```

依次将下面的 `preflight.sql` 改为 `apply.sql`、`verify.sql`，每一步都检查退出码与输出：

```sh
docker exec "$prom_db_container" sh -eu -c \
  'exec psql -X -v ON_ERROR_STOP=1 -U "$POSTGRES_USER" -d "${POSTGRES_DB:-$POSTGRES_USER}" -f /tmp/prom-upgrade/preflight.sql'
```

这些是由你手工执行的数据库操作。无需从维护机运行 `ga-remote-deploy.sh`
或 `ga-apply-migrations.sh`，也无需运行本地发布构建。

## 验证脚本

仅对已授权 disposable `db_poprako_ci` 执行：

```sh
CI_MIGRATION_DATABASE=1 DATABASE_URL='postgres://.../db_poprako_ci' \
  sh scripts/test-prom-upgrade.sh
```

覆盖旧库升级、三种 payload、未来延迟任务、时间和身份保留、Completed 删除、
Dead/Operator 保留、Processing 拒绝、缺少操作者、歧义 payload、未知状态、混合 schema、
索引不匹配时完整回滚、重跑保留已领取 token、两次完整 CD 重放、与新库结构一致，
最后执行仓库规定的 apply → revert-all → apply。它会重建 CI 库中的队列表，禁止用于生产。

CI 的 migrations job 已调用此脚本，因此每次检查同时覆盖手工升级与原有 Diesel 验证链。

# Unit Save API

`POST /api/v1/pages/{page_id}/units/save`

对页面内的翻译单元（Unit）进行 创建 → 保存 → 删除 操作。操作以有序事件流提交，服务端按顺序逐个应用。

---

## API 对比：v1（Save+Delete）→ v2（Create+Save+Delete）

### 旧版（Save+Delete）

旧版只有一个 `save` 操作，靠字段是否存在来推断意图：

| 意图 | 传入字段 | 推断逻辑 |
|------|----------|----------|
| 新建 | `local_id` 非空，`id` 空 | 有 `local_id` → 新建 |
| 更新 | `id` 非空，`local_id` 空 | 有 `id` 且至少一个可变字段 → 更新 |

```json
{
  "oper": "save",
  "local_id": "temp-uuid",    // ← 新建
  "id": null,                  // ← 新建
  "payload": { ... }           // ← flatten
}
```

### 新版（Create+Save+Delete）

每个操作有明确的 `oper` 标签，不再靠字段推断：

| 操作 | `oper` 值 | 标识字段 |
|------|-----------|----------|
| 新建 | `"create"` | `local_id`（客户端临时 ID）|
| 更新 | `"save"` | `id`（服务端已有 ID）|
| 删除 | `"delete"` | `id`（服务端已有 ID）|
| 移动 | 通过 `before_id`，不单独操作 | — |

**JSON 字段直接平铺，不再嵌套 `payload`**。`deny_unknown_fields` 开启，多传字段直接 422。

---

## 当前请求格式

```json
{
  "page_id": "page-1",
  "diff": {
    "page_id": "page-1",
    "opers": []
  }
}
```

两个 `page_id` 都必须与 URL 路径中的 `{page_id}` 一致。

---

## 操作说明

每个操作携带完整的 unit 载荷（删除除外），用于并发场景下的 replay 恢复。

**统一载荷字段：**

| 字段 | 类型 | 说明 |
|------|------|------|
| `is_bubble` | `bool` | 是否是气泡 |
| `is_proofread` | `bool` | 是否已校对 |
| `x_coord` | `f64` | X 坐标 |
| `y_coord` | `f64` | Y 坐标 |
| `translated_text` | `string \| null` | 翻译文本 |
| `last_translator_id` | `string \| null` | 翻译者 ID |
| `proofread_text` | `string \| null` | 校对文本 |
| `last_proofreader_id` | `string \| null` | 校对者 ID |

**规则：** 如果 `translated_text` 或 `proofread_text` 为非空，对应的 `last_translator_id` / `last_proofreader_id` 必须非空。纯空文本不需要对应编辑者 ID。

---

### Create — 新建

创建一个新 unit，使用客户端临时 ID。服务端返回临时 ID ↔ 服务端 ID 的映射。

```json
{
  "oper": "create",
  "local_id": "local-uuid-1",
  "before_id": "unit-b",
  "is_bubble": true,
  "is_proofread": false,
  "x_coord": 12.5,
  "y_coord": 33.0,
  "translated_text": "new text",
  "last_translator_id": "user-1",
  "proofread_text": null,
  "last_proofreader_id": null
}
```

| 字段 | 必填 | 说明 |
|------|------|------|
| `local_id` | ✅ | 客户端生成的临时标识符，用于在响应中匹配服务器 ID |
| `before_id` | ❌ | 放置在此 unit 之前。省略或 null 则放在末尾 |

---

### Save — 保存/恢复

用服务端 ID 更新已有 unit。如果该 unit 刚被删除（例如因并发），服务端会将该 unit 恢复至末尾后再更新。

```json
{
  "oper": "save",
  "id": "unit-a",
  "before_id": "unit-c",
  "is_bubble": true,
  "is_proofread": true,
  "x_coord": 10.0,
  "y_coord": 20.0,
  "translated_text": "translated",
  "last_translator_id": "user-1",
  "proofread_text": "proofread",
  "last_proofreader_id": "user-2"
}
```

| 字段 | 必填 | 说明 |
|------|------|------|
| `id` | ✅ | 服务端已知的 unit ID |
| `before_id` | ❌ | 放置在此 unit 之前。省略或 null 则保持原位或放在末尾 |

---

### Move Before

新版没有单独的移动操作。移动就是一个附带 `before_id` 的 save。如果只想移动位置不修改内容，照常传完整 payload 即可。

---

### Delete — 删除

```json
{
  "oper": "delete",
  "id": "unit-a"
}
```

Delete 不带任何载荷字段。删除一个不存在的 unit 也是合法操作（幂等）。

---

## `before_id` 定位规则

所有 rule 一致：

| 情况 | 结果 |
|------|------|
| `before_id` 省略或 `null` | 插入到末尾 |
| `before_id` 等于自身 ID | 插入到末尾（忽略自引用）|
| `before_id` 指向已删除/不存在的 ID | 插入到末尾 |
| `before_id` 指向存在的 ID | 插入到该 ID 之前 |

---

## 响应格式

```json
{
  "local_id_mappers": [
    {
      "local_id": "local-uuid-1",
      "unit_id": "snowflake-server-id"
    }
  ],
  "total_unit_count": 3,
  "translated_unit_count": 2,
  "proofread_unit_count": 1
}
```

| 字段 | 说明 |
|------|------|
| `local_id_mappers` | Create 操作的临时 ID → 服务端 ID 映射。Save/Delete 不会在此出现 |
| `total_unit_count` | 该页最新总 unit 数 |
| `translated_unit_count` | 该页最新已翻译 unit 数 |
| `proofread_unit_count` | 该页最新已校对 unit 数 |

**注意：** 响应不返回 unit 顺序。提交后调用 `GET /api/v1/pages/{page_id}/units` 获取最新顺序。

---

## 迁移指南（Save+Delete → Create+Save+Delete）

### 前端改动点

#### 1. 操作结构体变更

```typescript
// 旧版（不要再用了）
interface UnitOperSaveDelete {
  oper: 'save' | 'delete';
  local_id?: string;     // 可选，用于新建
  id?: string;           // 可选，用于更新
  payload?: UnitPayload;  // flatten 前的嵌套
}

// 新版
type UnitOper =
  | { oper: 'create'; local_id: string; before_id?: string; is_bubble: boolean; is_proofread: boolean; x_coord: number; y_coord: number; translated_text: string | null; last_translator_id: string | null; proofread_text: string | null; last_proofreader_id: string | null }
  | { oper: 'save';    id: string; before_id?: string; is_bubble: boolean; is_proofread: boolean; x_coord: number; y_coord: number; translated_text: string | null; last_translator_id: string | null; proofread_text: string | null; last_proofreader_id: string | null }
  | { oper: 'delete';  id: string };
```

#### 2. 载荷展开

```typescript
// 旧版 — payload 嵌套
{ oper: 'save', local_id: 'tmp', payload: { is_bubble: true, x_coord: 1.0 } }

// 新版 — 字段平铺
{ oper: 'create', local_id: 'tmp', is_bubble: true, x_coord: 1.0 }
```

#### 3. 创建不再用 `save` 冒充

```typescript
// 旧版（新建 = save 带 local_id）
{ oper: 'save', local_id: 'uuid', payload: { ... } }

// 新版（新建 = create）
{ oper: 'create', local_id: 'uuid', is_bubble: true, ... }
```

#### 4. 移动不再用 `id` 推断

```
// 旧版：携带 payload 的 save 带 before_id = 移动
// 新版：save 或 create 带 before_id 就自然有定位效果
//        无需额外字段，无需区分"移动"和"更新"
```

#### 5. 删除不再接收多余字段

```typescript
// 旧版可能残留的写法（仍然能解析但不推荐）
{ oper: 'delete', id: 'xxx', is_bubble: false }  // ❌ 新版会拒绝：deny_unknown_fields

// 新版正确写法
{ oper: 'delete', id: 'xxx' }                      // ✅ 只有 id
```

### 6. 旧版响应只有计数器，新版新增 `local_id_mappers`

```typescript
// 旧版响应
{ total_unit_count: 3, translated_unit_count: 2, proofread_unit_count: 1 }

// 新版响应
{
  local_id_mappers: [{ local_id: 'temp-1', unit_id: 'snowflake-xxx' }],
  total_unit_count: 3,
  translated_unit_count: 2,
  proofread_unit_count: 1
}
```

前端须从 `local_id_mappers` 取出相应的 mapping，将本地创建的临时 ID 替换为服务端 ID。

---

## 前端操作流程建议

1. 前端维护本地 unit 列表（含临时 ID）
2. 用户增删改后，计算 diff → 生成 opers 数组
3. 提交 `POST /api/v1/pages/{page_id}/units/save`
4. 根据响应中的 `local_id_mappers` 将本地临时 ID 替换为服务器 ID
5. 调用 `GET /api/v1/pages/{page_id}/units` 刷新最新顺序

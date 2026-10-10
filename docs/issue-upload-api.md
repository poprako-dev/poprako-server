# Issue 上传接口

接口用于一次性导入某个 Chapter 的当前监稿问题。导入会整体替换已有问题。

## 请求

`POST /api/v1/chapters/{chapter_id}/issues/import`

请求头：`Content-Type: application/json`

```json
{
  "pages": [
    {
      "page_artwork_id": "composite-page-id",
      "issues": [
        {
          "variant": "文字位置",
          "layer_name": "对白图层",
          "rect": {
            "x_coord": 0.1,
            "y_coord": 0.2,
            "width": 0.3,
            "height": 0.1
          },
          "note": "向左移动"
        }
      ]
    }
  ]
}
```

- `pages`：本次导入涉及的复合图页；可以为空。
- `page_artwork_id`：目标复合图页 ID，必须属于路径中的 Chapter，且不能重复。
- `issues`：该页的问题列表；数组顺序决定从 `0` 开始的 `index`。
- `variant`：必填，不能是空白字符串。
- `layer_name`：可选，图层可读名称；不能是空白字符串。省略或 `null` 表示整个复合图。
- `rect`：可选，基于整张复合图的归一化矩形。坐标范围为 `[0, 1]`，宽高必须大于 `0`，矩形须完全位于图内。
- `note`：必填字符串，可为空，也保留换行。

## 成功响应

HTTP `200`，标准成功 envelope：

```json
{
  "code": 0,
  "data": {
    "imported_page_count": 1,
    "imported_issue_count": 1
  }
}
```

`imported_page_count` 是请求中 `pages` 的数量（包括空问题列表的页）；`imported_issue_count` 是导入的问题总数。

## 权限与行为

- 仅该 Chapter 当前指派的 `REVIEWER` 可以上传；仅有管理员身份不够。
- 已发布的 Chapter 不接受导入。
- 每次成功导入都会替换该 Chapter 的全部当前问题并生成新的 Issue ID。未出现在 `pages` 中的复合图页，其旧问题会被清除。
- `pages: []` 或所有 `issues: []` 会清空当前问题，但不会删除复合图页或图片，也不会推进审核流程或修改 Units。
- 未知字段会被拒绝。问题目标必须使用复合图页 ID；导入不会调整复合图顺序。

## 错误

- `403`：当前用户没有该 Chapter 的 REVIEWER 指派权限。
- `422`：Chapter 不存在、已发布，或请求字段/目标复合图页不合法。

更完整的复合图和监稿生命周期说明见 [chapter-issues.md](chapter-issues.md#review-import)。

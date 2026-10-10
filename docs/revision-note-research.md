# Revision note：MangaProof 元数据与整章导入

> 背景研究：本文记录 MP 原生格式，不作为当前 PRK 导入协议。当前协议见 [chapter-issues.md](chapter-issues.md)：使用可读的 `layer_name`，不保存数字图层索引路径；一个 Chapter 最多一份当前监稿，整章导入整体替换。

调研日期：2026-10-09。本文保留调研时的 MP 原生格式分析与建议；已实现的后端契约以 [chapter-issues.md](chapter-issues.md) 为准。

## 调研时确认的范围

用户确认：note 离散地按 Page 存储；目前只支持从 MangaProof 元数据
文件解析并写入。每次提交整个 Chapter，没有单页写入接口。

因此，“类似 Unit”只指 Page 归属与结构化数据。当前范围不需要 Unit 的
局部编辑批次、链式排序、tombstone 恢复、save receipt 或编辑草稿协议。

分析使用当前工作区源码。Server HEAD 为
`5c3299671fc92595ab632fb2f79e4779af3f57de`；Web HEAD 为
`e854c3e8d36efaf4571c24b7ba7207afaf0f2663`，但 reviewer 位于用户尚未提交的
工作区改动中，不能只依据该 HEAD 复现当前前端。

## MangaProof 的模型

仓库为 [gunfub/MangaProof](https://github.com/gunfub/MangaProof)，核对版本为
`b4515fd1184ce6a6e77f89c4a4b42b564f30930e`。

### 问题分类与几何表示

默认提供 21 种问题分类。以下分组仅为本文便于阅读，源码是一个平铺列表：

| 分组 | 默认分类 |
| --- | --- |
| 文字样式与位置 | 居中错误、字体选择错误、字体字重错误、文字描边粗细错误、文字颜色错误、字号错误、文字位置错误、文字间距错误 |
| 气泡与修图 | 气泡处理错误、原文字擦除错误、背景擦除错误、网点对齐错误、网点残留、修图瑕疵 |
| 文字与排版 | 漏翻、漏字、错字、翻译错误、排版错误、文字溢出 |
| 其他 | 其他 |

分类在配置中以名称与快捷键存储，Issue 的 `type` 是字符串，读取配置时也
接受非默认名称；21 项不是封闭枚举。依据：
[默认列表](https://github.com/gunfub/MangaProof/blob/b4515fd1184ce6a6e77f89c4a4b42b564f30930e/mangaproof/config/settings.py#L80-L102)、
[配置读取](https://github.com/gunfub/MangaProof/blob/b4515fd1184ce6a6e77f89c4a4b42b564f30930e/mangaproof/config/settings.py#L974-L987)。

几何表示只有轴对齐矩形，没有点、箭头、多边形等 tagged union。
矩形用 PSD 整页像素坐标表达，不是图层局部坐标，也不是 0–1 相对坐标。
无框批注仍然使用同一个 Issue，矩形为 `(0, 0, 0, 0)`；
Ctrl+Enter 的默认分类为“其他”，自由文本写入 comment。
依据：[Issue 模型](https://github.com/gunfub/MangaProof/blob/b4515fd1184ce6a6e77f89c4a4b42b564f30930e/mangaproof/review/issue.py#L17-L44)、
[无框创建入口](https://github.com/gunfub/MangaProof/blob/b4515fd1184ce6a6e77f89c4a4b42b564f30930e/mangaproof/ui/main_window.py#L2152-L2163)。

### Issue 到现有前端字段的映射

| MangaProof | 当前 Web RevisionNote | 转换说明 |
| --- | --- | --- |
| `issue_id` | `id` | 来源 UUID hex；重复导入应保持可识别的来源身份 |
| `issue_no` | `number` | 任务内编号；不能用当前页数组下标覆盖 |
| `file` | 加载上下文的 Page | 源文件相对路径须映射到目标 Chapter 的 Page ID |
| `layer_id` | `layerId` | PSD 树索引路径；不是 Unit ID |
| `layer_name` | 无直接字段 | 可作为没有 PSD 图层树时的显示名快照 |
| `type` | `type` | 保留开放字符串 |
| `comment` | `content` | 保留换行与原始文字；空字符串也有意义 |
| `rect: {x,y,w,h}` | `rect: {xCoord,yCoord,width,height} \| null` | 无框转换为 null；有框需要像素到相对坐标转换 |

源字段依据：[序列化与读取](https://github.com/gunfub/MangaProof/blob/b4515fd1184ce6a6e77f89c4a4b42b564f30930e/mangaproof/review/issue.py#L30-L68)。

`layer_id` 与 Web 当前算法使用相同形态：根图层如 `0.0`、`0.1`，嵌套
图层如 `0.1.0`。索引来自 PSD 树，不能先过滤隐藏层再重新编号；同名图层
也不能按名字匹配。依据：
[MangaProof 图层建树](https://github.com/gunfub/MangaProof/blob/b4515fd1184ce6a6e77f89c4a4b42b564f30930e/mangaproof/psd/document.py#L188-L246)、
[Web 图层建树](../../poprako-web/src/route/_authenticated/reviewer/business/revision-note/psd-layer.ts)。

### 编号和状态不要混用

Issue 创建时在整个任务现有编号的最大值上加一；删除后可能留下空号。
工具另有显式编号重排，按文件、图层与问题顺序生成连续编号。
PDF 报告又会按每个 PSD 的问题列表从 1 编号，未必等于元数据的 issue_no。
当前只导入元数据，应保留 issue_no，并允许页内编号不连续。
依据：[创建与删除](https://github.com/gunfub/MangaProof/blob/b4515fd1184ce6a6e77f89c4a4b42b564f30930e/mangaproof/review/state.py#L113-L145)、
[显式重排](https://github.com/gunfub/MangaProof/blob/b4515fd1184ce6a6e77f89c4a4b42b564f30930e/mangaproof/review/numbering.py#L62-L150)、
[报告排序与编号](https://github.com/gunfub/MangaProof/blob/b4515fd1184ce6a6e77f89c4a4b42b564f30930e/mangaproof/report/generator.py#L512-L626)。

Issue 自身没有解决状态。`reviews` 单独存放图层的 unreviewed、passed、failed；
文件汇总另有 partial。不能将图层通过状态翻译成单条 note 的 is_resolved，
也不能据此决定导入时丢弃某条问题。当前范围只需导入 issues。
依据：[状态和任务模型](https://github.com/gunfub/MangaProof/blob/b4515fd1184ce6a6e77f89c4a4b42b564f30930e/mangaproof/review/state.py#L18-L97)。

## 元数据格式与转换限制

单文件任务保存为 `<stem>.mangaproof.json`，文件夹任务保存为
`.mangaproof.json`。当前 schema_version 为 1。

顶层包括任务身份、任务类型、原始 source 路径、当前文件/图层、files、
reviews、issues 和时间。`files` 记录相对路径、文件名、字节大小、mtime
以及可选的抽样 SHA-256。它不记录 PSD 的 width/height。
依据：[持久化路径](https://github.com/gunfub/MangaProof/blob/b4515fd1184ce6a6e77f89c4a4b42b564f30930e/mangaproof/review/persistence.py#L25-L49)、
[FileRecord 与 TaskState](https://github.com/gunfub/MangaProof/blob/b4515fd1184ce6a6e77f89c4a4b42b564f30930e/mangaproof/review/state.py#L42-L90)、
[任务序列化](https://github.com/gunfub/MangaProof/blob/b4515fd1184ce6a6e77f89c4a4b42b564f30930e/mangaproof/review/state.py#L213-L249)。

这带来两个实质性问题：

1. **单靠 JSON 不能可靠地产生前端相对矩形。** 转换需要对应 PSD 的画布
   宽高 W/H：`xCoord=x/W`、`yCoord=y/H`、`width=w/W`、`height=h/H`。
   不能默认拿原稿图片尺寸代替成稿 PSD 画布尺寸。
2. **文件路径不能直接当 Page ID。** file 是任务根目录下的 POSIX 相对
   路径，可能含子目录。不能只按 basename、文件数组位置或问题出现顺序
   静默匹配；无问题的页仍应参与完整文件映射。

第一个问题的最小方案是后端保留来源像素矩形，Web 在取得对应
RevisionPage 的 width/height 后转换为现有 RevisionNote。这样上传仍只需要
元数据文件，不要求后端解析 PSD。后端传输字段必须明确标注像素单位，不能
把未归一化的数据直接塞进现有相对矩形契约。

如果后端响应一定要直接提供 0–1 矩形，则另需可靠的 PSD 尺寸来源；此时
“只解析一个元数据 JSON”不足以完成全部转换。

来源自动框选可能向画布外扩，产生负坐标或超出画布的矩形。导入不宜简单
clamp，因为会改变原标注；应验证有限数值和正宽高，显示裁剪另行处理。
归一化之后仍可能超出 0–1；接入时应明确扩展现有前端的坐标约定。
`(0,0,0,0)` 的无框约定应显式识别，其他退化或损坏矩形应报错。
依据：[自动框选](https://github.com/gunfub/MangaProof/blob/b4515fd1184ce6a6e77f89c4a4b42b564f30930e/mangaproof/camera/centering.py#L64-L93)。

## 当前 poprako-web 的实际契约

[RevisionNote](../../poprako-web/src/route/_authenticated/reviewer/business/revision-note/revision-note.ts)
只有 id、number、type、content、可选 rect、可选 layerId；pageId 由加载上下文
提供。[CONTEXT.md](../../poprako-web/CONTEXT.md) 已明确它不关联翻译 Unit。

- type 是字符串，样例包含“断行”和“自定义：叠字节奏”；颜色由类型字符串
  稳定散列产生，不能从颜色反推类别。
- rect 和 layerId 彼此独立。整页无框说明、整页区域问题、图层无框批注都可
  表达，不要附加“有框必须有图层”约束。
- 加载控制器按 number 排序；图层筛选保留原编号，选择组时也能包含后代。
- 当前 UI 是只读展示，生产 WebReviewer 的两个资源加载器仍为 null。

依据：[样例](../../poprako-web/src/route/_authenticated/reviewer/business/test/revision-note-story-fixture.ts)、
[外观](../../poprako-web/src/route/_authenticated/reviewer/business/revision-note/revision-note-appearance.ts)、
[加载控制器](../../poprako-web/src/route/_authenticated/reviewer/business/revision-note/revision-page-controller.ts)、
[筛选](../../poprako-web/src/route/_authenticated/reviewer/business/revision-note/revision-page.ts)、
[只读项](../../poprako-web/src/route/_authenticated/reviewer/business/revision-note/RevisionNoteItem.tsx)、
[生产入口](../../poprako-web/src/route/_authenticated/reviewer/business/WebReviewer.tsx)。

## 最小实现建议

### 存储与写入

使用独立的 revision_note 模型，按 page_id 存储。保留来源身份、原编号、
分类、批注、像素矩形、图层路径；需要脱离 PSD 显示时再保留 layer_name
快照。不引入 next_id、translation/revision 内容组或 Unit 进度计数。

建议把一次整章提交定义为**完整替换该章已有导入结果**：先解析并验证整个
文件与全部 Page 映射，再在一个 Chapter 范围的事务内替换该章所有 note。
不含 issues 的页自然清空旧 note；合法的空 issues 表示该章当前没有问题。
这属于建议的替换语义，需在实现契约中明确，不能实现成不完整的按页 upsert。

导入失败应保持旧结果完整。重复提交同一快照应得到相同的业务数据；目前
没有增量 create，因此不需要复制 Unit 的 save_id/receipt 机制。来源
issue_id 应在声明的 Chapter/任务作用域内校验重复，避免重复导入时 UI
身份漂移，也不要假定两个不同 Chapter 永远不会上传同一来源任务。

并发整章导入可以串行化后以后提交快照为准。权限使用 Chapter 的读取和
导入规则；Unit 的 translator/proofreader 字段权限不适用于此模型。

### 接口与前端接入

接口可仅提供 Chapter 级 import 与 list：导入接受完整 MangaProof 元数据，
读取返回带 page_id 的结构化 note。Web 将整章结果按 Page 分组，现有
`LoadRevisionNotes(pageId, signal)` 可以读本地分组缓存，并不要求单页 HTTP
接口。具体路径与请求封装在实现时沿用仓库约定。

解析器只负责来源协议；Chapter 用例拥有权限、Page 映射、事务及替换。
Repository 只实现实际被该用例调用的批量操作，所有 Rust 查询使用 typed
Diesel。在线编辑、逐条解决状态和返修 PDF 都不属于这一期。

### 成稿与文件关联

现有 [PageRawIdentInfo](../src/model/read/proj/page.rs) 可以提供原文件名线索，
但需要先确认原稿名与 PSD 相对路径的映射规则。现有
[artwork 导出](../src/usecase/chapter_port/artwork.rs) 提供版本与内容身份；
PSD 树路径只在对应成稿内有意义。应明确导入对应的成稿，换版后不要默默
把旧 layer_id 解释成新图层。当前不必为了这个问题建立完整历史版本系统。

## 后续验证范围

实施时重点验证：完整多页导入、无问题页清空、空快照、相同快照重复导入、
坏文件不产生部分写入、跨章/未知/歧义文件映射、重复来源身份、无框与越界
矩形、非默认分类、多行批注、编号空号、嵌套与同名图层。

本次仅分析和编写本文，没有修改 Rust、前端源码或生成文件，也没有运行
应用测试。Unit 的旧文档仍描述 204 与无新建 ID 映射；现行
[HTTP handler](../src/api/http/handler/unit.rs)、
[save 用例](../src/usecase/unit.rs) 和 [Val](../src/data/val/unit.rs)
已采用 200、save_id 重放与 created_unit_ids。若后续参考 Unit，须以现行
源码为准；这些增量保存机制不需要带入当前整章导入范围。

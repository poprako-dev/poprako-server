下面这个方案按“黑盒 E2E 集成测试”设计：只通过 HTTP API 操作，不直接查 DB。初始假设只有三类 seed row：`sadmin user`、`default team`、`sadmin 在 default team 中的 member`。Swagger 里明确有登录/注册、成员邀请、作品集、漫画、章节、页、unit 保存、workflow stage、assignment、system mail 等接口；`POST /api/v1/comics` 还声明会同时创建 first chapter；workflow stage 枚举为 `raw-provide / translate / proofread / typeset-redraw / review / publish`，stage mask 是 6 个阶段各占 2 bit；unit 保存模型是 `SavePageUnitsData -> UnitDiffData.opers`，返回 `local_id_mappers / total_unit_count / translated_unit_count / proofread_unit_count`。

注意一个关键点：当前 Swagger 里的 unit 顺序输入不是你之前说的 `CandOrder`，而是 `UnitOperData.save` 的可选 `before_id`。同时 `UnitInfoVal` 本身不暴露 `index`，但 translation export 里的 unit 有 `unit_index`。所以 unit index 的严格断言应该优先走 `GET /chapters/{chapter_id}/translations/export?format=poprako`，而不是只看 `/pages/{page_id}/units`。

## 0. 测试命名与角色约定

每次测试生成唯一前缀，例如 `it_20260707_160500_`。所有 qid、昵称、team/workset/comic/chapter 标题都带此前缀，便于重复运行和清理。

角色 mask 建议先在测试代码里用常量表达。Swagger 只定义了 `RoleMask` 是整数 bitmask，没有暴露 role enum，因此下面的角色名是测试语义名，具体数值按后端真实常量替换：

`R_RAW = 1`，`R_TRANSLATOR = 2`，`R_PROOFREADER = 4`，`R_TYPESETTER = 8`，`R_REVIEWER = 16`，`R_PUBLISHER = 32`，`R_ADMIN = 64`。如果真实项目里 `64` 是 team admin，这里保留；其他位按实际调整。

本轮模拟 15 人汉化组：`sadmin` 作为组长/超级管理员；14 个普通成员分别是 `raw_01, raw_02, trans_01, trans_02, trans_03, proof_01, proof_02, type_01, type_02, redraw_01, review_01, review_02, publish_01, guest_01`。另建一个 `outsider_01` 用于跨 team 权限测试，可通过第二个 team 邀请注册。

## 1. 基线与鉴权测试

### A1. sadmin 登录和默认数据发现

操作：

1. `POST /api/v1/auth/login`，用默认 sadmin qid/password。
2. `GET /api/v1/users/me`。
3. `GET /api/v1/members/me?incl=team&offset=0&limit=20`。
4. `GET /api/v1/teams?offset=0&limit=20`。

断言：

1. 登录返回 `200`，body 有 `user_id` 和 `token`，响应设置 `authorization-token` cookie。
2. `/users/me` 返回 `is_sadmin = true`，`id = login.user_id`。
3. `/members/me` 至少 1 条，其中一条 `user_id = sadmin_id`，`team != null`，记录下 `default_team_id` 和 `sadmin_member_id`。
4. `/teams` 不传 `user_id` 对 sadmin 返回 `200`，列表包含 `default_team_id`。
5. `default team.workset_next_index` 是非负整数。
6. 所有 timestamp 字段是 Unix milliseconds 的整数，并且 `created_at <= updated_at`。

### A2. 未登录访问保护

操作：清 cookie 或新 client。

断言：

1. `GET /api/v1/users/me` 返回 `401`。
2. `GET /api/v1/members/me?offset=0&limit=20` 返回 `401`。
3. `GET /api/v1/teams?offset=0&limit=20` 返回 `401`。
4. `POST /api/v1/worksets` 返回 `401` 或 `403`，但不能成功。
5. `POST /api/v1/auth/logout` 返回 `204`，再次访问 `/users/me` 返回 `401`。

## 2. 成员邀请、注册、成员列表、权限边界

### B1. sadmin 批量邀请 14 个成员

操作：sadmin 对 default team 调用 14 次 `POST /api/v1/member-invitations`。

邀请矩阵：

| 人员       |               qid |         roles |
| ---------- | ----------------: | ------------: |
| raw_01     |     prefix_raw_01 |         R_RAW |
| raw_02     |     prefix_raw_02 |         R_RAW |
| trans_01   |   prefix_trans_01 |  R_TRANSLATOR |
| trans_02   |   prefix_trans_02 |  R_TRANSLATOR |
| trans_03   |   prefix_trans_03 |  R_TRANSLATOR |
| proof_01   |   prefix_proof_01 | R_PROOFREADER |
| proof_02   |   prefix_proof_02 | R_PROOFREADER |
| type_01    |    prefix_type_01 |  R_TYPESETTER |
| type_02    |    prefix_type_02 |  R_TYPESETTER |
| redraw_01  |  prefix_redraw_01 |  R_TYPESETTER |
| review_01  |  prefix_review_01 |    R_REVIEWER |
| review_02  |  prefix_review_02 |    R_REVIEWER |
| publish_01 | prefix_publish_01 |   R_PUBLISHER |
| guest_01   |   prefix_guest_01 |         R_RAW |

断言：

1. 每次返回 `201`，body 有 `id` 和 `code`。
2. `code` 非空，14 个 code 互不相同。
3. `GET /api/v1/teams/{default_team_id}/member-invitations?pending=true&incl=invitor&offset=0&limit=50` 返回至少这 14 条。
4. 每条 `pending = true`，`team_id = default_team_id`，`invitee_qid` 正确，`roles` 正确。
5. `invitor != null`，`invitor.id = sadmin_id`。
6. 用相同 `invitee_qid` 再邀请一次，返回 `409`。

### B2. 修改和删除邀请

操作：

1. 对 `guest_01` 的 invitation 调 `PUT /api/v1/member-invitations/{id}/roles`，把 roles 改成 `R_RAW | R_TRANSLATOR`。
2. 用错误 body id 调同一个接口。
3. 新建一个临时邀请 `prefix_cancelled_01`，然后 `DELETE /api/v1/member-invitations/{id}`。
4. 用被删除邀请的 code 注册。

断言：

1. 修改 roles 返回 `204`。
2. 再 list invitation，`guest_01.roles = R_RAW | R_TRANSLATOR`。
3. path id 与 body id 不一致返回 `422`。
4. 删除返回 `204`。
5. 被删除 code 用于 `/auth/register` 返回 `401`。
6. 删除同一个 invitation 第二次返回 `404`。

### B3. 14 人注册入组

操作：每个 invitee 调 `POST /api/v1/auth/register`，body 为 `{ qid, nickname, password, code }`。

断言：

1. 每个注册返回 `201`，body 有 `user_id` 和 `token`，cookie 设置成功。
2. 立刻用该用户 client 调 `/users/me`，返回 `qid/nickname/id` 正确，`is_sadmin = false`。
3. 调 `/members/me?incl=team&offset=0&limit=20`，返回一条 default team member。
4. member 的 `team_id = default_team_id`，`roles` 等于 invitation roles。
5. 用同一个 code 再注册一次，返回 `401` 或 `422`，不能创建第二个 user。
6. 用 `trans_01` 的 code 配 `trans_02` 的 qid 注册，返回 `422`。
7. `GET /teams/{default_team_id}/member-invitations?pending=true` 不再包含已注册的 14 个 invitation。
8. `GET /teams/{default_team_id}/member-invitations?pending=false` 包含这 14 个 invitation，`pending = false`。

### B4. 成员列表过滤和错误参数

操作：

1. sadmin 调 `GET /api/v1/members?team_id={default_team_id}&incl=user&offset=0&limit=50`。
2. `GET /api/v1/members?team_id={default_team_id}&role={R_TRANSLATOR}&offset=0&limit=50`。
3. `GET /api/v1/members?team_id={default_team_id}&fuzzy_nickname=trans&offset=0&limit=50`。
4. `GET /api/v1/members?owner_id={trans_01_user_id}&incl=team&offset=0&limit=20`。
5. 错误参数：同时传 `team_id` 和 `owner_id`。
6. 错误参数：`owner_id` 模式下传 `role`。
7. 错误参数：`role = R_TRANSLATOR | R_PROOFREADER` 作为复合 role filter。

断言：

1. team mode 返回 15 个成员：sadmin + 14 人。
2. 每个 member 都有 `id/user_id/team_id/nickname/roles/last_active_at`。
3. `incl=user` 时 `user != null`，且 `user.id = member.user_id`。
4. translator role filter 只返回 `trans_01/trans_02/trans_03/guest_01` 中实际含 translator bit 的成员。
5. fuzzy nickname 只返回昵称包含 `trans` 的成员。
6. owner mode 返回 trans_01 的 default team membership。
7. 同时传 `team_id` 和 `owner_id` 返回 `422`。
8. owner mode + role 返回 `422`。
9. 复合 role 作为 filter 返回 `422`，因为 Swagger 对 list filter 声明的是 single role bit。

### B5. 成员角色更新和权限

操作：

1. sadmin 把 `guest_01` 的 member roles 改成 `R_RAW | R_TRANSLATOR | R_PROOFREADER`。
2. `trans_01` 尝试修改 `proof_01` 的 member roles。
3. 用 path id/body id 不一致修改。
4. `guest_01` 尝试删除 `trans_01` member。
5. sadmin 删除一个临时 member，不删除核心 14 人。

断言：

1. sadmin 修改返回 `204`，重新 list 后 roles 生效。
2. 非管理员修改别人 roles 返回 `403`。
3. path/body id 不一致返回 `422`。
4. 非管理员删除别人 member 返回 `403`。
5. 删除临时 member 后，成员列表不再包含该 member。
6. 被删 member 的用户仍能登录，但访问 default team 资源返回 `403`。

## 3. 作品集、漫画、章节 index 与级联行为

这里按真实工作创建 4 个作品集：

1. `连载池`
2. `短篇池`
3. `加急池`
4. `归档池`

### C1. 创建 workset，并检测 index/next_index

操作：

1. sadmin 连续 `POST /api/v1/worksets` 4 次。
2. 每次后 `GET /api/v1/teams/{default_team_id}`。
3. `GET /api/v1/teams/{default_team_id}/worksets?offset=0&limit=20`。

断言：

1. 每次创建返回 `201`，body 有 `id`。
2. 列表包含 4 个新 workset。
3. 新 workset 的 `index` 按创建顺序递增。
4. `team.workset_next_index` 每创建一次 +1。
5. `workset_count` 没有字段，所以 active 数量通过 list 长度断言。
6. 非 default team member 调创建返回 `403`。
7. 未登录创建返回 `401`。

### C2. 删除中间 workset 后再创建

操作：

1. 删除 `短篇池`：`DELETE /api/v1/worksets/{short_ws_id}`。
2. 再创建 `短篇池-重建`。
3. list worksets。

断言：

1. 删除返回 `204`。
2. `GET /api/v1/worksets/{short_ws_id}` 返回 `404` 或 `403`，但不能返回旧数据。
3. list 不包含旧 `short_ws_id`。
4. 新 workset 的 `index` 应等于删除前的 `team.workset_next_index`，不回填旧 index。
5. active workset 数量仍为 4。
6. `team.workset_next_index` 再 +1。
7. 旧 workset 下如果后续曾创建漫画，删除时要级联删除其漫画和章节；本 case 可留到 C9 做完整级联。

### C3. 创建漫画，并验证 first chapter

在 `连载池` 下创建 3 部漫画：

1. `星尘旅人`，first chapter subtitle = `第 1 话 旧站重启`
2. `雨夜便利店`，first chapter subtitle = null
3. `钢铁魔女`，first chapter subtitle = `序章`

操作：3 次 `POST /api/v1/comics`。

断言：

1. 每次返回 `201`，body 同时有 `id` 和 `chapter_id`。
2. `GET /api/v1/comics/{comic_id}` 返回：
   - `id` 正确；
   - `workset_id` 正确；
   - `index` 按创建顺序递增；
   - `chapter_count = 1`；
   - `chapter_next_index = 1`；
   - `is_completed = false`；
   - `cover_url = null`。

3. `GET /api/v1/chapters/{chapter_id}` 返回：
   - `comic_id` 正确；
   - `index = 0`；
   - `page_count = 0`；
   - `total_unit_count = 0`；
   - `translated_unit_count = 0`；
   - `proofread_unit_count = 0`；
   - `creator_id = sadmin_id`；
   - subtitle 为指定值；若创建 comic 时未给 first subtitle，则只断言非空，不强依赖具体默认文案。

4. `GET /api/v1/comics/{comic_id}/chapters?offset=0&limit=20` 只有 first chapter。
5. `GET /api/v1/worksets/{ws_id}/comics?with=pinned_chapter&incl=workset.team&offset=0&limit=20` 返回这 3 部漫画，且 `workset != null`，`workset.team != null`，`pinned_chapter` 初始为 null 或符合当前业务默认。
6. 非该 team 成员创建漫画返回 `403`。
7. 用不存在 workset_id 创建漫画返回 `404`。

### C4. 删除中间漫画后再创建

操作：

1. 删除 `雨夜便利店`。
2. 创建 `雨夜便利店-重制版`。
3. list comics。

断言：

1. 删除返回 `204`。
2. `GET /api/v1/comics/{deleted_id}` 返回 `404`。
3. list 不包含 deleted comic。
4. 新漫画的 `index` 不回填旧 index，而是等于删除前 `workset.comic_next_index`。
5. `workset.comic_count` 等于 active comic 数量。
6. `workset.comic_next_index` 单调递增。
7. 删除 comic 后，其 first chapter `GET /chapters/{chapter_id}` 返回 `404`。
8. 按 `fuzzy_title=雨夜` 能搜到重制版，不能搜到已删除旧版。
9. `is_completed=false` filter 包含 active 未完结漫画。
10. `POST /api/v1/comics/{comic_id}/mark-completed` 设 true 后，`is_completed=true` filter 包含它，false filter 不包含它；再设 false 后恢复。

### C5. 创建多章节，并验证 chapter index/next_index

对 `星尘旅人` 创建章节：

1. first chapter 已存在：index 0。
2. 新建 `第 2 话 月面信号`。
3. 新建 `第 3 话 失控列车`。
4. 新建 `第 4 话 地下港口`。
5. 删除第 3 话。
6. 新建 `第 5 话 断层回声`。

断言：

1. 每次 `POST /api/v1/chapters` 返回 `201`。
2. 创建后 `GET /api/v1/comics/{comic_id}` 的 `chapter_count` 与 active chapter 数一致。
3. `chapter_next_index` 每创建一次 +1，删除不回退。
4. list chapters 中 active chapter 的 index 为 `[0,1,3,4]`，如果采用“不可回填”策略；如果产品明确要求重排，则改成 `[0,1,2,3]`，但必须二选一固定下来。
5. 被删 chapter `GET /api/v1/chapters/{id}` 返回 `404`。
6. 新建 chapter 不复用已删除 chapter 的 id。
7. `incl=comic.workset.team&incl=creator` 时，chapter 里 `comic/workset/team/creator` 均非空且 id 链正确。
8. 非该 team 成员创建 chapter 返回 `403`。
9. 用不存在 comic_id 创建返回 `404`。

建议这里采用“index 不回填”。原因是 Swagger 里 comic/workset 都有 `*_next_index` 字段，这种字段通常表示单调分配器。回填会让历史引用和协作界面更难稳定。

### C6. 章节 pin/unpin

操作：

1. `PATCH /api/v1/chapters/{ch2_id}` body `{ id: ch2_id, pin: true }`。
2. `GET /api/v1/comics/{comic_id}/chapters/pinned`。
3. `GET /api/v1/comics/{comic_id}?with=pinned_chapter`，如果 get comic 支持 `with` 参数则测；如果该 endpoint 没参数，只测 workset comics list。
4. 再 pin `ch4_id`。
5. 再对 `ch4_id` patch `{ pin: false }`。

断言：

1. patch 返回 `204`。
2. pinned endpoint 返回 `ch2_id`。
3. pin `ch4_id` 后，pinned endpoint 返回 `ch4_id`，`ch2.is_pinned = false`。
4. unpin 后，pinned endpoint 返回 null。
5. path id/body id 不一致返回 `422`。
6. 非授权用户 patch 返回 `403`。
7. patch 不传 `subtitle` 时原 subtitle 不变；只传 `subtitle` 时 pin 状态不变。

### C7. info update 全覆盖

操作与断言：

1. `PUT /api/v1/teams/{team_id}` 修改 name/description，返回 `204`；GET 后字段变化，`updated_at` 增大。
2. team path/body id 不一致返回 `422`。
3. `PUT /api/v1/worksets/{id}` 修改 name/description，返回 `204`；GET 后字段变化，`index/comic_count/comic_next_index` 不变。
4. `PUT /api/v1/comics/{id}` 修改 title/author/description，返回 `204`；GET 后字段变化，`index/chapter_count/chapter_next_index` 不变。
5. `PATCH /api/v1/chapters/{id}` 修改 subtitle，返回 `204`；GET 后字段变化，`index/page_count/unit counts/stages` 不变。
6. 普通 translator 修改 team/workset/comic/chapter profile 返回 `403`。
7. 用不存在 id update 返回 `404`。

### C8. 封面、头像 reserve/mark-uploaded

操作：

1. `POST /api/v1/teams/{team_id}/avatar/reserve` body `{ file_ext: "png" }`。
2. `POST /api/v1/teams/{team_id}/avatar/mark-uploaded` body `{ avatar_version }`。
3. 对 sadmin 或 trans_01 执行 `/users/{user_id}/avatar/reserve` 和 `mark-uploaded`。
4. 对 comic 执行 `/comics/{comic_id}/cover/reserve` 和 `mark-uploaded`。

断言：

1. reserve 返回 `200`，`put_url` 非空，version 是 int64。
2. mark 正确 version 返回 `204`。
3. GET team/user/comic 后对应 `avatar_url` 或 `cover_url` 从 null 变成非空。
4. 非 owner 修改 user avatar 返回 `403`。
5. 非授权成员修改 team avatar/comic cover 返回 `403`。
6. 不存在 id 返回 `404`，若 Swagger 未列 404 的 user avatar mark，则至少断言不能 `204`。

### C9. 级联删除：chapter/comic/workset/team

单独创建一个 `归档池-级联测试` workset，内含 2 个 comic，每个 comic 2 个 chapter，每个 chapter reserve 2 页，每页写 2 个 unit，并给其中一个 chapter 建 assignment 和 assignment invitation。

断言链：

1. 删除一个 chapter 后：
   - `GET /chapters/{chapter_id}` 返回 `404`；
   - comic 的 `chapter_count` -1；
   - comic 的 `chapter_next_index` 不回退；
   - owner assignment list 不再包含该 chapter 的 assignment；
   - chapter invitation list 无法再作为有效数据源；若接口返回 `403/404` 都可接受，但不能返回旧 invitation 为 pending。

2. 删除 comic 后：
   - `GET /comics/{comic_id}` 返回 `404`；
   - workset 的 `comic_count` -1；
   - comic 下所有 chapters `GET` 均 `404`；
   - 相关 assignments 从用户 owner list 消失。

3. 删除 workset 后：
   - `GET /worksets/{workset_id}` 返回 `404`；
   - team workset list 不包含它；
   - workset 下所有 comics `GET` 返回 `404`。

4. 不删除 default team。若测试创建了临时 team，删除临时 team 后：
   - `GET /teams/{team_id}` 返回 `404`；
   - 该 team 下 workset/comic/chapter 不能再被访问；
   - 该 team 的成员无法再通过该 membership 访问资源。

## 4. 页 reserve、图片状态、page index 与计数

选择 `星尘旅人 / 第 2 话 月面信号` 作为主生产章节 `main_chapter`。

### D1. 批量 reserve 页

操作：`POST /api/v1/chapters/{main_chapter_id}/pages/reserve` body `{ chapter_id, page_count: 8, file_ext: "jpg" }`。

断言：

1. 返回 `200`。
2. `creations.length = 8`。
3. 每个 creation 有 `page_id/put_url/image_version`。
4. 8 个 `page_id` 互不相同。
5. `put_url` 非空。
6. `image_version` 是 int64。
7. `GET /api/v1/chapters/{chapter_id}/pages?offset=0&limit=20` 返回 8 页。
8. pages 的 `index` 为 `0..7`。
9. 每页 `total_unit_count = 0`，`translated_unit_count = 0`，`proofread_unit_count = 0`。
10. `GET /api/v1/chapters/{chapter_id}` 返回 `page_count = 8`，三个 unit count 均为 0。
11. 对同一 chapter 再 reserve pages 返回 `422`。
12. `page_count = 0` 或负数返回 `422`。
13. path chapter_id 与 body chapter_id 不一致返回 `422`；如果当前实现未列该错误，也应至少不能成功。

### D2. mark-uploaded 与单页替换

操作：

1. 对 8 页分别 `POST /api/v1/pages/{page_id}/image/mark-uploaded`，使用 reserve 返回的 image_version。
2. 选择第 3 页调用 `POST /api/v1/pages/{page_id}/image/reserve`，file_ext = `png`。
3. 再 mark 第 3 页新 version。

断言：

1. 每次 mark 返回 `204`。
2. list pages 后，已 mark 的 page `image_url` 非空。
3. 单页 reserve 返回 `200`，body 的 `page_id` 等于 path page_id。
4. 新 `image_version` 大于旧 version。
5. mark 新 version 后 `image_url` 仍非空，`updated_at` 增大。
6. 非 assignment/非授权成员 mark 或 reserve page image 返回 `403`。
7. 不存在 page_id 返回 `404`。

### D3. 删除全部 pages 后重建

在另一个辅助 chapter 上操作，避免破坏主章节。

操作：

1. reserve 3 页。
2. 每页写若干 unit。
3. `DELETE /api/v1/chapters/{chapter_id}/pages`。
4. list pages。
5. GET chapter。
6. 再 reserve 2 页。

断言：

1. 删除返回 `204`。
2. list pages 返回空数组。
3. chapter 的 `page_count = 0`。
4. chapter 的 `total_unit_count/translated_unit_count/proofread_unit_count` 全部归零。
5. 再 reserve 成功。
6. 新 pages 的 index 为 `0,1`。
7. 旧 page_id 的 `/pages/{old_page_id}/units` 返回 `404` 或不能成功返回旧 units。
8. 旧 page image reserve/mark 返回 `404`。

## 5. Assignment 与 assignment invitation

### E1. 直接加入 chapter assignment

对 `main_chapter`：

操作：

1. `trans_01` 调 `POST /api/v1/assignments/join`，roles = `R_TRANSLATOR`。
2. `trans_02` 同样加入 translator。
3. `proof_01` 加入 proofreader。
4. `type_01` 加入 typesetter。
5. `review_01` 加入 reviewer。
6. `publish_01` 加入 publisher。
7. `guest_01` 尝试用不允许的 role 或跨阶段 role 加入，按业务规则测。

断言：

1. 合法加入返回 `201`，body 有 `id/chapter_id/user_id/roles`。
2. `chapter_id = main_chapter_id`，`user_id = 当前用户 id`。
3. 重复加入同一个 chapter，如果返回已有 assignment 或 409 未在 Swagger 中定义；建议期望不能创建重复 assignment。可断言 list 中 `(chapter_id,user_id)` 唯一。
4. 不可 assign 的 role 返回 `403`。
5. 不存在 chapter 返回 `404`。
6. `GET /api/v1/assignments?chapter_id={main_chapter_id}&incl=user&offset=0&limit=50` 返回所有加入者。
7. `GET /api/v1/assignments?owner_id={trans_01_id}&incl=chapter.comic.workset.team&offset=0&limit=50` 返回 trans_01 的 assignment，且嵌套对象 id 链正确。
8. `role={R_TRANSLATOR}` filter 只返回 translator assignment。
9. 复合 role filter 返回 `422`。
10. 同时传 `chapter_id` 和 `owner_id` 返回 `422`。
11. 两者都不传返回 `422`。

### E2. 管理员创建 assignment invitation

操作：

1. sadmin 给 `trans_03` 创建 main chapter assignment invitation，roles = `R_TRANSLATOR`。
2. `trans_03` 用 code 调 `/api/v1/assignment-invitations/join`。
3. sadmin 给 `proof_02` 创建 proofread invitation。
4. 用 `trans_01` 尝试消费 `proof_02` 的 code。
5. sadmin 给已 assigned 的 `trans_01` 再创建 invitation。

断言：

1. 创建返回 `201`，body 有 `id/code`。
2. `GET /api/v1/chapters/{chapter_id}/assignment-invitations?pending=true&offset=0&limit=20` 包含新 invitation。
3. 正确 invitee join 返回 `201`，assignment roles 正确。
4. join 后 pending list 不再包含该 invitation，pending=false list 包含它。
5. 错误用户消费 code 返回 `422`。
6. 不存在 code 返回 `404`。
7. 已 assigned 用户再被邀请返回 `409`。
8. 无权限用户创建 invitation 返回 `403`。
9. 删除 pending invitation 后，code 不能再 join。
10. 删除已 consumed invitation 如果允许，返回 `204`；否则按实现固定为 `403/404`，但后续 assignment 不应被删除。

### E3. 更新和删除 assignment

操作：

1. sadmin 把 `trans_03` 的 assignment roles 从 `R_TRANSLATOR` 改为 `R_TRANSLATOR | R_PROOFREADER`。
2. path/body chapter_id 不一致。
3. path/body user_id 不一致。
4. `trans_01` 尝试修改 `proof_01` assignment。
5. sadmin 删除 `trans_03` assignment。

断言：

1. 合法更新返回 `204`，list 后 roles 更新。
2. chapter_id mismatch 返回 `422`。
3. user_id mismatch 返回 `422`。
4. 非管理员修改别人 assignment 返回 `403`。
5. 删除返回 `204`。
6. 删除后 chapter assignment list 不包含 trans_03。
7. trans_03 owner assignment list 不包含该 chapter。
8. 删除同一 assignment 第二次返回 `404`。
9. 被删除 assignment 的用户再保存 unit，如权限依赖 assignment，应返回 `403`。

## 6. Unit save：顺序、计数、导出、复杂并发

主章节 8 页，选 `p0, p1, p2` 做高强度测试。

### F1. p0 初始创建 5 个气泡 unit

由 `raw_01` 或有权限用户执行：

`POST /api/v1/pages/{p0_id}/units/save`，body：

每个 save oper 都含：

- `oper = "save"`
- `local_id = "p0_lu_01" ... "p0_lu_05"`
- `id = null`
- `before_id = null`
- `is_bubble = true`
- `is_proofread = false`
- `x_coord/y_coord` 分别为不同坐标
- `translated_text = null`
- `proofread_text = null`

断言：

1. 返回 `200`。
2. `local_id_mappers.length = 5`。
3. 每个 mapper 的 `local_id` 等于请求 local_id。
4. 每个 `unit_id` 非空且互不相同，且不等于 local_id。
5. 返回 `total_unit_count = 5`。
6. `translated_unit_count = 0`。
7. `proofread_unit_count = 0`。
8. `GET /pages/{p0_id}/units?offset=0&limit=20` 返回 5 个 unit。
9. 每个 unit 的 `page_id = p0_id`。
10. `translated_text/proofread_text/last_translator_id/last_proofreader_id` 为 null。
11. `GET /chapters/{main_chapter_id}` 的 `total_unit_count` 增加 5。
12. `GET /chapters/{main_chapter_id}/pages` 中 p0 的 `total_unit_count = 5`。
13. export `format=poprako` 中 p0 的 units 有 `unit_index = 0..4`。

### F2. 使用 before_id 插入和重排

操作：

1. 在第 2 个 unit 前插入 `p0_lu_insert_before_02`，`before_id = unit_02_id`。
2. 再把第 5 个已有 unit 用 `save` + `id = unit_05_id` + `before_id = unit_01_id` 尝试移动到最前，同时修改坐标。

断言：

1. 插入返回 `200`，mapper 只包含新 local_id。
2. 总数从 5 到 6。
3. export order 变成 `[unit_01, inserted, unit_02, unit_03, unit_04, unit_05]`，或如果 before 语义是“插到 before_id 之前”，必须严格如此。
4. 已有 id 的 save 如果支持移动，返回 `200` 后 export order 为 `[unit_05, unit_01, inserted, unit_02, unit_03, unit_04]`。
5. 如果当前实现不支持 move，则应返回 `422`，且顺序完全保持上一步；这个行为必须固定，不能“200 但没动”。
6. 移动或更新已有 unit 时，`local_id_mappers` 不应包含已有 id。
7. 任何情况下 unit id 集合无重复。
8. `unit_index` 连续，无洞、无重复。

### F3. 翻译保存和计数

操作：

1. `trans_01` 更新 unit 1、2、3，写入 `translated_text`。
2. `trans_02` 更新 unit 4、5、6，写入 `translated_text`。
3. 保持 `is_proofread = false`。
4. list units 和 GET chapter/pages。

断言：

1. 两次 save 都返回 `200`。
2. 每个更新后的 unit `translated_text` 等于提交文本。
3. `translated_unit_count = 6`。
4. `proofread_unit_count = 0`。
5. page p0 count 与 chapter aggregate count 一致。
6. 如果服务端应使用当前登录用户作为 `last_translator_id`，则断言 unit 1-3 为 `trans_01_id`，unit 4-6 为 `trans_02_id`；如果 DTO 设计为客户端传入快照，也必须断言它等于请求值。这个点不能不测。
7. 未分配 translator 角色的 `guest_01` 写 translated_text 返回 `403`。
8. 提交 page_id 与 path page_id 不一致返回 `422` 或至少不能成功。
9. `before_id` 指向不存在 unit 返回 `422`。
10. 删除不存在 unit id 返回 `422` 或返回 `200` 且计数不变；建议固定为 `422`。

### F4. 校对保存和 proofread 计数

操作：

1. `proof_01` 对 unit 1、2、3 写 `proofread_text`，`is_proofread = true`。
2. `proof_02` 对 unit 4、5 写 `proofread_text`，`is_proofread = true`。
3. 对 unit 6 写 `proofread_text` 但 `is_proofread = false`，用于区分计数依据。

断言：

1. 保存返回 `200`。
2. unit 1-5 的 `is_proofread = true`。
3. unit 6 的 `proofread_text` 非空但 `is_proofread = false`。
4. `proofread_unit_count = 5`，证明计数按 `is_proofread`，不是按 `proofread_text != null`。
5. chapter aggregate `proofread_unit_count` 同步为 5。
6. `last_proofreader_id` 按当前用户或请求值固定断言。
7. 没有 proofreader assignment 的用户保存 proofread 字段返回 `403`。
8. 把 unit 2 改回 `is_proofread = false` 后，`proofread_unit_count` 从 5 降到 4。
9. 再改回 true，计数回到 5。

### F5. 删除 unit 与计数回退

操作：

1. 删除一个未翻译未校对 unit。
2. 删除一个已翻译未校对 unit。
3. 删除一个已翻译已校对 unit。
4. list/export。

断言：

1. 每次删除返回 `200`。
2. total count 每次 -1。
3. 删除已翻译 unit 时 translated count -1。
4. 删除已校对 unit 时 proofread count -1。
5. export 中 unit_index 重新连续。
6. 被删 unit 不再出现在 list/export。
7. 重复删除同一个 id 返回 `422` 或 `200` 且计数不变；建议固定为 `422`。
8. chapter aggregate 与 page count 完全一致。

### F6. p1 并发：多翻译同时改不同 unit

准备：p1 建 12 个 unit，全部未翻译。

并发操作：

1. `trans_01` 更新 unit 1-4。
2. `trans_02` 更新 unit 5-8。
3. `trans_03` 更新 unit 9-12。
4. 三个请求同时发出。

断言：

1. 三个请求都返回 `200`。
2. 最终 p1 `total_unit_count = 12`。
3. `translated_unit_count = 12`。
4. 每个 unit 的 translated_text 与对应 translator 请求一致。
5. 没有 unit 丢失。
6. export unit_index 仍为 0..11。
7. chapter aggregate 增加 12 translated units。
8. 每个请求返回的 count 可以是中间态或最终态，但最后一次 list 的 count 必须正确；如果要求每个响应都返回事务提交后的最新全页 count，则三个响应最终应分别呈现单调不减。

### F7. p1 并发：同一个 unit 冲突写

准备：选 p1 unit 1，当前 text = `old`。

并发操作：

1. `trans_01` 写 `A version`。
2. `trans_02` 写 `B version`。
3. 两个请求同时发送。

断言采用“允许结果集合”，因为 Swagger 没有 version 字段：

1. 如果实现是 last-write-wins：两个请求可都 `200`，最终 text 必须严格等于 `A version` 或 `B version` 之一，不能混合，不能空。
2. 如果实现有内部冲突检测：一个 `200`，另一个 `422`，最终 text 等于成功请求。
3. 不允许两个都 `200` 但最终 text 不是任一请求值。
4. 不允许 count 错误。
5. 不允许 unit id 变化。
6. `updated_at` 必须大于旧值。
7. export 顺序不变。

### F8. p1 并发：同 before_id 插入

准备：选定 anchor = p1 unit 4。

并发操作：

1. `raw_01` 插入 `A_before_anchor`，`before_id = anchor_id`。
2. `raw_02` 插入 `B_before_anchor`，`before_id = anchor_id`。
3. 同时发送。

断言：

1. 两个请求都应 `200`；如果有锁竞争失败，也必须是明确的 `422/409`，不能 500。
2. 最终新增成功数等于成功响应数。
3. 新 unit id 全部唯一。
4. 所有成功插入的 unit 都排在 anchor 之前。
5. anchor 之后的原有相对顺序不变。
6. unit_index 连续。
7. page/chapter total count 正确。

### F9. p1 并发：删除与更新同一 unit

准备：选 unit X，当前有 translated_text。

并发操作：

1. `trans_01` 更新 X 的 translated_text。
2. `raw_01` 删除 X。

断言允许两类合法结果：

1. 删除先提交：update 应返回 `422` 或不能成功；最终 X 不存在，count -1。
2. 更新先提交：delete 返回 `200`；最终 X 不存在，count -1。
3. 如果两个都 `200`，最终 X 不存在也可接受，但 count 只能 -1。
4. 不允许最终 X 存在但 delete 返回 `200`。
5. 不允许 total count -2。
6. 不允许 translated count 与剩余 translated_text 数量不一致。

### F10. 导入/导出回归

操作：

1. 对 main chapter `GET /translations/export?format=poprako`。
2. 对 main chapter `GET /translations/export?format=label-plus`。
3. 把 poprako export 内容重新 import 到一个空白辅助 chapter。
4. 导出辅助 chapter。

断言：

1. poprako export 返回 `200`，content-type 是 JSON 或可解析 JSON。
2. label-plus export 返回 `200`，content-type 是 text/plain 或可作为文本处理。
3. JSON export 的 `chapter_id/comic_id/comic_title/pages/page_index/unit_index` 完整。
4. 每页 page_index 与 list pages index 一致。
5. 每个 unit_index 连续。
6. import 返回 `200`，`imported_page_count` 和 `imported_unit_count` 等于源数据。
7. 辅助 chapter 再 export 后，页数、unit 数、坐标、翻译文本、校对文本一致。
8. invalid format content 返回 `422`。
9. 无权限用户 export/import 返回 `403`。

## 7. Workflow 推进、退回与 system mail

Stage 测试必须覆盖 6 个阶段：`raw-provide -> translate -> proofread -> typeset-redraw -> review -> publish`。Swagger 暴露的是 `POST /chapters/{id}/stage/advance`，body 里 `oper` 可以是 `advance` 或 `revert`，所以虽然 path 叫 advance，仍要用同一 endpoint 测 revert。

### G1. 初始 workflow 状态

操作：GET main chapter。

断言：

1. `stages` 是非负整数。
2. 新 chapter 的 6 个阶段都处于初始态。若 phase 编码为 0=pending，则 `stages = 0`；如果不是，至少记录 baseline。
3. 不允许新 chapter 直接处于 publish/review completed。
4. chapter 有 assignments：translator/proofreader/typesetter/reviewer/publisher 各至少 1 个。

### G2. 非法跳阶段

操作：

1. 初始态直接 advance `publish`。
2. 初始态直接 advance `proofread`。
3. 初始态 revert `raw-provide`。
4. 用不存在 stage 字符串。
5. path id/body id 不一致。

断言：

1. publish/proofread 跳阶段返回 `422`。
2. pending stage revert 返回 `422`。
3. 非 enum stage 返回 `422`。
4. id mismatch 返回 `422`。
5. 非授权用户操作返回 `403`。
6. 所有失败操作后 `stages` 不变。
7. 不产生 system mail。

### G3. raw-provide 阶段推进

操作：

1. sadmin 或 raw_01 advance `raw-provide` 第一次。
2. GET chapter。
3. advance `raw-provide` 第二次。
4. GET chapter。
5. 查询相关用户 system mails。

断言：

1. 第一次返回 `204`，`stages` 发生变化。
2. 第二次返回 `204`，`stages` 再次变化。
3. 第三次 advance `raw-provide` 返回 `422`，因为该阶段已 completed。
4. raw completed 后，advance `translate` 应允许。
5. raw completed 后，`trans_01/trans_02/trans_03` 的 unread system mail 增加，内容应包含 comic title、chapter subtitle、`translate` 或中文等价阶段名。
6. `proof_01/type_01/review_01/publish_01` 不应收到“开始翻译”的通知。
7. 操作者自己是否收到通知必须固定：建议不通知触发者，或只通知下阶段 assignee。测试应按产品规则断言。
8. system mail 的 `read = false`，`created_at` 在操作时间窗口内。
9. `GET /system-mails?read=false` 能查到，`read=true` 查不到。

### G4. translate 阶段：开始、完成、退回

操作：

1. advance `translate` 第一次，表示开始翻译。
2. 在 p0/p1 进行翻译保存。
3. advance `translate` 第二次，表示翻译完成。
4. advance `proofread` 第一次。
5. revert `proofread`，模拟校对发现问题退回。
6. revert `translate`，模拟翻译阶段从 completed 退回 ongoing 或 pending。

断言：

1. 每次合法操作返回 `204`。
2. 每次成功后 `stages` 都变化。
3. translate 未完成前 advance proofread 返回 `422`。
4. translate completed 后 proofread advance 允许。
5. proofread revert 后，proofread phase 下降一级。
6. translate revert 后，translate phase 下降一级，同时后续 proofread/typeset/review/publish 不得保持“已完成”的非法状态；如果实现要求级联回退，断言后续阶段被清理。
7. translate completed 时，proofreader assignees 收到 unread system mail。
8. revert translate 时，translator assignees 收到返工 mail。
9. 不属于该 chapter assignment 的同角色用户不应收到此 chapter 的 mail。
10. 失败的非法 transition 不产生 mail。

### G5. 完整流转到 publish

操作顺序：

1. raw-provide advance x2。
2. translate advance x2。
3. proofread advance x2。
4. typeset-redraw advance x2。
5. review advance x2。
6. publish advance x2。

中间每完成一个阶段都查一次 chapter 和 mails。

断言：

1. 每个阶段从 pending 到 ongoing 到 completed 需要两次 advance；如果真实实现只需要一次，测试应以 GET 后 phase 变化为准，但必须固定状态机。
2. 阶段不能越过前置阶段。
3. 前置阶段 revert 后，后置阶段不能继续 advance。
4. 最终 publish completed 后，再 advance publish 返回 `422`。
5. 每个阶段完成后，下一阶段 assignee 收到 mail：
   - raw completed -> translators；
   - translate completed -> proofreaders；
   - proofread completed -> typesetters/redraw；
   - typeset completed -> reviewers；
   - review completed -> publishers/admins；
   - publish completed -> admins 或所有参与者，按产品规则固定。

6. 每封 mail 只属于接收者本人；用户 A 不能通过 list 看到用户 B 的 mail。
7. `POST /system-mails/mark-read` 标记当前用户自己的 mails 返回 `204`。
8. 标记别人 mail id 返回 `403`。
9. 标记后 `read=false` list 不再包含，`read=true` list 包含。
10. 同一 mail 重复 mark-read 返回 `204` 且幂等，或固定为可接受行为，但不能 500。

## 8. 公告、评论、用户资料

### H1. announcement

操作：

1. sadmin 创建公告：`POST /api/v1/announcements`。
2. `GET /api/v1/teams/{team_id}/announcements?incl=user&offset=0&limit=20`。
3. translator 尝试创建公告。
4. outsider 尝试 list default team announcements。

断言：

1. 创建返回 `201`，body 有 id。
2. list 包含该公告。
3. title/content/team_id/user_id 正确。
4. `incl=user` 时 user 非空，id 为 sadmin。
5. 普通无权限创建返回 `403`。
6. outsider list 返回 `403`。
7. 不存在 team 创建返回 `404`。
8. pagination：limit=1 只返回 1 条，offset=1 不返回第一条。

### H2. comment

操作：

1. 组内 5 人分别创建 comment，模拟任务留言。
2. list comments with incl=user。
3. outsider 创建/list。

断言：

1. 合法成员创建返回 `201`。
2. list 返回 5 条，按时间或默认顺序固定；至少每条 id 唯一。
3. content/user_id/team_id 正确。
4. incl=user 正确。
5. outsider 创建返回 `403`。
6. 不存在 team 返回 `404`。
7. pagination 正确。
8. comment 不应影响 team updated_at，除非产品明确要求；这个行为要固定。

### H3. 用户资料更新

操作：

1. trans_01 修改自己的 nickname/qid。
2. trans_02 尝试把 qid 改成 trans_01 的 qid。
3. trans_02 尝试修改 trans_01 的资料。
4. path/body id mismatch。
5. sadmin 尝试删除一个临时 user。

断言：

1. 自己修改返回 `204`，GET 后字段变化。
2. qid 冲突返回 `409`。
3. 修改别人返回 `403`，除非 sadmin；普通用户必须被拒。
4. mismatch 返回 `422`。
5. 删除用户返回 `204`。
6. 被删用户 login 返回 `401`。
7. 被删用户相关 member 从 members list 消失，或至少不能再访问资源；具体级联策略固定。

## 9. 跨 team 隔离

### I1. 创建第二 team 和 outsider

操作：

1. sadmin `POST /api/v1/teams` 创建 `外包协作组`。
2. sadmin 邀请 `outsider_01` 入第二 team。
3. outsider 注册/登录。
4. outsider 尝试访问 default team 的 worksets/comics/chapters/pages/units/announcements/member invitations。

断言：

1. sadmin 创建 team 返回 `201`。
2. outsider 是第二 team member。
3. outsider `GET /teams?user_id={outsider_id}` 只看到第二 team，不看到 default team，除非公开策略允许；如果允许看到 team profile，也不能看到私有 descendants。
4. outsider list default worksets 返回 `403`。
5. outsider get default workset 返回 `403`。
6. outsider get default comic/chapter/pages/units 返回 `403`。
7. outsider create default comment/announcement/member invitation/assignment invitation 返回 `403`。
8. outsider 不收到 default team workflow mails。
9. default team 成员不自动拥有第二 team 权限。
10. sadmin 作为 super admin 可以 list all teams。

## 10. 全局一致性断言，建议每个 mutation 后都跑

这些不是泛泛检查，而是可直接实现的断言函数。

### J1. Team invariant

输入 `team_id`。

断言：

1. `GET /teams/{team_id}` 成功。
2. `workset_next_index >= max(active_workset.index) + 1`，无 workset 时 `>= 0`。
3. active workset ids 唯一。
4. active workset index 唯一。
5. workset list pagination 全量拼接后数量等于不分页 list 的数量，如果实现支持大 limit。
6. 非成员访问 descendants 返回 `403`。

### J2. Workset invariant

输入 `workset_id`。

断言：

1. `comic_count = active comics.length`。
2. `comic_next_index >= max(active_comic.index) + 1`。
3. active comic id 唯一。
4. active comic index 唯一。
5. `fuzzy_title` 结果是全集子集。
6. `is_completed` filter 结果全部满足布尔条件。
7. `incl=workset.team` 的 id 链正确。
8. `with=pinned_chapter` 只返回该 comic 当前 pinned chapter 或 null。

### J3. Comic invariant

输入 `comic_id`。

断言：

1. `chapter_count = active chapters.length`。
2. `chapter_next_index >= max(active_chapter.index) + 1`。
3. active chapter id 唯一。
4. active chapter index 唯一。
5. 最多一个 chapter `is_pinned = true`。
6. pinned endpoint 返回的 id 等于 list 中 `is_pinned=true` 的 chapter id，或 null。
7. 每个 chapter 的 `comic_id = comic_id`。
8. chapter `incl=comic.workset.team` 链正确。

### J4. Chapter invariant

输入 `chapter_id`。

断言：

1. `page_count = active pages.length`。
2. `total_unit_count = sum(page.total_unit_count)`。
3. `translated_unit_count = sum(page.translated_unit_count)`。
4. `proofread_unit_count = sum(page.proofread_unit_count)`。
5. 每个 page 的 `chapter_id = chapter_id`。
6. page id 唯一。
7. page index 唯一且连续，从 0 开始。
8. `stages` 是非负整数，并且只在合法 transition 后变化。
9. assignments 中 `(chapter_id,user_id)` 唯一。
10. assignment invitation pending/consumed 状态与实际 join 行为一致。

### J5. Page/unit invariant

输入 `page_id`。

断言：

1. list units 返回的 unit id 唯一。
2. 每个 unit 的 `page_id = page_id`。
3. page `total_unit_count = unit_infos.length`。
4. page `translated_unit_count = count(unit.translated_text != null && unit.translated_text != "")`，如果产品定义空字符串也算翻译，则把判断固定为 `!= null`。
5. page `proofread_unit_count = count(unit.is_proofread == true)`。
6. export 的 unit ids 与 list units 完全一致。
7. export 的 `unit_index` 连续，从 0 到 n-1。
8. export 的 unit order 与 list units order 一致；如果 list API 不保证顺序，则只断言 export。
9. 删除/插入/更新后，不允许出现重复 index、跳 index、孤儿 unit。
10. 并发后重复跑此 invariant。

### J6. Mail invariant

输入用户 client。

断言：

1. unread list 中所有 `read=false`。
2. read list 中所有 `read=true`。
3. mail id 唯一。
4. `created_at` 是 int64 且按列表排序规则稳定。
5. `mark-read` 后 read 状态改变。
6. 当前用户无法 mark 其他用户 mail。
7. 业务事件产生的 mail title/content 必须含足够定位信息：team/comic/chapter/stage/触发动作中的至少 comic+chapter+stage。
8. 不相关用户收不到 mail。

## 11. 推荐测试文件拆分

建议按依赖顺序拆成这些集成测试文件，避免一个巨型 case 失败后全局难定位：

1. `it_00_bootstrap_auth_default_seed`
2. `it_01_member_invitation_register_roles`
3. `it_02_workset_comic_chapter_index`
4. `it_03_page_reserve_image`
5. `it_04_assignment_invitation`
6. `it_05_unit_save_order_count`
7. `it_06_unit_concurrency`
8. `it_07_workflow_sysmail`
9. `it_08_info_update_upload_mark`
10. `it_09_cross_team_permission`
11. `it_10_cascade_delete_cleanup`

每个文件都使用同一个 `RunCtx`：保存 `sadmin_client`、所有用户 client、`default_team_id`、member ids、workset ids、comic ids、chapter ids、page ids、unit ids、assignment ids、invitation ids。每一步创建后都立刻把 id 写入 ctx，后续断言只用 API 返回的 id，不硬编码数据库 id。

## 12. 最小通过标准

这套方案跑完后，至少应能精确回答这些问题：

1. 默认 sadmin/default team/default member 是否可通过 API 自发现。
2. 成员 invitation/register/join/roles/pending 状态是否闭环。
3. member/assignment 的 role filter 是否严格拒绝 composite role。
4. workset/comic/chapter 的 index 是否单调、删除后是否回填。
5. create comic 是否必然原子创建 first chapter。
6. chapter/page/unit 三层计数是否始终一致。
7. unit save 的 `local_id -> server unit_id` 映射是否正确。
8. `before_id` 是否能稳定控制 unit 顺序。
9. unit 并发写是否不丢数据、不重复 id、不破坏计数。
10. workflow 是否拒绝非法跳转。
11. workflow advance/revert 是否产生正确 system mail。
12. system mail 是否只给正确的人、只允许本人读取和 mark-read。
13. assignment invitation 是否防错人消费、防重复 assigned。
14. info update 是否检查 path/body id 一致性。
15. reserve/mark-uploaded 是否正确更新 avatar/cover/image url。
16. 跨 team 是否严格隔离。
17. chapter/comic/workset/team 删除是否正确级联。
18. 所有失败请求是否保持数据不变，不产生脏 mail、脏 assignment、脏 unit count。

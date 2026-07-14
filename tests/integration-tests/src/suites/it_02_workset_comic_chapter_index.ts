// it_02 — Workset, comic, chapter index allocation and pin/unpin.
//
// Preconditions:
//   - it_00 + it_01 have run. `ctx.sadmin` is authenticated; `ctx.users`
//     has the 14 personas.
//
// Postconditions:
//   - `ctx.ids.worksetIds` keyed by label: `连载池`, `短篇池-重建`,
//     `加急池`, `归档池`. (`短篇池` is deleted in C2 and replaced by
//     `短篇池-重建`.)
//   - `ctx.ids.comicIds` keyed by title label: `星尘旅人`, `钢铁魔女`,
//     `雨夜便利店-重制版`. (`雨夜便利店` is deleted in C4.)
//   - `ctx.ids.firstChapterIds` keyed by comic label.
//   - `ctx.main` set to the `星尘旅人 / 第 2 话 月面信号` ChapterRefs with
//     chapters index 0..4 (第 3 话 deleted, 第 5 话 added; active indexes
//     [0,1,3,4]). `pageIds` stays empty until it_03.
//   - `ctx.auxChapters.get("cascade")` set to a 2-comic subtree on `归档池`
//     for it_10 (comicA with 2 chapters, comicB with 1 chapter).
//
// Covers test-plan: C1, C2, C3, C4, C5, C6, C7.
//
// Grounded permission pins (verified against src/complex):
//   - workset/comic/chapter create/update/delete: team ADMIN. sadmin is the
//     only team admin in the seed (member roles include ADMIN=128). The 14
//     personas are workers (no ADMIN bit) -> 403/4 on create.
//   - chapter create auto-creates an ADMIN assignment for the creator, so
//     sadmin can later pin/patch chapters sadmin created.
//   - chapter pin/subtitle patch: caller needs a chapter ADMIN assignment
//     (check_admin). sadmin has it from create; trans_01 does not -> 403/4.
//
// Status: IMPLEMENTED.

import assert from "node:assert/strict";

import { testEnv } from "../config/env.js";
import { expectError, expectStatus } from "../http/assertions.js";
import type { ErrorBody } from "../http/apiClient.js";
import { ApiClient } from "../http/apiClient.js";
import {
    assertComicInvariant,
    assertSubtreeInvariants,
    assertTeamInvariant,
} from "../http/invariants.js";
import {
    createChapter,
    createComic,
    createWorkset,
    deleteWorkset,
    getChapter,
    getComic,
    getPinnedChapter,
    getTeam,
    getWorkset,
    listComicChapters,
    listTeamWorksets,
    listWorksetComicInfos,
    listWorksetComics,
    patchChapter,
    updateComic,
    updateTeam,
    updateWorkset,
} from "../http/fixtures.js";
import type { ChapterInfoVal, ComicInfoVal, TeamInfoVal, WorksetInfoVal } from "../http/types.js";
import { stagePhase } from "../state/stages.js";
import { titled } from "../state/prefix.js";
import type { ChapterRefs, RunCtx } from "../state/runCtx.js";

export const IMPLEMENTED = true as const;

export async function runIt02Module(ctx: RunCtx): Promise<void> {
    const teamId = ctx.ids.defaultTeamId;
    const trans01 = ctx.users.get("trans_01");

    assert.ok(trans01, "it_01 must have registered trans_01");

    // ---------- C1. create 4 worksets, verify index monotonic ----------

    const worksetLabels = ["连载池", "短篇池", "加急池", "归档池"];
    const createdWorksets: WorksetInfoVal[] = [];

    for (let i = 0; i < worksetLabels.length; i++) {
        const label = worksetLabels[i]!;

        const ws = await createWorkset(ctx.sadmin, teamId, titled(label), `${label} desc`);

        ctx.ids.worksetIds[label] = ws.id;

        const wsInfo = await getWorkset(ctx.sadmin, ws.id);

        createdWorksets.push(wsInfo);

        // index strictly increasing and matches creation order (0-based)
        assert.equal(wsInfo.index, i);
    }

    // list contains the 4 new worksets; indexes unique and increasing
    const listed = await listTeamWorksets(ctx.sadmin, teamId);

    for (const ws of createdWorksets) {
        const found = listed.find((item) => item.id === ws.id);

        assert.ok(found, `list must include workset ${ws.id}`);
    }

    const createdIndexes = createdWorksets.map((ws) => ws.index);

    assert.equal(new Set(createdIndexes).size, createdIndexes.length, "workset indexes unique");

    await assertTeamInvariant(ctx.sadmin, teamId);

    // C1.6: non-admin (trans_01) create workset -> 403/4
    expectError(
        await trans01.api.post<ErrorBody>("/api/v1/worksets", {
            description: "x",
            name: titled("trans-ws"),
            team_id: teamId,
        }),
        403,
        4,
    );

    // C1.7: unauthenticated create -> 401/3
    expectError(
        await new ApiClient(testEnv.apiBaseUrl).post<ErrorBody>("/api/v1/worksets", {
            description: "x",
            name: "x",
            team_id: teamId,
        }),
        401,
        3,
    );

    // ---------- C2. delete middle workset + recreate (no index backfill) ----------

    const shortWsId = ctx.ids.worksetIds["短篇池"]!;

    await deleteWorkset(ctx.sadmin, shortWsId);

    // GET old workset -> 422/2
    expectError(await ctx.sadmin.get<ErrorBody>(`/api/v1/worksets/${shortWsId}`), 422, 2);

    // list no longer contains old id
    const listedAfterDelete = await listTeamWorksets(ctx.sadmin, teamId);

    assert.ok(!listedAfterDelete.find((ws) => ws.id === shortWsId), "deleted workset must not list");

    // recreate 短篇池-重建; new index must not backfill (should be > any existing index)
    const listedAfterDeleteWss = await listTeamWorksets(ctx.sadmin, teamId);
    const maxExistingWsIndex = Math.max(...listedAfterDeleteWss.map((ws) => ws.index));

    const rebuilt = await createWorkset(ctx.sadmin, teamId, titled("短篇池-重建"), "rebuilt");

    ctx.ids.worksetIds["短篇池-重建"] = rebuilt.id;

    const rebuiltInfo = await getWorkset(ctx.sadmin, rebuilt.id);

    assert.ok(rebuiltInfo.index > maxExistingWsIndex, "rebuilt index must not backfill");

    // active count back to 4
    const listedAfterRebuild = await listTeamWorksets(ctx.sadmin, teamId);

    assert.equal(listedAfterRebuild.length, 4);

    // ---------- C3. create 3 comics under 连载池, verify first chapter ----------

    const serialWsId = ctx.ids.worksetIds["连载池"]!;

    const comicSpecs: Array<{
        label: string;
        title: string;
        author: string;
        firstSubtitle: string | null;
    }> = [
        { label: "星尘旅人", title: "星尘旅人", author: "作者A", firstSubtitle: "第 1 话 旧站重启" },
        { label: "雨夜便利店", title: "雨夜便利店", author: "作者B", firstSubtitle: null },
        { label: "钢铁魔女", title: "钢铁魔女", author: "作者C", firstSubtitle: "序章" },
    ];

    for (let i = 0; i < comicSpecs.length; i++) {
        const spec = comicSpecs[i]!;
        const comic = await createComic(
            ctx.sadmin,
            serialWsId,
            titled(spec.title),
            spec.author,
            spec.firstSubtitle ?? undefined,
        );

        ctx.ids.comicIds[spec.label] = comic.id;
        ctx.ids.firstChapterIds[spec.label] = comic.chapter_id;

        const comicInfo = await getComic(ctx.sadmin, comic.id);

        assert.equal(comicInfo.workset_id, serialWsId);
        assert.equal(comicInfo.index, i, `comic ${spec.label} index monotonic`);
        assert.equal(comicInfo.chapter_count, 1);
        assert.equal(comicInfo.cover_url, null);

        const chapterInfo = await getChapter(ctx.sadmin, comic.chapter_id);

        assert.equal(chapterInfo.comic_id, comic.id);
        assert.equal(chapterInfo.index, 0);
        assert.equal(chapterInfo.page_count, 0);
        assert.equal(chapterInfo.total_unit_count, 0);
        assert.equal(chapterInfo.translated_unit_count, 0);
        assert.equal(chapterInfo.proofread_unit_count, 0);
        assert.equal(chapterInfo.creator_id, ctx.ids.defaultUserId);

        if (spec.firstSubtitle !== null) {
            assert.equal(chapterInfo.subtitle, spec.firstSubtitle);
        } else {
            assert.ok(chapterInfo.subtitle.length > 0, "default subtitle must be non-empty");
        }
    }

    // list chapters of each comic has exactly the first chapter
    for (const spec of comicSpecs) {
        const comicId = ctx.ids.comicIds[spec.label]!;

        const chapters = await listComicChapters(ctx.sadmin, comicId);

        assert.equal(chapters.length, 1);
        assert.equal(chapters[0]!.id, ctx.ids.firstChapterIds[spec.label]);
    }

    // workset comics list with with=pinned_chapter & incl=workset.team
    const serialComicPayload = await listWorksetComicInfos(
        ctx.sadmin,
        serialWsId,
        "&with=pinned_chapter&incl=workset.team",
    );

    const { comics: serialComics, pinned_chapters: pinnedChapters } = serialComicPayload;

    assert.equal(serialComics.length, 3);

    assert.equal(pinnedChapters.length, serialComics.length);

    for (const [index, comic] of serialComics.entries()) {
        assert.ok(comic.workset, "incl=workset.team must embed workset");
        assert.equal(comic.workset?.id, serialWsId, "workset.id must be the parent workset");
        assert.equal(comic.workset?.team_id, teamId, "workset.team_id must be the default team");
        assert.ok(comic.team, "workset.team incl must populate comic.team");
        assert.equal(comic.team?.id, teamId, "comic.team.id must be the default team");
        const pinnedChapter = pinnedChapters[index];

        assert.ok(pinnedChapter, "with=pinned_chapter must return a pinned chapter");

        const firstChId = ctx.ids.firstChapterIds[
            comicSpecs.find((spec) => ctx.ids.comicIds[spec.label] === comic.id)!.label
        ];

        assert.equal(
            pinnedChapter.id,
            firstChId,
            "auto-pinned chapter must be the comic's first chapter",
        );
    }

    // C3.6: non-admin create comic -> 403/4
    expectError(
        await trans01.api.post<ErrorBody>("/api/v1/comics", {
            author: "x",
            description: null,
            first_chapter_subtitle: null,
            title: titled("trans-comic"),
            workset_id: serialWsId,
        }),
        403,
        4,
    );

    // C3.7: create comic with non-existent workset_id -> 422/2
    expectError(
        await ctx.sadmin.post<ErrorBody>("/api/v1/comics", {
            author: "x",
            description: null,
            first_chapter_subtitle: null,
            title: titled("ghost-comic"),
            workset_id: "workset-does-not-exist",
        }),
        422,
        2,
    );

    // ---------- C4. delete middle comic + recreate (no index backfill) ----------

    const yuyeId = ctx.ids.comicIds["雨夜便利店"]!;
    const yuyeFirstChId = ctx.ids.firstChapterIds["雨夜便利店"]!;
    const serialComicsBeforeDelete = await listWorksetComics(ctx.sadmin, serialWsId);
    const maxExistingComicIndex = Math.max(...serialComicsBeforeDelete.map((c) => c.index));

    expectStatus(await ctx.sadmin.delete<null>(`/api/v1/comics/${yuyeId}`), 204);

    // GET deleted comic -> 422/2
    expectError(await ctx.sadmin.get<ErrorBody>(`/api/v1/comics/${yuyeId}`), 422, 2);

    // list excludes deleted comic
    const serialComicsAfterDelete = await listWorksetComics(ctx.sadmin, serialWsId);

    assert.ok(!serialComicsAfterDelete.find((comic) => comic.id === yuyeId));

    // workset comic_count == 2 (active)
    const serialWsAfterDelete = await getWorkset(ctx.sadmin, serialWsId);

    assert.equal(serialWsAfterDelete.comic_count, 2);

    // deleted comic's first chapter -> 422/2
    expectError(await ctx.sadmin.get<ErrorBody>(`/api/v1/chapters/${yuyeFirstChId}`), 422, 2);

    // recreate 雨夜便利店-重制版; new index must not backfill (> any existing)
    const yuyeReborn = await createComic(
        ctx.sadmin,
        serialWsId,
        titled("雨夜便利店-重制版"),
        "作者B2",
        "第 1 话 重制",
    );

    ctx.ids.comicIds["雨夜便利店-重制版"] = yuyeReborn.id;
    ctx.ids.firstChapterIds["雨夜便利店-重制版"] = yuyeReborn.chapter_id;

    const yuyeRebornInfo = await getComic(ctx.sadmin, yuyeReborn.id);

    assert.ok(yuyeRebornInfo.index > maxExistingComicIndex, "rebuilt comic index no backfill");

    // workset comic_count back to 3
    const serialWsAfterRebuild = await getWorkset(ctx.sadmin, serialWsId);

    assert.equal(serialWsAfterRebuild.comic_count, 3);

    // fuzzy_title=雨夜 finds 重制版, not the deleted original
    const fuzzyYuye = await listWorksetComics(
        ctx.sadmin,
        serialWsId,
        `&fuzzy_title=${encodeURIComponent(titled("雨夜"))}`,
    );

    assert.ok(fuzzyYuye.find((comic) => comic.id === yuyeReborn.id), "fuzzy must find 重制版");
    assert.ok(!fuzzyYuye.find((comic) => comic.id === yuyeId), "fuzzy must not find deleted original");

    // ---------- C5. multi-chapter on 星尘旅人, verify chapter index ----------

    const xingchenId = ctx.ids.comicIds["星尘旅人"]!;
    const xingchenFirstCh = ctx.ids.firstChapterIds["星尘旅人"]!;
    const chapterSpecs: Array<{ label: string; subtitle: string }> = [
        { label: "ch2", subtitle: "第 2 话 月面信号" },
        { label: "ch3", subtitle: "第 3 话 失控列车" },
        { label: "ch4", subtitle: "第 4 话 地下港口" },
    ];

    const chapterIdsByLabel: Record<string, string> = { ch1: xingchenFirstCh };

    const existingChapters = await listComicChapters(ctx.sadmin, xingchenId);
    const maxExistingChIndex = Math.max(...existingChapters.map((ch) => ch.index), 0);

    for (let i = 0; i < chapterSpecs.length; i++) {
        const spec = chapterSpecs[i]!;
        const ch = await createChapter(ctx.sadmin, xingchenId, titled(spec.subtitle));

        chapterIdsByLabel[spec.label] = ch.id;

        const xingchenAfter = await getComic(ctx.sadmin, xingchenId);

        assert.equal(xingchenAfter.chapter_count, 2 + i, "chapter_count == active count");
    }

    // delete 第 3 话 (ch3)
    const ch3Id = chapterIdsByLabel["ch3"]!;
    const chaptersBeforeChDelete = await listComicChapters(ctx.sadmin, xingchenId);
    const maxChIndexBeforeDelete = Math.max(...chaptersBeforeChDelete.map((ch) => ch.index));

    expectStatus(await ctx.sadmin.delete<null>(`/api/v1/chapters/${ch3Id}`), 204);

    expectError(await ctx.sadmin.get<ErrorBody>(`/api/v1/chapters/${ch3Id}`), 422, 2);

    // chapter_count -1
    const xingchenAfterChDelete = await getComic(ctx.sadmin, xingchenId);

    assert.equal(xingchenAfterChDelete.chapter_count, 3);

    // create 第 5 话 断层回声; new index must not backfill (> any existing)
    const ch5 = await createChapter(ctx.sadmin, xingchenId, titled("第 5 话 断层回声"));

    chapterIdsByLabel["ch5"] = ch5.id;

    const ch5Info = await getChapter(ctx.sadmin, ch5.id);

    assert.ok(ch5Info.index > maxChIndexBeforeDelete, "new chapter index no backfill");

    // active indexes are [0,1,3,4] (ch1=0, ch2=1, ch4=3, ch5=4) — no backfill
    const activeChapters = await listComicChapters(ctx.sadmin, xingchenId);

    const activeIndexes = activeChapters.map((ch) => ch.index).sort((a, b) => a - b);

    assert.deepEqual(activeIndexes, [0, 1, 3, 4], "chapter indexes no backfill");

    // incl=comic.workset.team: nested id chain correct
    const withIncl = await listComicChapters(
        ctx.sadmin,
        xingchenId,
        "&incl=comic.workset.team",
    );

    for (const ch of withIncl) {
        assert.ok(ch.comic, "comic embedded");
        assert.equal(ch.comic?.id, xingchenId);
        assert.ok(ch.comic?.workset, "workset embedded");
        assert.equal(ch.comic?.workset?.id, serialWsId);
        assert.ok(ch.comic?.team, "workset.team incl populates comic.team");
        assert.equal(ch.comic?.team?.id, teamId);
        // creator not requested — must be null
        assert.equal(ch.creator, null, "creator not included");
    }

    // C5.8: non-admin create chapter -> 403/4
    expectError(
        await trans01.api.post<ErrorBody>("/api/v1/chapters", {
            comic_id: xingchenId,
            subtitle: titled("trans-ch"),
        }),
        403,
        4,
    );

    // C5.9: create chapter with non-existent comic_id -> 422/2
    expectError(
        await ctx.sadmin.post<ErrorBody>("/api/v1/chapters", {
            comic_id: "comic-does-not-exist",
            subtitle: "x",
        }),
        422,
        2,
    );

    // Set ctx.main to 第 2 话 月面信号 (ch2) — the high-traffic chapter.
    ctx.main = {
        chapterId: chapterIdsByLabel["ch2"]!,
        comicId: xingchenId,
        worksetId: serialWsId,
        pageIds: [],
        assignmentIds: {},
    };

    // ---------- C6. chapter pin/unpin ----------

    const ch2Id = chapterIdsByLabel["ch2"]!;
    const ch4Id = chapterIdsByLabel["ch4"]!;

    // pin ch2
    await patchChapter(ctx.sadmin, ch2Id, { pin: true });

    let pinned = await getPinnedChapter(ctx.sadmin, xingchenId);

    assert.equal(pinned?.id ?? null, ch2Id, "pinned endpoint returns ch2");

    // pin ch4 -> pin moves to ch4, ch2 unpins
    await patchChapter(ctx.sadmin, ch4Id, { pin: true });

    pinned = await getPinnedChapter(ctx.sadmin, xingchenId);

    assert.equal(pinned?.id ?? null, ch4Id, "pin moves to ch4");

    const ch2After = await getChapter(ctx.sadmin, ch2Id);

    assert.equal(ch2After.is_pinned, false, "ch2 unpinned after ch4 pinned");

    // unpin ch4
    await patchChapter(ctx.sadmin, ch4Id, { pin: false });

    pinned = await getPinnedChapter(ctx.sadmin, xingchenId);

    assert.equal(pinned, null, "pinned endpoint null after unpin");

    // C6.5: path/body id mismatch -> 422 code 7
    expectError(
        await ctx.sadmin.patch<ErrorBody>(`/api/v1/chapters/${ch2Id}`, {
            id: "not-the-path-id",
            pin: true,
        }),
        422,
        7,
    );

    // C6.6: non-authorized user patch -> 403/4 (trans_01 has no admin assignment)
    expectError(
        await trans01.api.patch<ErrorBody>(`/api/v1/chapters/${ch2Id}`, {
            id: ch2Id,
            pin: true,
        }),
        403,
        4,
    );

    // C6.7: subtitle-only patch leaves pin unchanged; pin-only patch leaves subtitle unchanged
    const ch2Before = await getChapter(ctx.sadmin, ch2Id);
    const ch2OriginalSubtitle = ch2Before.subtitle;

    await patchChapter(ctx.sadmin, ch2Id, { subtitle: titled("第 2 话 改名") });

    const ch2AfterSub = await getChapter(ctx.sadmin, ch2Id);

    assert.equal(ch2AfterSub.subtitle, titled("第 2 话 改名"));
    assert.equal(ch2AfterSub.is_pinned, ch2Before.is_pinned, "subtitle patch must not change pin");

    // restore subtitle for downstream modules
    await patchChapter(ctx.sadmin, ch2Id, { subtitle: ch2OriginalSubtitle });

    await patchChapter(ctx.sadmin, ch2Id, { pin: true });

    const ch2AfterPin = await getChapter(ctx.sadmin, ch2Id);

    assert.equal(ch2AfterPin.is_pinned, true);
    assert.equal(ch2AfterPin.subtitle, ch2OriginalSubtitle, "pin patch must not change subtitle");

    // unpin to leave clean state
    await patchChapter(ctx.sadmin, ch2Id, { pin: false });

    // ---------- C7. info update full coverage ----------

    // team update
    const teamForUpdate = await getTeam(ctx.sadmin, teamId);
    const teamOriginalName = teamForUpdate.name;
    const teamOriginalDesc = teamForUpdate.description;
    const teamOriginalUpdated = teamForUpdate.updated_at;

    await updateTeam(ctx.sadmin, teamId, titled("team-renamed"), "updated desc");

    const teamUpdated = await getTeam(ctx.sadmin, teamId);

    assert.equal(teamUpdated.name, titled("team-renamed"));
    assert.equal(teamUpdated.description, "updated desc");
    assert.ok(teamUpdated.updated_at >= teamOriginalUpdated, "updated_at must not decrease");

    // restore team profile (downstream modules / cleanup assume seed name)
    await updateTeam(ctx.sadmin, teamId, teamOriginalName, teamOriginalDesc);

    // team path/body id mismatch -> 422 code 7
    expectError(
        await ctx.sadmin.put<ErrorBody>(`/api/v1/teams/${teamId}`, {
            description: "x",
            id: "not-the-path-id",
            name: "x",
        }),
        422,
        7,
    );

    // workset update
    const wsForUpdate = await getWorkset(ctx.sadmin, serialWsId);
    const wsOriginalName = wsForUpdate.name;
    const wsOriginalDesc = wsForUpdate.description ?? "";

    await updateWorkset(ctx.sadmin, serialWsId, titled("ws-renamed"), "ws updated");

    const wsUpdated = await getWorkset(ctx.sadmin, serialWsId);

    assert.equal(wsUpdated.name, titled("ws-renamed"));
    assert.equal(wsUpdated.description, "ws updated");
    assert.equal(wsUpdated.index, wsForUpdate.index, "index unchanged");
    assert.equal(wsUpdated.comic_count, wsForUpdate.comic_count, "comic_count unchanged");

    // restore workset name
    await updateWorkset(ctx.sadmin, serialWsId, wsOriginalName, wsOriginalDesc || undefined);

    // comic update
    const comicForUpdate = await getComic(ctx.sadmin, xingchenId);
    const comicOriginalTitle = comicForUpdate.title;
    const comicOriginalAuthor = comicForUpdate.author;

    await updateComic(ctx.sadmin, xingchenId, titled("comic-renamed"), "new author", "new desc");

    const comicUpdated = await getComic(ctx.sadmin, xingchenId);

    assert.equal(comicUpdated.title, titled("comic-renamed"));
    assert.equal(comicUpdated.author, "new author");
    assert.equal(comicUpdated.description, "new desc");
    assert.equal(comicUpdated.index, comicForUpdate.index, "index unchanged");
    assert.equal(comicUpdated.chapter_count, comicForUpdate.chapter_count, "chapter_count unchanged");

    // restore comic profile
    await updateComic(ctx.sadmin, xingchenId, comicOriginalTitle, comicOriginalAuthor);

    // chapter subtitle patch (already exercised in C6.7); counters must not change
    const chForUpdate = await getChapter(ctx.sadmin, ch2Id);
    const chOriginalSubtitle = chForUpdate.subtitle;

    await patchChapter(ctx.sadmin, ch2Id, { subtitle: titled("ch-renamed") });

    const chUpdated = await getChapter(ctx.sadmin, ch2Id);

    assert.equal(chUpdated.subtitle, titled("ch-renamed"));
    assert.equal(chUpdated.index, chForUpdate.index);
    assert.equal(chUpdated.page_count, chForUpdate.page_count);
    assert.equal(chUpdated.total_unit_count, chForUpdate.total_unit_count);
    assert.equal(chUpdated.translated_unit_count, chForUpdate.translated_unit_count);
    assert.equal(chUpdated.proofread_unit_count, chForUpdate.proofread_unit_count);
    assert.equal(chUpdated.stages, chForUpdate.stages, "stages unchanged by subtitle patch");

    // restore chapter subtitle
    await patchChapter(ctx.sadmin, ch2Id, { subtitle: chOriginalSubtitle });

    // C7.6: non-admin update team/workset/comic/chapter profile -> 403/4 each
    expectError(
        await trans01.api.put<ErrorBody>(`/api/v1/teams/${teamId}`, {
            description: "x",
            id: teamId,
            name: "x",
        }),
        403,
        4,
    );

    expectError(
        await trans01.api.put<ErrorBody>(`/api/v1/worksets/${serialWsId}`, {
            description: "x",
            id: serialWsId,
            name: "x",
        }),
        403,
        4,
    );

    expectError(
        await trans01.api.put<ErrorBody>(`/api/v1/comics/${xingchenId}`, {
            author: "x",
            description: null,
            id: xingchenId,
            title: "x",
        }),
        403,
        4,
    );

    expectError(
        await trans01.api.patch<ErrorBody>(`/api/v1/chapters/${ch2Id}`, {
            id: ch2Id,
            subtitle: "x",
        }),
        403,
        4,
    );

    // C7.7: update with non-existent id -> 422/2
    expectError(
        await ctx.sadmin.put<ErrorBody>("/api/v1/worksets/workset-does-not-exist", {
            description: "x",
            id: "workset-does-not-exist",
            name: "x",
        }),
        422,
        2,
    );

    expectError(
        await ctx.sadmin.put<ErrorBody>("/api/v1/comics/comic-does-not-exist", {
            author: "x",
            description: null,
            id: "comic-does-not-exist",
            title: "x",
        }),
        422,
        2,
    );

    // non-existent chapter patch -> 403/4 (admin check runs before existence check)
    expectError(
        await ctx.sadmin.patch<ErrorBody>("/api/v1/chapters/chapter-does-not-exist", {
            id: "chapter-does-not-exist",
            subtitle: "x",
        }),
        403,
        4,
    );

    // ---------- aux: cascade subtree on 归档池 for it_10 ----------

    const archiveWsId = ctx.ids.worksetIds["归档池"]!;

    const cascadeComicA = await createComic(
        ctx.sadmin,
        archiveWsId,
        titled("归档漫画A"),
        "authA",
        "归档A第1话",
    );

    const cascadeComicACh1 = cascadeComicA.chapter_id;
    const cascadeComicACh2 = (await createChapter(ctx.sadmin, cascadeComicA.id, titled("归档A第2话"))).id;

    const cascadeComicB = await createComic(
        ctx.sadmin,
        archiveWsId,
        titled("归档漫画B"),
        "authB",
        "归档B第1话",
    );

    const cascadeComicBCh1 = cascadeComicB.chapter_id;

    ctx.auxChapters.set("cascade", {
        chapterId: cascadeComicACh2,
        comicId: cascadeComicA.id,
        worksetId: archiveWsId,
        pageIds: [],
        assignmentIds: {},
    });

    // keep the other cascade chapter/comic ids reachable for it_10 via auxChapters
    // (store them as extra fields on the ChapterRefs through a side map)
    cascadeExtraIds.cascadeComicAId = cascadeComicA.id;
    cascadeExtraIds.cascadeComicACh1 = cascadeComicACh1;
    cascadeExtraIds.cascadeComicBId = cascadeComicB.id;
    cascadeExtraIds.cascadeComicBCh1 = cascadeComicBCh1;

    // ---------- final invariants ----------

    await assertSubtreeInvariants(ctx.sadmin, serialWsId);
    await assertSubtreeInvariants(ctx.sadmin, archiveWsId);
    await assertTeamInvariant(ctx.sadmin, teamId);

    // sanity: main chapter is at workflow baseline
    const mainChapter = await getChapter(ctx.sadmin, ctx.main.chapterId);

    assert.equal(mainChapter.stages, 0, "main chapter must start at workflow baseline");
    void stagePhase; // referenced for downstream modules' convenience
}

// Side-channel for it_10 to reach the full cascade subtree (the other comic
// + chapters not held in `ctx.auxChapters.get("cascade")`).
export const cascadeExtraIds: {
    cascadeComicAId: string;
    cascadeComicACh1: string;
    cascadeComicBId: string;
    cascadeComicBCh1: string;
} = {
    cascadeComicAId: "",
    cascadeComicACh1: "",
    cascadeComicBId: "",
    cascadeComicBCh1: "",
};

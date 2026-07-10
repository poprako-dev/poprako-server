// it_03 — Page reserve, image mark-uploaded, page index/counters, delete+rebuild.
//
// Preconditions:
//   - it_00 + it_01 + it_02 have run.
//   - `ctx.main` is `星尘旅人 / 第 2 话 月面信号` with no pages yet.
//   - `ctx.auxChapters.get("cascade")` exists (2 comics on 归档池).
//
// Postconditions:
//   - `ctx.main.pageIds` filled with 8 page ids (index 0..7), each image
//     marked uploaded.
//   - `ctx.auxChapters.get("cascade")` has 2 pages reserved on its chapter
//     (and on the other cascade chapter) for it_10.
//   - `ctx.auxChapters.get("d3")` set to a `钢铁魔女 / 第 2 话 D3辅助`
//     chapter with 3 pages + 6 units, then deleted-and-rebuilt to 2 pages.
//
// Covers test-plan: D1, D2, D3.
//
// Grounded permission pins:
//   - page reserve (batch + single): assignment with RAW_PROVIDER or REVIEWER.
//   - page image mark-uploaded: assignment with RAW_PROVIDER.
//   - page delete-all: team ADMIN.
//   - sadmin's auto-assignment from chapter create is ADMIN only, so sadmin
//     CANNOT reserve pages until granted RAW_PROVIDER. We reuse
//     `grantChapterWorkerRoles` (sets raw_provider + translator timestamps)
//     to add RAW_PROVIDER to sadmin's admin assignment on the main chapter.
//   - guest_01 has no assignment on main -> reserve/mark -> 403/4.
//
// Status: IMPLEMENTED.

import assert from "node:assert/strict";

import { grantChapterWorkerRoles } from "../db/seed.js";
import { expectError, expectStatus } from "../http/assertions.js";
import type { ErrorBody } from "../http/apiClient.js";
import {
    assertChapterInvariant,
    assertChapterPageCountersConsistent,
} from "../http/invariants.js";
import {
    createChapter,
    deleteChapterPages,
    getChapter,
    listChapterPages,
    markPageImageUploaded,
    newBubbleUnit,
    reserveChapterPages,
    reservePageImage,
    savePageUnits,
} from "../http/fixtures.js";
import { titled } from "../state/prefix.js";
import type { ChapterRefs, RunCtx } from "../state/runCtx.js";
import { cascadeExtraIds } from "./it_02_workset_comic_chapter_index.js";

export const IMPLEMENTED = true as const;

export async function runIt03Module(ctx: RunCtx): Promise<void> {
    assert.ok(ctx.main, "it_02 must have set ctx.main");
    assert.ok(ctx.ids.defaultUserId);

    const mainChapterId = ctx.main.chapterId;
    const guest01 = ctx.users.get("guest_01");

    assert.ok(guest01, "it_01 must have registered guest_01");

    // Grant sadmin RAW_PROVIDER (+ TRANSLATOR) on main so page reserve/mark
    // and unit save (translator) are permitted.
    await grantChapterWorkerRoles(mainChapterId, ctx.ids.defaultUserId);

    // ---------- D1. batch reserve 8 pages on main ----------

    const reserveVal = await reserveChapterPages(ctx.sadmin, mainChapterId, 8, "jpg");

    assert.equal(reserveVal.creations.length, 8);

    const pageIds: string[] = [];
    const pageVersions = new Map<string, number>();
    const seenPageIds = new Set<string>();

    for (const creation of reserveVal.creations) {
        assert.ok(creation.page_id);
        assert.ok(creation.put_url.startsWith("http"), "put_url must be an http url");
        assert.ok(Number.isInteger(creation.image_version) && creation.image_version > 0);

        assert.ok(!seenPageIds.has(creation.page_id), "page ids must be unique");
        seenPageIds.add(creation.page_id);

        pageIds.push(creation.page_id);
        pageVersions.set(creation.page_id, creation.image_version);
    }

    ctx.main.pageIds = pageIds;

    // list pages: 8 pages, index 0..7, all unit counts 0
    const pages = await listChapterPages(ctx.sadmin, mainChapterId);

    assert.equal(pages.length, 8);

    const sortedPages = [...pages].sort((a, b) => a.index - b.index);

    sortedPages.forEach((page, i) => {
        assert.equal(page.index, i, `page index must be ${i}`);
        assert.equal(page.total_unit_count, 0);
        assert.equal(page.translated_unit_count, 0);
        assert.equal(page.proofread_unit_count, 0);
    });

    // chapter counters
    const mainChapter = await getChapter(ctx.sadmin, mainChapterId);

    assert.equal(mainChapter.page_count, 8);
    assert.equal(mainChapter.total_unit_count, 0);
    assert.equal(mainChapter.translated_unit_count, 0);
    assert.equal(mainChapter.proofread_unit_count, 0);

    // reserve again on same chapter -> 422/2 (already reserved)
    expectError(
        await ctx.sadmin.post<ErrorBody>(`/api/v1/chapters/${mainChapterId}/pages/reserve`, {
            chapter_id: mainChapterId,
            file_ext: "jpg",
            page_count: 1,
        }),
        422,
        2,
    );

    // page_count 0 -> 422/2
    expectError(
        await ctx.sadmin.post<ErrorBody>(`/api/v1/chapters/${mainChapterId}/pages/reserve`, {
            chapter_id: mainChapterId,
            file_ext: "jpg",
            page_count: 0,
        }),
        422,
        2,
    );

    // path chapter_id / body chapter_id mismatch -> 422 (path/body id rule)
    expectError(
        await ctx.sadmin.post<ErrorBody>(`/api/v1/chapters/${mainChapterId}/pages/reserve`, {
            chapter_id: "not-the-path-id",
            file_ext: "jpg",
            page_count: 1,
        }),
        422,
        7,
    );

    await assertChapterInvariant(ctx.sadmin, mainChapterId);

    // ---------- D2. mark-uploaded + single page replace ----------

    // mark all 8 pages uploaded
    for (const pageId of pageIds) {
        const version = pageVersions.get(pageId)!;

        expectStatus(
            await ctx.sadmin.post<null>(`/api/v1/pages/${pageId}/image/mark-uploaded`, {
                image_version: version,
            }),
            204,
        );
    }

    // after mark, each page image_url non-null
    const markedPages = await listChapterPages(ctx.sadmin, mainChapterId);

    for (const page of markedPages) {
        assert.ok(page.image_url, `page ${page.id} must have image_url after mark`);
    }

    // single page replace on page index 2 (p2)
    const p2Id = pageIds[2]!;
    const p2OldVersion = pageVersions.get(p2Id)!;

    const p2Reserve = await reservePageImage(ctx.sadmin, p2Id, "png");

    assert.equal(p2Reserve.page_id, p2Id);
    assert.ok(p2Reserve.put_url.startsWith("http"));
    assert.ok(p2Reserve.image_version !== undefined && p2Reserve.image_version > p2OldVersion, "new image_version must exceed old");

    const p2NewVersion = p2Reserve.image_version!;

    // mark new version
    expectStatus(
        await ctx.sadmin.post<null>(`/api/v1/pages/${p2Id}/image/mark-uploaded`, {
            image_version: p2NewVersion,
        }),
        204,
    );

    // p2 image_url still non-null; updated_at increased
    const p2After = (await listChapterPages(ctx.sadmin, mainChapterId)).find((p) => p.id === p2Id);

    assert.ok(p2After?.image_url, "p2 image_url still non-null after replace");

    // stale version mark -> 422/2 (version mismatch)
    expectError(
        await ctx.sadmin.post<ErrorBody>(`/api/v1/pages/${p2Id}/image/mark-uploaded`, {
            image_version: p2OldVersion,
        }),
        422,
        2,
    );

    // D2.6: non-assigned member (guest_01 has no assignment on main) mark/reserve -> 403/4
    expectError(
        await guest01.api.post<ErrorBody>(`/api/v1/pages/${p2Id}/image/mark-uploaded`, {
            image_version: p2NewVersion,
        }),
        403,
        4,
    );

    expectError(
        await guest01.api.post<ErrorBody>(`/api/v1/pages/${p2Id}/image/reserve`, {
            file_ext: "jpg",
        }),
        403,
        4,
    );

    // D2.7: non-existent page_id reserve/mark -> 422/2
    expectError(
        await ctx.sadmin.post<ErrorBody>("/api/v1/pages/page-does-not-exist/image/reserve", {
            file_ext: "jpg",
        }),
        422,
        2,
    );

    expectError(
        await ctx.sadmin.post<ErrorBody>("/api/v1/pages/page-does-not-exist/image/mark-uploaded", {
            image_version: 1,
        }),
        422,
        2,
    );

    // ---------- D3. delete all pages + rebuild (on aux chapter, NOT main) ----------

    // Build the d3 aux chapter on 钢铁魔女.
    const gangtieId = ctx.ids.comicIds["钢铁魔女"]!;

    const d3Chapter = await createChapter(ctx.sadmin, gangtieId, titled("第 2 话 D3辅助"));

    const d3ChapterId = d3Chapter.id;

    // grant sadmin worker roles on d3 chapter (gets RAW_PROVIDER + TRANSLATOR)
    await grantChapterWorkerRoles(d3ChapterId, ctx.ids.defaultUserId);

    const d3Refs: ChapterRefs = {
        chapterId: d3ChapterId,
        comicId: gangtieId,
        worksetId: ctx.ids.worksetIds["连载池"]!,
        pageIds: [],
        assignmentIds: {},
    };

    ctx.auxChapters.set("d3", d3Refs);

    // reserve 3 pages on d3
    const d3Reserve = await reserveChapterPages(ctx.sadmin, d3ChapterId, 3, "jpg");

    assert.equal(d3Reserve.creations.length, 3);

    const d3PageIds = d3Reserve.creations.map((c) => c.page_id);

    d3Refs.pageIds = d3PageIds;

    // write 2 units on each of the 3 pages (sadmin has TRANSLATOR via grant)
    for (const pageId of d3PageIds) {
        const save = await savePageUnits(ctx.sadmin, pageId, [
            newBubbleUnit("d3_u1", 0.1, 0.1),
            newBubbleUnit("d3_u2", 0.2, 0.2),
        ]);

        assert.equal(save.total_unit_count, 2);
    }

    const d3ChapterBefore = await getChapter(ctx.sadmin, d3ChapterId);

    assert.equal(d3ChapterBefore.page_count, 3);
    assert.equal(d3ChapterBefore.total_unit_count, 6);
    assert.equal(d3ChapterBefore.translated_unit_count, 0);
    assert.equal(d3ChapterBefore.proofread_unit_count, 0);

    // delete all pages
    await deleteChapterPages(ctx.sadmin, d3ChapterId);

    // list pages empty
    const d3PagesAfterDelete = await listChapterPages(ctx.sadmin, d3ChapterId);

    assert.equal(d3PagesAfterDelete.length, 0);

    // chapter counters all zero
    const d3ChapterAfter = await getChapter(ctx.sadmin, d3ChapterId);

    assert.equal(d3ChapterAfter.page_count, 0);
    assert.equal(d3ChapterAfter.total_unit_count, 0);
    assert.equal(d3ChapterAfter.translated_unit_count, 0);
    assert.equal(d3ChapterAfter.proofread_unit_count, 0);

    // rebuild: reserve 2 pages
    const d3RebuildReserve = await reserveChapterPages(ctx.sadmin, d3ChapterId, 2, "jpg");

    assert.equal(d3RebuildReserve.creations.length, 2);

    const d3RebuildPages = await listChapterPages(ctx.sadmin, d3ChapterId);

    assert.equal(d3RebuildPages.length, 2);

    const d3RebuildSorted = [...d3RebuildPages].sort((a, b) => a.index - b.index);

    d3RebuildSorted.forEach((page, i) => {
        assert.equal(page.index, i, `rebuild page index must be ${i}`);
    });

    // old page id units -> 422/2
    const oldD3PageId = d3PageIds[0]!;

    expectError(
        await ctx.sadmin.get<ErrorBody>(`/api/v1/pages/${oldD3PageId}/units?offset=0&limit=20`),
        422,
        2,
    );

    // old page id image reserve -> 422/2
    expectError(
        await ctx.sadmin.post<ErrorBody>(`/api/v1/pages/${oldD3PageId}/image/reserve`, {
            file_ext: "jpg",
        }),
        422,
        2,
    );

    // update d3Refs pageIds to the rebuilt pages
    d3Refs.pageIds = d3RebuildReserve.creations.map((c) => c.page_id);

    await assertChapterInvariant(ctx.sadmin, d3ChapterId);
    await assertChapterPageCountersConsistent(ctx.sadmin, d3ChapterId);

    // ---------- aux: reserve pages on cascade chapters for it_10 ----------

    const cascadeRefs = ctx.auxChapters.get("cascade");

    assert.ok(cascadeRefs, "it_02 must have set cascade aux chapter");

    // Grant sadmin RAW_PROVIDER on each cascade chapter first (reserve needs
    // RAW_PROVIDER; sadmin's auto-assignment from create is ADMIN only).
    await grantChapterWorkerRoles(cascadeRefs.chapterId, ctx.ids.defaultUserId);
    await grantChapterWorkerRoles(cascadeExtraIds.cascadeComicACh1, ctx.ids.defaultUserId);
    await grantChapterWorkerRoles(cascadeExtraIds.cascadeComicBCh1, ctx.ids.defaultUserId);

    // reserve 2 pages on cascadeComicACh2 (the cascade ChapterRefs chapterId)
    const cascadeACh2Reserve = await reserveChapterPages(
        ctx.sadmin,
        cascadeRefs.chapterId,
        2,
        "jpg",
    );

    assert.equal(cascadeACh2Reserve.creations.length, 2);

    cascadeRefs.pageIds = cascadeACh2Reserve.creations.map((c) => c.page_id);

    // reserve 2 pages on cascadeComicACh1 and cascadeComicBCh1 too
    const reserveACh1 = await reserveChapterPages(
        ctx.sadmin,
        cascadeExtraIds.cascadeComicACh1,
        2,
        "jpg",
    );

    const reserveBCh1 = await reserveChapterPages(
        ctx.sadmin,
        cascadeExtraIds.cascadeComicBCh1,
        2,
        "jpg",
    );

    assert.equal(reserveACh1.creations.length, 2);
    assert.equal(reserveBCh1.creations.length, 2);

    // sanity: main chapter invariant still holds
    await assertChapterInvariant(ctx.sadmin, mainChapterId);
}

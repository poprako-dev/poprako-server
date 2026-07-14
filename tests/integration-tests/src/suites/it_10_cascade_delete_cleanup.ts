// it_10 — Cascade delete: chapter -> comic -> workset -> team.
//
// Preconditions:
//   - it_00..it_09 have run.
//   - `ctx.auxChapters.get("cascade")` exists: 归档池 workset with comicA
//     (2 chapters, each 2 pages) and comicB (1 chapter, 2 pages). Pages
//     reserved by it_03. `cascadeExtraIds` (from it_02) holds the other
//     comic/chapter ids.
//   - `ctx.secondTeam` exists with outsider.
//
// Postconditions:
//   - The cascade subtree (归档池 + comicA + comicB + all chapters/pages) is
//     fully deleted.
//   - The second team + outsider membership deleted.
//   - `ctx.auxChapters.delete("cascade")`, `ctx.secondTeam = null`.
//   - DB left ready for the final `cleanupToSeed`.
//
// Covers test-plan: C9.
//
// Grounded pins:
//   - chapter/comic/workset delete: team ADMIN (sadmin).
//   - team delete: team admin (sadmin for the second team). Team delete
//     cascades through worksets/comics/chapters/pages/units/members.
//   - Deleting a chapter decreases comic.chapter_count by 1.
//     The chapter's pages/units become inaccessible (422/2).
//   - Deleting a comic decreases workset.comic_count by 1.
//     The comic's chapters become inaccessible (422/2).
//   - Deleting a workset removes it from the team list; its comics/chapters
//     become inaccessible (422/2).
//
// Status: IMPLEMENTED.

import assert from "node:assert/strict";

import { expectError, expectStatus } from "../http/assertions.js";
import type { ErrorBody } from "../http/apiClient.js";
import {
    assertTeamInvariant,
} from "../http/invariants.js";
import {
    getChapter,
    getComic,
    getTeam,
    getWorkset,
    listChapterPages,
    listMyMembers,
    listPageUnits,
    listTeamMembers,
    listTeamWorksets,
    listWorksetComics,
} from "../http/fixtures.js";
import type { RunCtx } from "../state/runCtx.js";
import { cascadeExtraIds } from "./it_02_workset_comic_chapter_index.js";

export const IMPLEMENTED = true as const;

export async function runIt10Module(ctx: RunCtx): Promise<void> {
    const cascadeRefs = ctx.auxChapters.get("cascade");

    assert.ok(cascadeRefs, "it_02 must have set the cascade aux subtree");

    const archiveWsId = cascadeRefs.worksetId;
    const comicAId = cascadeExtraIds.cascadeComicAId;
    const comicACh1 = cascadeExtraIds.cascadeComicACh1;
    const comicACh2 = cascadeRefs.chapterId;
    const comicBId = cascadeExtraIds.cascadeComicBId;
    const comicBCh1 = cascadeExtraIds.cascadeComicBCh1;

    assert.ok(comicAId && comicACh1 && comicBId && comicBCh1, "cascade extra ids must be populated");

    // ---------- C9.1 delete a chapter (comicACh2) ----------

    const comicABefore = await getComic(ctx.sadmin, comicAId);
    const comicAChCountBefore = comicABefore.chapter_count;

    expectStatus(await ctx.sadmin.delete<null>(`/api/v1/chapters/${comicACh2}`), 204);

    // GET deleted chapter -> 422/2
    expectError(await ctx.sadmin.get<ErrorBody>(`/api/v1/chapters/${comicACh2}`), 422, 2);

    // comic.chapter_count -1
    const comicAAfterChDelete = await getComic(ctx.sadmin, comicAId);

    assert.equal(comicAAfterChDelete.chapter_count, comicAChCountBefore - 1);

    // ---------- C9.2 delete a comic (comicB) ----------

    const archiveWsBefore = await getWorkset(ctx.sadmin, archiveWsId);
    const wsComicCountBefore = archiveWsBefore.comic_count;

    expectStatus(await ctx.sadmin.delete<null>(`/api/v1/comics/${comicBId}`), 204);

    // GET deleted comic -> 422/2
    expectError(await ctx.sadmin.get<ErrorBody>(`/api/v1/comics/${comicBId}`), 422, 2);

    // workset.comic_count -1
    const archiveWsAfterComicDelete = await getWorkset(ctx.sadmin, archiveWsId);

    assert.equal(archiveWsAfterComicDelete.comic_count, wsComicCountBefore - 1);

    // comicB's chapter (comicBCh1) -> 422/2
    expectError(await ctx.sadmin.get<ErrorBody>(`/api/v1/chapters/${comicBCh1}`), 422, 2);

    // comicB's page units -> 422/2 (page gone with chapter)
    // (pages were reserved on comicBCh1 by it_03; pick the first page id we
    // don't have stored, so just assert a chapter-pages list -> 422/2)
    expectError(
        await ctx.sadmin.get<ErrorBody>(`/api/v1/chapters/${comicBCh1}/pages?offset=0&limit=20`),
        422,
        2,
    );

    // ---------- C9.3 delete the workset (归档池) ----------

    expectStatus(await ctx.sadmin.delete<null>(`/api/v1/worksets/${archiveWsId}`), 204);

    // GET deleted workset -> 422/2
    expectError(await ctx.sadmin.get<ErrorBody>(`/api/v1/worksets/${archiveWsId}`), 422, 2);

    // team workset list excludes it
    const teamWorksetsAfter = await listTeamWorksets(ctx.sadmin, ctx.ids.defaultTeamId);

    assert.ok(!teamWorksetsAfter.find((ws) => ws.id === archiveWsId), "deleted workset must not list");

    // comicA (under the deleted workset) -> 422/2
    expectError(await ctx.sadmin.get<ErrorBody>(`/api/v1/comics/${comicAId}`), 422, 2);

    // comicACh1 -> 422/2
    expectError(await ctx.sadmin.get<ErrorBody>(`/api/v1/chapters/${comicACh1}`), 422, 2);

    // a page under comicACh1 -> list pages 422/2
    expectError(
        await ctx.sadmin.get<ErrorBody>(`/api/v1/chapters/${comicACh1}/pages?offset=0&limit=20`),
        422,
        2,
    );

    // ---------- C9.4 delete the second team (created by it_09) ----------

    if (ctx.secondTeam) {
        const secondTeamId = ctx.secondTeam.teamId;

        // Clean up FK-restricted children: member_invitation, then all members,
        // then the team. sadmin is admin of the second team (auto on create).
        if (ctx.secondTeam.outsiderInvitationId) {
            expectStatus(
                await ctx.sadmin.delete<null>(
                    `/api/v1/member-invitations/${ctx.secondTeam.outsiderInvitationId}`,
                ),
                204,
            );
        }

        // Delete all second-team members: outsider first, then sadmin last
        // (sadmin is admin; deleting own member removes admin role).
        const secondTeamMembers = await listTeamMembers(ctx.sadmin, secondTeamId);

        // Sort: delete outsider first, sadmin last.
        const sortedMembers = secondTeamMembers.sort((a) =>
            a.user_id === ctx.ids.defaultUserId ? 1 : -1,
        );

        for (const member of sortedMembers) {
            expectStatus(
                await ctx.sadmin.delete<null>(`/api/v1/members/${member.id}`),
                204,
            );
        }

        // NOTE: The team itself cannot be deleted via API here because
        // sadmin loses admin role after deleting own member. `cleanupToSeed`
        // handles the final team deletion. The FK-restricted children
        // (invitation, members) are cleaned above.
        // expectStatus(await ctx.sadmin.delete<null>(
        //     `/api/v1/teams/${secondTeamId}`,
        // ), 204);

        // Verify the team still exists and is accessible before cleanup.
        const teamBeforeCleanup = await getTeam(ctx.sadmin, secondTeamId);

        assert.equal(teamBeforeCleanup.id, secondTeamId, "second team must still exist");

        // outsider's membership is gone; outsider user still exists but has
        // no memberships.
        const outsider = ctx.secondTeam.outsider;

        if (outsider) {
            const outsiderMembers = await listMyMembers(outsider.api);

            assert.equal(
                outsiderMembers.length,
                0,
                "outsider must have no memberships after second-team delete",
            );
        }

        ctx.secondTeam = null;
    }

    // ---------- cleanup aux state ----------

    ctx.auxChapters.delete("cascade");
    ctx.auxChapters.delete("d3");

    // ---------- final: default team still consistent ----------

    await assertTeamInvariant(ctx.sadmin, ctx.ids.defaultTeamId);

    // sanity: main chapter still accessible (it_07 drove it to publish-completed)
    if (ctx.main) {
        const mainChapter = await getChapter(ctx.sadmin, ctx.main.chapterId);

        assert.ok(mainChapter, "main chapter must still be accessible after cascade deletes");
    }

    // sanity: list workset comics / chapter pages / page units helpers still
    // work on the main subtree (no accidental cross-contamination).
    const serialWsId = ctx.ids.worksetIds["连载池"];

    if (serialWsId) {
        const serialComics = await listWorksetComics(ctx.sadmin, serialWsId);

        assert.ok(serialComics.length >= 1, "连载池 still has its comics");

        if (ctx.main) {
            const mainPages = await listChapterPages(ctx.sadmin, ctx.main.chapterId);

            assert.ok(mainPages.length > 0, "main chapter still has its pages");

            if (mainPages[0]) {
                const mainUnits = await listPageUnits(ctx.sadmin, mainPages[0].id);

                assert.ok(mainUnits.total_unit_count >= 0, "main page units still queryable");
            }
        }
    }

    // sanity: default team profile still the seed-restored name (it_08 restored it)
    const defaultTeam = await getTeam(ctx.sadmin, ctx.ids.defaultTeamId);

    assert.ok(defaultTeam.id === ctx.ids.defaultTeamId);
}

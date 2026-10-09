// it_14 — One current review per Chapter, whole-chapter replacement and read-only Page queries.

import * as assert from "@std/assert";
import { withDatabaseClient } from "../db/seed.ts";
import { expectError, expectNoContent, expectSuccessData } from "../http/assertions.ts";
import type { ErrorBody, SuccessBody } from "../http/apiClient.ts";
import { createComic, createWorkset } from "../http/fixtures.ts";
import type { RunCtx } from "../state/runCtx.ts";

export const IMPLEMENTED = true as const;

interface Rect {
    x_coord: number;
    y_coord: number;
    width: number;
    height: number;
}

interface Issue {
    id: string;
    page_id: string;
    index: number;
    variant: string;
    layer_path: string | null;
    rect: Rect | null;
    note: string;
}

interface ImportResult {
    imported_page_count: number;
    imported_issue_count: number;
}

export async function runIt14Module(ctx: RunCtx): Promise<void> {
    const workset = await createWorkset(ctx.sadmin, ctx.ids.defaultTeamId, "chapter review fixtures");
    const comic = await createComic(ctx.sadmin, workset.id, "review", "author", "review chapter");
    const chapterId = comic.chapter_id;
    const pageIds = [crypto.randomUUID(), crypto.randomUUID()];
    const base = `/api/v1/chapters/${chapterId}/issues/import`;
    const guest = ctx.users.get("guest_01")!;
    const rect = { x_coord: 0.1, y_coord: 0.2, width: 0.3, height: 0.1 };
    const body = {
        pages: [{
            issues: [
                { variant: "custom category", layer_path: "opaque/path", rect, note: " first line\nsecond line " },
                { variant: "whole composite", note: "" },
                { variant: "layer without rectangle", layer_path: "0.1.0", rect: null, note: "" },
                { variant: "composite rectangle", layer_path: null, rect, note: "" },
            ],
        }, { issues: [] }],
    };
    const read = async (pageId: string) =>
        expectSuccessData(await ctx.sadmin.get<SuccessBody<Issue[]>>(`/api/v1/pages/${pageId}/issues`), 200);

    await withDatabaseClient(async (client) => {
        for (const [index, pageId] of pageIds.entries()) {
            await client.queryObject("INSERT INTO t_page (f_id, f_chapter_id, f_index) VALUES ($1, $2, $3)", [
                pageId,
                chapterId,
                index,
            ]);
        }
        await client.queryObject("UPDATE t_chapter SET f_page_count = 2 WHERE f_id = $1", [chapterId]);
    });

    // IS1: administrator status alone cannot replace a review; team members can read an empty Page.
    expectError(await ctx.sadmin.post<ErrorBody>(base, body), 403, 4);
    assert.assertEquals(await read(pageIds[0]!), []);
    assert.assertEquals(
        expectSuccessData(await guest.api.get<SuccessBody<Issue[]>>(`/api/v1/pages/${pageIds[1]}/issues`), 200),
        [],
    );
    expectError(await guest.api.post<ErrorBody>(base, body), 403, 4);

    await withDatabaseClient(async (client) => {
        await client.queryObject(
            "UPDATE t_assignment SET f_assigned_reviewer_at = NOW() WHERE f_chapter_id = $1 AND f_user_id = $2",
            [chapterId, ctx.ids.defaultUserId],
        );
    });

    // IS2: Page-position mapping, all optional geometry combinations, open categories and verbatim notes.
    const imported = expectSuccessData(await ctx.sadmin.post<SuccessBody<ImportResult>>(base, body), 200);
    assert.assertEquals(imported, { imported_page_count: 2, imported_issue_count: 4 });
    const first = await read(pageIds[0]!);
    assert.assertEquals(first.map((issue) => issue.index), [0, 1, 2, 3]);
    assert.assertEquals(first[0]!.page_id, pageIds[0]);
    assert.assertEquals(first[0]!.note, " first line\nsecond line ");
    assert.assertEquals(first[0]!.rect, rect);
    assert.assertEquals(first[1]!.layer_path, null);
    assert.assertEquals(first[1]!.rect, null);
    assert.assertEquals(await read(pageIds[1]!), []);

    // IS3: every rejected complete import preserves the current review.
    const invalidBodies = [
        { pages: [] },
        { pages: [{ issues: [] }] },
        { pages: [{ issues: [{ variant: " ", note: "" }] }, { issues: [] }] },
        { pages: [{ issues: [{ variant: "custom", layer_path: " ", note: "" }] }, { issues: [] }] },
        ...[
            { ...rect, x_coord: -0.1 },
            { ...rect, width: 0 },
            { ...rect, height: -1 },
            { ...rect, x_coord: 0.9 },
        ].map((invalidRect) => ({
            pages: [{ issues: [{ variant: "custom", rect: invalidRect, note: "" }] }, { issues: [] }],
        })),
    ];
    for (const invalid of invalidBodies) {
        expectError(await ctx.sadmin.post<ErrorBody>(base, invalid), 422, 2);
        assert.assertEquals(await read(pageIds[0]!), first);
    }

    // IS4: replacement renews IDs and all-empty Page entries clear the entire current review.
    expectSuccessData(await ctx.sadmin.post<SuccessBody<ImportResult>>(base, body), 200);
    const replacement = await read(pageIds[0]!);
    assert.assert(replacement.every((issue) => !first.some((old) => old.id === issue.id)));
    expectSuccessData(
        await ctx.sadmin.post<SuccessBody<ImportResult>>(base, { pages: [{ issues: [] }, { issues: [] }] }),
        200,
    );
    assert.assertEquals(await read(pageIds[0]!), []);
    expectSuccessData(await ctx.sadmin.post<SuccessBody<ImportResult>>(base, body), 200);

    // IS5: new artwork confirmations clear the review, while same-generation retries preserve later imports.
    const allocateArtwork = async (byte: number) =>
        expectSuccessData(
            await ctx.sadmin.post<SuccessBody<{ artwork_version: number }>>(
                `/api/v1/chapters/${chapterId}/artwork/alloc`,
                {
                    artwork_hash: btoa(String.fromCharCode(...new Uint8Array(32).fill(byte))),
                    new_byte_len: 1024,
                    ext: "zip",
                },
            ),
            200,
        );
    const mark = async (version: number) =>
        expectNoContent(
            await ctx.sadmin.post(`/api/v1/chapters/${chapterId}/artwork/mark-uploaded`, {
                artwork_version: version,
            }),
        );
    const artwork = await allocateArtwork(1);
    assert.assertEquals((await read(pageIds[0]!)).length, 4);
    await mark(artwork.artwork_version);
    assert.assertEquals(await read(pageIds[0]!), []);
    expectSuccessData(await ctx.sadmin.post<SuccessBody<ImportResult>>(base, body), 200);
    await mark(artwork.artwork_version);
    assert.assertEquals((await read(pageIds[0]!)).length, 4);
    const newArtwork = await allocateArtwork(2);
    expectError(
        await ctx.sadmin.post<ErrorBody>(`/api/v1/chapters/${chapterId}/artwork/mark-uploaded`, {
            artwork_version: artwork.artwork_version,
        }),
        422,
        2,
    );
    assert.assertEquals((await read(pageIds[0]!)).length, 4);
    await mark(newArtwork.artwork_version);
    assert.assertEquals(await read(pageIds[0]!), []);

    // IS6: published Chapters reject imports; Page deletion cascades stored review details.
    expectSuccessData(await ctx.sadmin.post<SuccessBody<ImportResult>>(base, body), 200);
    await withDatabaseClient(async (client) => {
        await client.queryObject("UPDATE t_chapter SET f_published_at = NOW() WHERE f_id = $1", [chapterId]);
    });
    expectError(await ctx.sadmin.post<ErrorBody>(base, body), 422, 2);
    assert.assertEquals((await read(pageIds[0]!)).length, 4);
    await withDatabaseClient(async (client) => {
        await client.queryObject("DELETE FROM t_page WHERE f_id = $1", [pageIds[0]]);
        const count = await client.queryObject<{ count: bigint }>(
            "SELECT count(*) FROM t_issue WHERE f_page_id = $1",
            [pageIds[0]],
        );
        assert.assertEquals(Number(count.rows[0]!.count), 0);
    });
}

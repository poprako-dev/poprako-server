// it_14 — Composite page identity, independent ordering and whole-chapter review replacement.

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
    page_artwork_id: string;
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

interface Image {
    page_artwork_id?: string;
    raw_ident: string;
    image_hash: string;
    new_byte_len?: number;
    ext: string;
}

interface Allocation {
    page_artwork_id: string;
    index: number;
    image_version: number;
    slot: unknown | null;
}

interface ArtworkPage {
    id: string;
    index: number;
    raw_ident: string | null;
    image_uploaded: boolean;
}

export async function runIt14Module(ctx: RunCtx): Promise<void> {
    const workset = await createWorkset(ctx.sadmin, ctx.ids.defaultTeamId, "chapter review fixtures");
    const comic = await createComic(ctx.sadmin, workset.id, "review", "author", "review chapter");
    const chapterId = comic.chapter_id;
    const sourcePageId = crypto.randomUUID();
    const base = `/api/v1/chapters/${chapterId}/issues/import`;
    const artifacts = `/api/v1/chapters/${chapterId}/page-artworks`;
    const guest = ctx.users.get("guest_01")!;
    const hash = (byte: number) => btoa(String.fromCharCode(...new Uint8Array(32).fill(byte)));
    const image: Image = { raw_ident: "same.psd", image_hash: hash(1), new_byte_len: 1024, ext: "png" };
    const allocate = async (pages: Image[]) =>
        expectSuccessData(
            await ctx.sadmin.post<SuccessBody<{ pages: Allocation[] }>>(`${artifacts}/alloc`, { pages }),
            200,
        ).pages;
    const readPages = async () => expectSuccessData(await ctx.sadmin.get<SuccessBody<ArtworkPage[]>>(artifacts), 200);
    const read = async () =>
        expectSuccessData(await ctx.sadmin.get<SuccessBody<Issue[]>>(`/api/v1/chapters/${chapterId}/issues`), 200);
    const markImage = async (page: Allocation) =>
        expectNoContent(
            await ctx.sadmin.post(`/api/v1/page-artworks/${page.page_artwork_id}/image/mark-uploaded`, {
                image_version: page.image_version,
            }),
        );

    await withDatabaseClient(async (client) => {
        await client.queryObject("INSERT INTO t_page (f_id, f_chapter_id, f_index) VALUES ($1, $2, 0)", [
            sourcePageId,
            chapterId,
        ]);
        await client.queryObject("UPDATE t_chapter SET f_page_count = 1 WHERE f_id = $1", [chapterId]);
    });

    // IS1: administrators maintain composites, but review import requires REVIEWER assignment.
    const pages = await allocate([image, image, image]);
    assert.assertEquals(new Set(pages.map((page) => page.page_artwork_id)).size, 3);
    assert.assertEquals((await readPages()).map((page) => page.image_uploaded), [false, false, false]);
    for (const page of pages) await markImage(page);
    const ids = pages.map((page) => page.page_artwork_id);
    const rect = { x_coord: 0.1, y_coord: 0.2, width: 0.3, height: 0.1 };
    const body = {
        pages: [{
            page_artwork_id: ids[0]!,
            issues: [
                { variant: "custom category", layer_path: "opaque/path", rect, note: " first line\nsecond line " },
                { variant: "whole composite", note: "" },
                { variant: "layer without rectangle", layer_path: "0.1.0", rect: null, note: "" },
                { variant: "composite rectangle", layer_path: null, rect, note: "" },
            ],
        }, { page_artwork_id: ids[2]!, issues: [] }],
    };
    expectError(await ctx.sadmin.post<ErrorBody>(base, body), 403, 4);
    assert.assertEquals(await read(), []);
    assert.assertEquals(
        expectSuccessData(await guest.api.get<SuccessBody<Issue[]>>(`/api/v1/chapters/${chapterId}/issues`), 200),
        [],
    );
    expectError(await guest.api.post<ErrorBody>(base, body), 403, 4);
    expectError(await guest.api.post<ErrorBody>(`${artifacts}/alloc`, { pages: [] }), 403, 4);
    await withDatabaseClient(async (client) => {
        await client.queryObject(
            "UPDATE t_assignment SET f_assigned_reviewer_at = NOW() WHERE f_chapter_id = $1 AND f_user_id = $2",
            [chapterId, ctx.ids.defaultUserId],
        );
    });

    // IS2: explicit composite targets, optional geometry, open categories and verbatim notes.
    assert.assertEquals(expectSuccessData(await ctx.sadmin.post<SuccessBody<ImportResult>>(base, body), 200), {
        imported_page_count: 2,
        imported_issue_count: 4,
    });
    const first = await read();
    assert.assertEquals(first.map((issue) => issue.index), [0, 1, 2, 3]);
    assert.assertEquals(first[0]!.page_artwork_id, ids[0]);
    assert.assertEquals(first[0]!.note, " first line\nsecond line ");
    assert.assertEquals(first[0]!.rect, rect);
    assert.assertEquals(first[1]!.layer_path, null);
    assert.assertEquals(first[1]!.rect, null);

    // IS3: rejected imports preserve the full review, including duplicate or unknown targets.
    const target = body.pages[0]!;
    const invalidBodies = [
        { pages: [{ ...target, issues: [{ variant: " ", note: "" }] }] },
        { pages: [{ ...target, issues: [{ variant: "custom", layer_path: " ", note: "" }] }] },
        { pages: [target, target] },
        { pages: [{ ...target, page_artwork_id: sourcePageId }] },
        ...[
            { ...rect, x_coord: -0.1 },
            { ...rect, width: 0 },
            { ...rect, height: -1 },
            { ...rect, x_coord: 0.9 },
        ].map((invalidRect) => ({
            pages: [{ ...target, issues: [{ variant: "custom", rect: invalidRect, note: "" }] }],
        })),
    ];
    for (const invalid of invalidBodies) {
        expectError(await ctx.sadmin.post<ErrorBody>(base, invalid), 422, 2);
        assert.assertEquals(await read(), first);
    }

    // IS4: whole replacement renews IDs; partial target lists and empty imports are valid.
    expectSuccessData(await ctx.sadmin.post<SuccessBody<ImportResult>>(base, { pages: [target] }), 200);
    assert.assert((await read()).every((issue) => !first.some((old) => old.id === issue.id)));
    expectSuccessData(
        await ctx.sadmin.post<SuccessBody<ImportResult>>(base, { pages: [{ page_artwork_id: ids[2], issues: [] }] }),
        200,
    );
    assert.assertEquals(await read(), []);
    expectSuccessData(await ctx.sadmin.post<SuccessBody<ImportResult>>(base, { pages: [] }), 200);
    assert.assertEquals((await readPages()).length, 3);
    expectSuccessData(await ctx.sadmin.post<SuccessBody<ImportResult>>(base, body), 200);
    const current = await read();

    // IS5: PSD ZIP confirmations and retries preserve the review across generations.
    for (const byte of [1, 2]) {
        const artwork = expectSuccessData(
            await ctx.sadmin.post<SuccessBody<{ artwork_version: number }>>(
                `/api/v1/chapters/${chapterId}/artwork/alloc`,
                { artwork_hash: hash(byte), new_byte_len: 1024, ext: "zip" },
            ),
            200,
        );
        for (const _retry of [0, 1]) {
            expectNoContent(
                await ctx.sadmin.post(`/api/v1/chapters/${chapterId}/artwork/mark-uploaded`, {
                    artwork_version: artwork.artwork_version,
                }),
            );
            assert.assertEquals(await read(), current);
        }
    }

    // IS7: reorder uses explicit IDs; identical hashes and filenames never merge pages.
    const reordered = await allocate([...ids].reverse().map((id) => ({ ...image, page_artwork_id: id })));
    assert.assert(reordered.every((page) => page.slot === null));
    assert.assertEquals((await readPages()).map((page) => page.id), [...ids].reverse());
    assert.assertEquals(await read(), current);
    const replaced = expectSuccessData(
        await ctx.sadmin.post<SuccessBody<Allocation>>(`/api/v1/page-artworks/${ids[0]}/image/alloc`, {
            ...image,
            raw_ident: "renamed.psd",
            image_hash: hash(2),
        }),
        200,
    );
    expectError(
        await ctx.sadmin.post<ErrorBody>(`/api/v1/page-artworks/${ids[0]}/image/mark-uploaded`, {
            image_version: pages[0]!.image_version,
        }),
        422,
        2,
    );
    await markImage(replaced);
    await markImage(replaced);
    assert.assertEquals(await read(), current);

    // IS8: unknown manifest identities reject atomically; removing a composite cascades only its issues.
    expectError(
        await ctx.sadmin.post<ErrorBody>(`${artifacts}/alloc`, {
            pages: [{ ...image, page_artwork_id: sourcePageId }],
        }),
        422,
        2,
    );
    assert.assertEquals(await read(), current);
    await allocate([{ ...image, page_artwork_id: ids[2] }]);
    assert.assertEquals(await read(), []);
    await allocate([]);
    assert.assertEquals(await readPages(), []);

    // IS6: source deletion is independent; published chapters reject all composite and review writes.
    const finalPage = (await allocate([image]))[0]!;
    const finalReview = { pages: [{ ...target, page_artwork_id: finalPage.page_artwork_id }] };
    expectSuccessData(await ctx.sadmin.post<SuccessBody<ImportResult>>(base, finalReview), 200);
    const finalIssues = await read();
    await withDatabaseClient(async (client) => {
        await client.queryObject("DELETE FROM t_page WHERE f_id = $1", [sourcePageId]);
        await client.queryObject("UPDATE t_chapter SET f_published_at = NOW() WHERE f_id = $1", [chapterId]);
    });
    expectError(await ctx.sadmin.post<ErrorBody>(base, finalReview), 422, 2);
    expectError(await ctx.sadmin.post<ErrorBody>(`${artifacts}/alloc`, { pages: [] }), 422, 2);
    expectError(
        await ctx.sadmin.post<ErrorBody>(`/api/v1/page-artworks/${finalPage.page_artwork_id}/image/alloc`, {
            ...image,
        }),
        422,
        2,
    );
    expectError(
        await ctx.sadmin.post<ErrorBody>(`/api/v1/page-artworks/${finalPage.page_artwork_id}/image/mark-uploaded`, {
            image_version: finalPage.image_version,
        }),
        422,
        2,
    );
    assert.assertEquals(await read(), finalIssues);
}

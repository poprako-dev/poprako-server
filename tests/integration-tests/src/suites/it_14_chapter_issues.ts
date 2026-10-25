// it_14 — Composite page identity, independent ordering and whole-chapter review replacement.

import { verifyReviewEditor } from "./review_editor.ts";
import assert from "node:assert/strict";
import { withDatabaseClient } from "../db/seed.ts";
import { expectError, expectNoContent, expectSuccessData } from "../http/assertions.ts";
import type { ErrorBody, SuccessBody } from "../http/apiClient.ts";
import { createComic, createWorkset, joinChapterAssignment } from "../http/fixtures.ts";
import { ROLE } from "../state/roles.ts";
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

interface ArtworkPageSource {
    id: string;
    index: number;
    raw_ident: string | null;
    image_uploaded: boolean;
    image_url?: string;
    image_thumbnail_url?: string;
}

export async function runIt14Module(ctx: RunCtx): Promise<void> {
    const workset = await createWorkset(ctx.sadmin, ctx.ids.defaultTeamId, "chapter review fixtures");
    const comic = await createComic(ctx.sadmin, workset.id, "review", "author", "review chapter");
    const chapterId = comic.chapter_id;
    const sourcePageSourceId = crypto.randomUUID();
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
    const readPageSources = async () =>
        expectSuccessData(await ctx.sadmin.get<SuccessBody<ArtworkPageSource[]>>(artifacts), 200);
    const read = async () =>
        expectSuccessData(await ctx.sadmin.get<SuccessBody<Issue[]>>(`/api/v1/chapters/${chapterId}/issues`), 200);
    const markImage = async (page: Allocation) =>
        expectNoContent(
            await ctx.sadmin.post(`/api/v1/page-artworks/${page.page_artwork_id}/image/mark-uploaded`, {
                image_version: page.image_version,
            }),
        );

    await withDatabaseClient(async (client) => {
        await client.query("INSERT INTO t_page_source (f_id, f_chapter_id, f_index) VALUES ($1, $2, 0)", [
            sourcePageSourceId,
            chapterId,
        ]);
        await client.query("UPDATE t_chapter SET f_page_source_count = 1 WHERE f_id = $1", [chapterId]);
    });

    // IS1: administrators maintain composites, but review import requires REVIEWER assignment.
    const pages = await allocate([image, image, image]);
    assert.deepEqual(new Set(pages.map((page) => page.page_artwork_id)).size, 3);
    assert.deepEqual(
        (await readPageSources()).map((page) => page.image_uploaded),
        [false, false, false],
    );
    for (const page of pages) await markImage(page);
    const ids = pages.map((page) => page.page_artwork_id);
    const rect = { x_coord: 0.1, y_coord: 0.2, width: 0.3, height: 0.1 };
    const body = {
        pages: [
            {
                page_artwork_id: ids[0]!,
                issues: [
                    {
                        variant: "\u2003custom category\u2003",
                        rect,
                        note: " first line\nsecond line ",
                    },
                    { variant: "whole composite", note: "" },
                    { variant: "whole-page note", rect: null, note: "" },
                    { variant: "composite rectangle", rect, note: "" },
                ],
            },
            { page_artwork_id: ids[2]!, issues: [] },
        ],
    };
    expectError(await ctx.sadmin.post<ErrorBody>(base, body), 403, 4);
    assert.deepEqual(await read(), []);
    assert.deepEqual(
        expectSuccessData(await guest.api.get<SuccessBody<Issue[]>>(`/api/v1/chapters/${chapterId}/issues`), 200),
        [],
    );
    expectError(await guest.api.post<ErrorBody>(base, body), 403, 4);
    expectError(await guest.api.post<ErrorBody>(`${artifacts}/alloc`, { pages: [] }), 403, 4);
    await joinChapterAssignment(ctx.sadmin, chapterId, ROLE.REVIEWER);

    // IS2: explicit composite targets, optional geometry, open categories and verbatim notes.
    assert.deepEqual(expectSuccessData(await ctx.sadmin.post<SuccessBody<ImportResult>>(base, body), 200), {
        imported_page_count: 2,
        imported_issue_count: 4,
    });
    const first = await read();
    assert.deepEqual(
        first.map((issue) => issue.index),
        [0, 1, 2, 3],
    );
    assert.deepEqual(first[0]!.page_artwork_id, ids[0]);
    assert.deepEqual(first[0]!.variant, "custom category");
    assert.deepEqual(first[0]!.note, " first line\nsecond line ");
    assert.ok(!Object.hasOwn(first[0]!, "layer_name"));
    assert.deepEqual(first[0]!.rect, rect);
    assert.deepEqual(first[1]!.rect, null);

    const retiredField = await ctx.sadmin.post(base, {
        pages: [{ page_artwork_id: ids[0]!, issues: [{ variant: "custom", layer_name: "retired", note: "" }] }],
    });
    assert.equal(retiredField.status, 422);
    assert.deepEqual(await read(), first);

    // IS3: rejected imports preserve the full review, including duplicate or unknown targets.
    const target = body.pages[0]!;
    const invalidBodies = [
        { pages: [{ ...target, issues: [{ variant: " ", note: "" }] }] },
        { pages: [target, target] },
        { pages: [{ ...target, page_artwork_id: sourcePageSourceId }] },
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
        assert.deepEqual(await read(), first);
    }

    // IS4: whole replacement renews IDs; partial target lists and empty imports are valid.
    expectSuccessData(await ctx.sadmin.post<SuccessBody<ImportResult>>(base, { pages: [target] }), 200);
    assert.ok((await read()).every((issue) => !first.some((old) => old.id === issue.id)));
    expectSuccessData(
        await ctx.sadmin.post<SuccessBody<ImportResult>>(base, { pages: [{ page_artwork_id: ids[2], issues: [] }] }),
        200,
    );
    assert.deepEqual(await read(), []);
    expectSuccessData(await ctx.sadmin.post<SuccessBody<ImportResult>>(base, { pages: [] }), 200);
    assert.deepEqual((await readPageSources()).length, 3);
    expectSuccessData(await ctx.sadmin.post<SuccessBody<ImportResult>>(base, body), 200);
    await verifyReviewEditor(ctx.sadmin, chapterId, ids[0]!);
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
        for (let retry = 0; retry < 2; retry++) {
            expectNoContent(
                await ctx.sadmin.post(`/api/v1/chapters/${chapterId}/artwork/mark-uploaded`, {
                    artwork_version: artwork.artwork_version,
                }),
            );
            assert.deepEqual(await read(), current);
        }
    }

    // IS7: explicit IDs preserve identity; uploaded composites expose only raw and thumbnail URLs.
    const reordered = await allocate([...ids].reverse().map((id) => ({ ...image, page_artwork_id: id })));
    assert.ok(reordered.every((page) => page.slot === null));
    const reorderedPageSources = await readPageSources();
    assert.deepEqual(
        reorderedPageSources.map((page) => page.id),
        [...ids].reverse(),
    );
    for (const page of reorderedPageSources) {
        assert.deepEqual(typeof page.image_url, "string");
        assert.deepEqual(typeof page.image_thumbnail_url, "string");
        assert.ok(!Object.hasOwn(page, "image_optimized_url"));
    }
    assert.deepEqual(await read(), current);
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
    assert.deepEqual(await read(), current);

    // IS8: unknown manifest identities reject atomically; removing a composite cascades only its issues.
    expectError(
        await ctx.sadmin.post<ErrorBody>(`${artifacts}/alloc`, {
            pages: [{ ...image, page_artwork_id: sourcePageSourceId }],
        }),
        422,
        2,
    );
    assert.deepEqual(await read(), current);
    await allocate([{ ...image, page_artwork_id: ids[2] }]);
    assert.deepEqual(await read(), []);
    await allocate([]);
    assert.deepEqual(await readPageSources(), []);

    // IS6: source deletion is independent; published chapters reject all composite and review writes.
    const finalPageSource = (await allocate([image]))[0]!;
    const finalReview = { pages: [{ ...target, page_artwork_id: finalPageSource.page_artwork_id }] };
    expectSuccessData(await ctx.sadmin.post<SuccessBody<ImportResult>>(base, finalReview), 200);
    const finalIssues = await read();
    await withDatabaseClient(async (client) => {
        await client.query("DELETE FROM t_page_source WHERE f_id = $1", [sourcePageSourceId]);
        await client.query("UPDATE t_chapter SET f_published_at = NOW() WHERE f_id = $1", [chapterId]);
    });
    expectError(await ctx.sadmin.post<ErrorBody>(base, finalReview), 422, 2);
    expectError(await ctx.sadmin.post<ErrorBody>(`${artifacts}/alloc`, { pages: [] }), 422, 2);
    expectError(
        await ctx.sadmin.post<ErrorBody>(`/api/v1/page-artworks/${finalPageSource.page_artwork_id}/image/alloc`, {
            ...image,
        }),
        422,
        2,
    );
    expectError(
        await ctx.sadmin.post<ErrorBody>(
            `/api/v1/page-artworks/${finalPageSource.page_artwork_id}/image/mark-uploaded`,
            {
                image_version: finalPageSource.image_version,
            },
        ),
        422,
        2,
    );
    assert.deepEqual(await read(), finalIssues);
}

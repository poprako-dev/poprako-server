// it_11 — Immutable comic archive endpoint.
//
// Preconditions:
//   - it_00..it_10 have run and the default super-admin remains active.
//   - ctx.users has 14 non-admin member personas from it_01.
//
// Postconditions:
//   - An independent archive workset remains active for final cleanup.
//   - The archived comic subtree is absent from active tables and represented
//     by one compressed immutable archive row.
//
// Covers archive creation, permission rejection (non-admin member),
// repeated-archive failure, child-resource inaccessibility, audit fields,
// outbox delete records, active-data cleanup, and stable workset comic counts.

import assert from "node:assert/strict";

import { grantChapterWorkerRoles, withDatabaseClient } from "../db/seed.js";
import { expectError } from "../http/assertions.js";
import type { ErrorBody } from "../http/apiClient.js";
import {
    archiveComic,
    createChapter,
    createComic,
    createWorkset,
    getWorkset,
    listWorksetComics,
    newPageManifest,
    reserveChapterPages,
    reserveComicCover,
} from "../http/fixtures.js";
import type { RunCtx } from "../state/runCtx.js";

export const IMPLEMENTED = true as const;

interface ActiveImageKeys {
    cover_key: string | null;
    page_key: string | null;
}

interface ArchiveAuditRow {
    f_archiver_id: string;
    f_created_at: Date;
}

export async function runIt11Module(ctx: RunCtx): Promise<void> {
    const workset = await createWorkset(
        ctx.sadmin,
        ctx.ids.defaultTeamId,
        "archive endpoint fixture",
    );
    const comic = await createComic(
        ctx.sadmin,
        workset.id,
        "archive endpoint comic",
        "archive endpoint author",
        "archive endpoint chapter",
    );

    // Create a second chapter so the archive snapshot is multi-chapter.
    const chapter2 = await createChapter(ctx.sadmin, comic.id, "second chapter");

    await grantChapterWorkerRoles(comic.chapter_id, ctx.ids.defaultUserId);
    await grantChapterWorkerRoles(chapter2.id, ctx.ids.defaultUserId);

    // Reserve one page in each chapter.
    await reserveChapterPages(ctx.sadmin, comic.chapter_id, newPageManifest(1, "png"));
    await reserveChapterPages(ctx.sadmin, chapter2.id, newPageManifest(1, "png"));

    await reserveComicCover(ctx.sadmin, comic.id, "png");

    // ---------- permission: non-admin member cannot archive ----------

    const guest = ctx.users.get("guest_01");

    assert.ok(guest, "guest_01 must be registered by it_01");

    expectError(
        await guest.api.post<ErrorBody>(`/api/v1/comics/${comic.id}/archive`),
        403,
        4,
    );

    // ---------- snapshot pre-archive state ----------

    const workset_before_archive = await getWorkset(ctx.sadmin, workset.id);
    const active_image_keys = await withDatabaseClient(async (client) => {
        const image_result = await client.query<ActiveImageKeys>(
            `
              SELECT
                comic."f_cover_key" AS cover_key,
                page."f_image_key" AS page_key
              FROM "t_comic" comic
              JOIN "t_page" page ON page."f_chapter_id" = $1
              WHERE comic."f_id" = $2
              ORDER BY page."f_image_key"
              LIMIT 1
            `,
            [comic.chapter_id, comic.id],
        );

        return image_result.rows[0]!;
    });

    assert.ok(active_image_keys.cover_key, "archive fixture must retain a reserved cover key");
    assert.ok(active_image_keys.page_key, "archive fixture must retain a reserved page key");

    // ---------- archive the comic ----------

    const archive_comic_val = await archiveComic(ctx.sadmin, comic.id);

    assert.notEqual(archive_comic_val.archived_comic_id, comic.id);

    // ---------- active-comic and its chapters are inaccessible ----------

    expectError(await ctx.sadmin.get<ErrorBody>(`/api/v1/comics/${comic.id}`), 422, 2);
    expectError(await ctx.sadmin.get<ErrorBody>(`/api/v1/chapters/${comic.chapter_id}`), 422, 2);
    expectError(await ctx.sadmin.get<ErrorBody>(`/api/v1/chapters/${chapter2.id}`), 422, 2);

    // ---------- child resources are inaccessible ----------

    expectError(
        await ctx.sadmin.get<ErrorBody>(`/api/v1/comics/${comic.id}/chapters?offset=0&limit=20`),
        422,
        2,
    );
    expectError(
        await ctx.sadmin.get<ErrorBody>(`/api/v1/chapters/${comic.chapter_id}/pages?offset=0&limit=20`),
        422,
        2,
    );
    expectError(
        await ctx.sadmin.get<ErrorBody>(`/api/v1/chapters/${chapter2.id}/pages?offset=0&limit=20`),
        422,
        2,
    );

    // ---------- repeated archive on the same comic fails ----------

    expectError(
        await ctx.sadmin.post<ErrorBody>(`/api/v1/comics/${comic.id}/archive`),
        422,
        2,
    );

    // ---------- workset comic count unchanged (archive does not decrement) ----------

    const archived_workset = await getWorkset(ctx.sadmin, workset.id);
    const active_comics = await listWorksetComics(ctx.sadmin, workset.id);

    assert.equal(archived_workset.comic_count, workset_before_archive.comic_count);
    assert.ok(!active_comics.some((active_comic) => active_comic.id === comic.id));

    // ---------- archive audit rows and outbox ----------

    const archive_rows = await withDatabaseClient(async (client) => {
        const [comic_rows, delete_rows] = await Promise.all([
            client.query<ArchiveAuditRow>(
                `SELECT "f_archiver_id", "f_created_at" FROM "t_comic_archive" WHERE "f_id" = $1`,
                [archive_comic_val.archived_comic_id],
            ),
            client.query<{ object_key: string }>(
                `
                  SELECT "f_payload"->'Image'->'Delete'->>'object_key' AS object_key
                  FROM "t_local_message"
                  WHERE "f_topic" = 'image'
                    AND "f_payload" ? 'Image'
                    AND "f_payload"->'Image' ? 'Delete'
                `,
            ),
        ]);

        return { comic_rows, delete_rows };
    });

    assert.equal(archive_rows.comic_rows.rows.length, 1);
    assert.equal(archive_rows.comic_rows.rows[0]!.f_archiver_id, ctx.ids.defaultUserId);
    assert.ok(Number.isFinite(archive_rows.comic_rows.rows[0]!.f_created_at.getTime()));

    // Image delete keys in outbox cover both the reserved cover and all page images.
    const delete_keys = archive_rows.delete_rows.rows
        .map((delete_row) => delete_row.object_key)
        .filter((object_key): object_key is string => object_key !== null);

    assert.ok(delete_keys.includes(active_image_keys.cover_key));
    assert.ok(delete_keys.includes(active_image_keys.page_key));

    // All reserved page images across both chapters must have delete entries.
    // Two pages total → at least 2 page-key delete entries.
    const page_delete_keys = delete_keys.filter(
        (object_key) => object_key.startsWith("page/chapter_"),
    );

    assert.ok(page_delete_keys.length >= 2, "all reserved page images must have delete entries");
}

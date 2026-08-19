// it_05 — Unit save: basic creation, counts, export, import/export.
//
// Covers F1/F2 creation and linked ordering plus F10 import/export.

import assert from "node:assert/strict";

import { grantChapterWorkerRoles } from "../db/seed.js";
import { expectError, expectStatus } from "../http/assertions.js";
import type { ErrorBody } from "../http/apiClient.js";
import {
    assertPageExportInvariant,
    assertPageUnitInvariant,
} from "../http/invariants.js";
import {
    createChapter,
    exportLabelPlus,
    exportPoprako,
    getChapter,
    importTranslations,
    newBubbleUnit,
    newPageManifest,
    reserveChapterPages,
    savePageUnits,
} from "../http/fixtures.js";
import { titled } from "../state/prefix.js";
import type { RunCtx } from "../state/runCtx.js";

export const IMPLEMENTED = true as const;

export async function runIt05Module(ctx: RunCtx): Promise<void> {
    assert.ok(ctx.main, "it_02 must have set ctx.main");
    assert.ok(ctx.main.pageIds.length >= 3, "it_03 must have reserved at least 3 pages on main");

    const mainChapterId = ctx.main.chapterId;
    const p0Id = ctx.main.pageIds[0]!;
    const trans01 = ctx.users.get("trans_01")!;

    // ---------- F1. create 5 bubble units on p0 ----------

    const f1Edits = Array.from({ length: 5 }, (_, i) =>
        newBubbleUnit(`p0_lu_0${i + 1}`, 0.1 * (i + 1), 0.1 * (i + 1)),
    );

    const f1Save = await savePageUnits(trans01.api, p0Id, f1Edits);

    assert.equal(f1Save.total_unit_count, 5);
    assert.equal(f1Save.translated_unit_count, 0);
    assert.equal(f1Save.proofread_unit_count, 0);

    const mainAfterF1 = await getChapter(ctx.sadmin, mainChapterId);

    assert.equal(mainAfterF1.total_unit_count, 5);

    const exportAfterF1 = await exportPoprako(ctx.sadmin, mainChapterId);
    const p0ExportAfterF1 = exportAfterF1.pages.find((p) => p.page_id === p0Id);

    assert.ok(p0ExportAfterF1, "export must include p0");
    assert.equal(p0ExportAfterF1!.units.length, 5);

    await assertPageUnitInvariant(ctx.sadmin, p0Id);
    await assertPageExportInvariant(ctx.sadmin, mainChapterId, p0Id);

    // Capture unit IDs in export order for F2.
    const p0UnitIds = f1Save.unit_infos.map((unit) => unit.id);

    // ---------- F2. next_id insert ----------

    const anchoredEdit = {
        edit: "create" as const,
        local_id: "p0_lu_insert_before_02",
        next_id: p0UnitIds[1],
        is_bubble: true,
        coord: {
            x_coord: 0.15,
            y_coord: 0.15,
        },
    };

    const f2Save = await savePageUnits(trans01.api, p0Id, [anchoredEdit]);

    assert.equal(f2Save.total_unit_count, 6, "total 5 -> 6 after next_id insert");

    const exportAfterF2 = await exportPoprako(ctx.sadmin, mainChapterId);
    const p0ExportAfterF2 = exportAfterF2.pages.find((p) => p.page_id === p0Id)!;

    const f2Order = [...p0ExportAfterF2.units]
        .sort((a, b) => a.unit_index - b.unit_index)
        .map((u) => u.unit_id);

    // New unit inserted before the 2nd unit: [u0, inserted, u1, u2, u3, u4]
    assert.equal(f2Order.length, 6);
    assert.equal(f2Order[0], p0UnitIds[0], "u0 stays first");
    assert.equal(f2Order[1], f2Save.unit_infos[1]!.id, "inserted unit before u1");
    assert.equal(f2Order[2], p0UnitIds[1], "u1 shifted to index 2");

    await assertPageExportInvariant(ctx.sadmin, mainChapterId, p0Id);

    // ---------- F10. import/export regression ----------

    const mainExport = await exportPoprako(ctx.sadmin, mainChapterId);

    assert.ok(mainExport.chapter_id);
    assert.ok(mainExport.comic_id);
    assert.ok(mainExport.pages.length > 0);

    const mainLabelPlus = await exportLabelPlus(ctx.sadmin, mainChapterId);

    assert.ok(mainLabelPlus.length > 0, "label-plus export must be non-empty text");

    const gangtieId = ctx.ids.comicIds["钢铁魔女"]!;
    const f10Chapter = await createChapter(ctx.sadmin, gangtieId, titled("第 7 话 F10导入"));

    await grantChapterWorkerRoles(f10Chapter.id, ctx.ids.defaultUserId);

    const f10Reserve = await reserveChapterPages(
        ctx.sadmin,
        f10Chapter.id,
        newPageManifest(mainExport.pages.length, "jpg"),
    );

    assert.equal(f10Reserve.pages.length, mainExport.pages.length);

    // JSON body and query enum values use snake_case.
    expectStatus(
        await ctx.sadmin.post<ErrorBody>(`/api/v1/chapters/${f10Chapter.id}/translations/import`, {
            content: "garbage-content",
            format: "label-plus",
        }),
        422,
    );

    // Valid format with invalid content -> 422/2.
    expectError(
        await ctx.sadmin.post<ErrorBody>(`/api/v1/chapters/${f10Chapter.id}/translations/import`, {
            content: "garbage-content",
            format: "label_plus",
        }),
        422,
        2,
    );

    const imported = await importTranslations(
        ctx.sadmin,
        f10Chapter.id,
        "poprako",
        JSON.stringify(mainExport),
    );

    assert.equal(imported.imported_page_count, mainExport.pages.length);
    assert.equal(
        imported.imported_unit_count,
        mainExport.pages.reduce((count, page) => count + page.units.length, 0),
    );

    const importedExport = await exportPoprako(ctx.sadmin, f10Chapter.id);

    assert.equal(importedExport.pages.length, mainExport.pages.length);

    mainExport.pages.forEach((sourcePage, pageIndex) => {
        const targetPage = importedExport.pages[pageIndex]!;
        const sourceUnits = [...sourcePage.units].sort((a, b) => a.unit_index - b.unit_index);
        const targetUnits = [...targetPage.units].sort((a, b) => a.unit_index - b.unit_index);

        assert.equal(targetUnits.length, sourceUnits.length);

        sourceUnits.forEach((sourceUnit, unitIndex) => {
            const targetUnit = targetUnits[unitIndex]!;

            assert.equal(targetUnit.unit_index, sourceUnit.unit_index);
            assert.equal(targetUnit.x_coord, sourceUnit.x_coord);
            assert.equal(targetUnit.y_coord, sourceUnit.y_coord);
            assert.equal(targetUnit.is_bubble, sourceUnit.is_bubble);
            assert.equal(targetUnit.translated_text, sourceUnit.translated_text);
            assert.equal(targetUnit.is_proofread, sourceUnit.is_proofread);
            assert.equal(targetUnit.proofread_text, sourceUnit.proofread_text);
            assert.notEqual(targetUnit.unit_id, sourceUnit.unit_id);
        });
    });

    await importTranslations(
        ctx.sadmin,
        f10Chapter.id,
        "poprako",
        JSON.stringify(mainExport),
    );

    const repeatedExport = await exportPoprako(ctx.sadmin, f10Chapter.id);
    assert.deepEqual(
        repeatedExport.pages.map((page) => page.units.length),
        importedExport.pages.map((page) => page.units.length),
    );

    // cleanup F10 aux chapter
    expectStatus(await ctx.sadmin.delete<null>(`/api/v1/chapters/${f10Chapter.id}`), 204);
}

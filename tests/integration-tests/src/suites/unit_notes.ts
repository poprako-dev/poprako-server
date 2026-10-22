// F1.notes — Independent annotations, progress, and translation exchange.
import assert from "node:assert/strict";

import {
    createChapter,
    exportTranslations,
    getChapter,
    importTranslations,
    joinChapterAssignment,
    listPageUnits,
    newBubbleUnit,
    newPageManifest,
    reserveChapterPages,
    savePageUnits,
} from "../http/fixtures.ts";
import { assertPageUnitInvariant } from "../http/invariants.ts";
import { titled } from "../state/prefix.ts";
import { ROLE } from "../state/roles.ts";
import type { RunCtx } from "../state/runCtx.ts";
import { PHASE, stagePhase } from "../state/stages.ts";

export async function verifyUnitNotes(ctx: RunCtx): Promise<void> {
    const translator = ctx.users.get("trans_01")!.api;
    const proofreader = ctx.users.get("proof_02")!.api;
    const chapter = await createChapter(ctx.sadmin, ctx.main!.comicId, titled("unit annotations"));

    await joinChapterAssignment(ctx.sadmin, chapter.id, ROLE.RAW_PROVIDER);
    await joinChapterAssignment(translator, chapter.id, ROLE.TRANSLATOR);
    await joinChapterAssignment(proofreader, chapter.id, ROLE.PROOFREADER);

    const allocation = await reserveChapterPages(ctx.sadmin, chapter.id, newPageManifest(1, "png"));
    const pageId = allocation.pages[0]!.page_id;
    const initial = newBubbleUnit("note", 0.1, 0.2);

    initial.translation_note = "uncertain phrase";
    initial.typography_note = " \t\u3000";

    const created = await savePageUnits(translator, pageId, [initial]);
    const id = created.unit_infos[0]!.id;

    assert.equal(created.translated_unit_count, 0);
    assert.equal(created.unit_infos[0]!.typography_note, undefined);
    assert.equal(stagePhase((await getChapter(ctx.sadmin, chapter.id)).stages, "translate"), PHASE.PENDING);

    const note = " small text\n\nkeep parentheses ";
    const saved = await savePageUnits(proofreader, pageId, [
        { edit: "patch", id, typography_note: { type: "assign", value: note } },
        { edit: "patch", id, translation_note: { type: "assign", value: "checked internally" } },
    ]);

    assert.equal(saved.translated_unit_count, 1);
    assert.equal(saved.proofread_unit_count, 0);
    assert.equal(saved.unit_infos[0]!.typography_note, note);
    assert.equal(saved.unit_infos[0]!.translated_text, undefined);
    assert.equal(saved.unit_infos[0]!.last_translator_id, undefined);
    assert.equal(saved.unit_infos[0]!.last_proofreader_id, undefined);
    assert.equal(stagePhase((await getChapter(ctx.sadmin, chapter.id)).stages, "translate"), PHASE.ACTIVE);
    assert.equal(stagePhase((await getChapter(ctx.sadmin, chapter.id)).stages, "proofread"), PHASE.PENDING);

    const skipped = await savePageUnits(translator, pageId, [
        { edit: "patch", id, typography_note: null, translation_note: { type: "skip" } },
    ]);

    assert.equal(skipped.unit_infos[0]!.typography_note, note);
    assert.equal(skipped.unit_infos[0]!.updated_at, saved.unit_infos[0]!.updated_at);

    await savePageUnits(translator, pageId, [{ edit: "delete", id }]);
    const restored = await savePageUnits(proofreader, pageId, [{ edit: "patch", id }]);

    assert.equal(restored.translated_unit_count, 1);
    assert.equal(restored.unit_infos[0]!.typography_note, note);

    const exported = await exportTranslations(translator, chapter.id, ["poprako", "label_plus"]);
    const native = exported.poprako!;
    const nativeUnit = native.pages[0]!.units[0]!;

    assert.equal(nativeUnit.typography_note, note);
    assert.equal(nativeUnit.translation_note, "checked internally");
    assert.ok(exported.label_plus!.includes(`###${note}###`));
    assert.ok(!exported.label_plus!.includes("checked internally"));

    await importTranslations(proofreader, chapter.id, "label_plus", "overwrite", exported.label_plus!);
    const lpImported = await listPageUnits(translator, pageId);

    assert.equal(lpImported.unit_infos[0]!.typography_note, note);
    assert.equal(lpImported.unit_infos[0]!.translation_note, undefined);
    assert.equal(lpImported.unit_infos[0]!.proofread_text, undefined);
    assert.equal(lpImported.translated_unit_count, 1);
    assert.equal(lpImported.proofread_unit_count, 0);

    await importTranslations(translator, chapter.id, "poprako", "overwrite", JSON.stringify(native));
    const nativeImported = await listPageUnits(proofreader, pageId);
    const importedId = nativeImported.unit_infos[0]!.id;

    assert.equal(nativeImported.unit_infos[0]!.typography_note, note);
    assert.equal(nativeImported.unit_infos[0]!.translation_note, "checked internally");

    const cleared = await savePageUnits(proofreader, pageId, [
        { edit: "patch", id: importedId, typography_note: { type: "assign", value: "\n\t\u3000" } },
        { edit: "patch", id: importedId, translation_note: { type: "clear" } },
    ]);

    assert.equal(cleared.translated_unit_count, 0);
    assert.equal(cleared.unit_infos[0]!.typography_note, undefined);
    assert.equal(cleared.unit_infos[0]!.translation_note, undefined);

    delete nativeUnit.typography_note;
    delete nativeUnit.translation_note;
    await importTranslations(translator, chapter.id, "poprako", "overwrite", JSON.stringify(native));

    const legacy = await listPageUnits(translator, pageId);

    assert.equal(legacy.unit_infos[0]!.typography_note, undefined);
    assert.equal(legacy.unit_infos[0]!.translation_note, undefined);
    assert.equal(legacy.translated_unit_count, 0);

    await assertPageUnitInvariant(translator, pageId);
}

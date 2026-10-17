// F1 token coverage: concurrent process-local retries return identical ID mappings.
import assert from "node:assert/strict";

import type { ApiClient, ErrorBody, SuccessBody } from "../http/apiClient.ts";
import { expectSuccessData } from "../http/assertions.ts";
import { listPageUnits } from "../http/fixtures.ts";
import type { UnitEdit } from "../http/fixtures.ts";
import type { ListPageUnitInfosVal, SavePageUnitEditsVal } from "../http/types.ts";

export async function saveWithConcurrentReplay(
    api: ApiClient,
    pageId: string,
    edits: UnitEdit[],
): Promise<ListPageUnitInfosVal> {
    const saveId = crypto.randomUUID();
    const path = `/api/v1/pages/${pageId}/units/save?save_id=${saveId}`;
    const [first, simultaneous] = await Promise.all([
        api.post<SuccessBody<SavePageUnitEditsVal> | ErrorBody>(path, edits),
        api.post<SuccessBody<SavePageUnitEditsVal> | ErrorBody>(path, edits),
    ]);

    const originalMapping = expectSuccessData<SavePageUnitEditsVal>(first, 200);
    const simultaneousMapping = expectSuccessData<SavePageUnitEditsVal>(simultaneous, 200);

    assert.deepEqual(simultaneousMapping, originalMapping);
    assert.equal(originalMapping.created_unit_ids.length, edits.filter((edit) => edit.edit === "create").length);

    const original = await listPageUnits(api, pageId);

    const replay = expectSuccessData<SavePageUnitEditsVal>(
        await api.post<SuccessBody<SavePageUnitEditsVal> | ErrorBody>(path, edits),
        200,
    );

    assert.deepEqual(replay, originalMapping);

    const changed = [...edits].reverse();

    assert.notDeepEqual(changed, edits, "fixture must change the typed payload order");
    const changedReplay = expectSuccessData<SavePageUnitEditsVal>(
        await api.post<SuccessBody<SavePageUnitEditsVal> | ErrorBody>(path, changed),
        200,
    );

    assert.deepEqual(changedReplay, originalMapping);

    const listed = await listPageUnits(api, pageId);

    assert.deepEqual(listed, original, "a completed token must skip every repeated batch");

    return listed;
}

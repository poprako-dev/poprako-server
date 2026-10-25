// F1 token coverage: concurrent process-local retries return identical ID mappings.
import assert from "node:assert/strict";

import type { ApiClient, ErrorBody, SuccessBody } from "../http/apiClient.ts";
import { expectSuccessData } from "../http/assertions.ts";
import { listPageSourceUnits } from "../http/fixtures.ts";
import type { UnitEdit } from "../http/fixtures.ts";
import type { ListPageSourceUnitInfosVal, SavePageSourceUnitEditsVal } from "../http/types.ts";

export async function saveWithConcurrentReplay(
    api: ApiClient,
    pageId: string,
    edits: UnitEdit[],
): Promise<ListPageSourceUnitInfosVal> {
    const saveId = crypto.randomUUID();
    const path = `/api/v1/page-sources/${pageId}/units/save?save_id=${saveId}`;
    const [first, simultaneous] = await Promise.all([
        api.post<SuccessBody<SavePageSourceUnitEditsVal> | ErrorBody>(path, edits),
        api.post<SuccessBody<SavePageSourceUnitEditsVal> | ErrorBody>(path, edits),
    ]);

    const originalMapping = expectSuccessData<SavePageSourceUnitEditsVal>(first, 200);
    const simultaneousMapping = expectSuccessData<SavePageSourceUnitEditsVal>(simultaneous, 200);

    assert.deepEqual(simultaneousMapping, originalMapping);
    assert.equal(originalMapping.created_unit_ids.length, edits.filter((edit) => edit.edit === "create").length);

    const original = await listPageSourceUnits(api, pageId);

    const replay = expectSuccessData<SavePageSourceUnitEditsVal>(
        await api.post<SuccessBody<SavePageSourceUnitEditsVal> | ErrorBody>(path, edits),
        200,
    );

    assert.deepEqual(replay, originalMapping);

    const changed = [...edits].reverse();

    assert.notDeepEqual(changed, edits, "fixture must change the typed payload order");
    const changedReplay = expectSuccessData<SavePageSourceUnitEditsVal>(
        await api.post<SuccessBody<SavePageSourceUnitEditsVal> | ErrorBody>(path, changed),
        200,
    );

    assert.deepEqual(changedReplay, originalMapping);

    const listed = await listPageSourceUnits(api, pageId);

    assert.deepEqual(listed, original, "a completed token must skip every repeated batch");

    return listed;
}

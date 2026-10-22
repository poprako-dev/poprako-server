// it_12 append coverage runs against an existing writable team or comic base.
// The helper deletes its added entries so the caller retains its original fixtures.
import assert from "node:assert/strict";

import type { ApiClient, ErrorBody, SuccessBody } from "../http/apiClient.ts";
import { expectError, expectNoContent, expectSuccessData, expectSuccessList } from "../http/assertions.ts";
import type { AppendTermbaseInstr, AppendTermbaseVal, TermbaseInfoView, TermInfoView } from "../http/types.ts";

export async function verifyTermbaseAppend(api: ApiClient, reader: ApiClient, termbaseId: string): Promise<void> {
    const path = `/api/v1/termbases/${termbaseId}`;
    const original = expectSuccessData<TermbaseInfoView>(await api.get(path), 200);
    const first: AppendTermbaseInstr = {
        terms: [
            { source: "Append Source", targets: ["Preferred", "Second"], comment: "first\nsecond" },
            { source: "Append Stable", targets: ["Stable"] },
        ],
    };

    assert.deepEqual(expectSuccessData(await api.post<SuccessBody<AppendTermbaseVal>>(`${path}/append`, first), 200), {
        added: 2,
        updated: 0,
        unchanged: 0,
    });

    const before = expectSuccessList<TermInfoView>(await api.get(`${path}/terms?offset=0&limit=200`), 200);
    const source = before.find((term) => term.source === "Append Source");
    const stable = before.find((term) => term.source === "Append Stable");

    assert.ok(source);
    assert.ok(stable);

    const batch: AppendTermbaseInstr = {
        terms: [
            { source: " append source ", targets: ["second", "Third"], comment: "second" },
            { source: "APPEND STABLE", targets: ["stable"], comment: null },
            { source: "Append Added", targets: ["Added"] },
        ],
    };

    assert.deepEqual(expectSuccessData(await api.post<SuccessBody<AppendTermbaseVal>>(`${path}/append`, batch), 200), {
        added: 1,
        updated: 1,
        unchanged: 1,
    });

    const after = expectSuccessList<TermInfoView>(await api.get(`${path}/terms?offset=0&limit=200`), 200);
    const merged = after.find((term) => term.id === source.id);

    assert.ok(merged);
    assert.equal(merged.source, source.source);
    assert.deepEqual(merged.targets, ["Preferred", "Second", "Third"]);
    assert.equal(merged.comment, "first\nsecond\nsecond");
    assert.equal(merged.creator_id, source.creator_id);
    assert.equal(merged.created_at, source.created_at);
    assert.deepEqual(
        after.find((term) => term.id === stable.id),
        stable,
    );

    const base = expectSuccessData<TermbaseInfoView>(await api.get(path), 200);

    assert.equal(base.name, original.name);
    assert.equal(base.description, original.description);
    assert.equal(base.term_count, original.term_count + 3);

    for (const terms of [[], [{ source: "APPEND SOURCE", targets: ["preferred"], comment: merged.comment }]]) {
        assert.deepEqual(
            expectSuccessData(await api.post<SuccessBody<AppendTermbaseVal>>(`${path}/append`, { terms }), 200),
            {
                added: 0,
                updated: 0,
                unchanged: terms.length,
            },
        );
        assert.deepEqual(expectSuccessData(await api.get(path), 200), base);
    }

    for (const body of [
        { name: "Forbidden metadata", terms: [] },
        { terms: [{ source: "Foreign", targets: ["T"], id: source.id }] },
    ]) {
        const response = await api.post<ErrorBody>(`${path}/append`, body);

        assert.equal(response.status, 422);
    }

    for (const terms of [
        [
            { source: "Duplicate", targets: ["A"] },
            { source: " duplicate ", targets: ["B"] },
        ],
        [
            { source: "Valid", targets: ["A"] },
            { source: " ", targets: ["B"] },
        ],
        Array.from({ length: 201 }, (_, index) => ({ source: `Over capacity ${index}`, targets: ["T"] })),
    ]) {
        expectError(await api.post<ErrorBody>(`${path}/append`, { terms }), 422, 2);
        assert.deepEqual(expectSuccessData(await api.get(path), 200), base);
    }

    expectError(await reader.post<ErrorBody>(`${path}/append`, { terms: [] }), 403, 4);
    expectError(await api.post<ErrorBody>("/api/v1/termbases/missing-append-base/append", { terms: [] }), 422, 2);

    const final = expectSuccessList<TermInfoView>(await api.get(`${path}/terms?offset=0&limit=200`), 200);

    assert.deepEqual(final, after);

    for (const term of after.filter((term) => term.source.startsWith("Append "))) {
        expectNoContent(await api.delete<null>(`/api/v1/terms/${term.id}`));
    }
}

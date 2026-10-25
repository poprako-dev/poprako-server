// IS9–IS11: page CAS, replay and atomic chapter category rename over HTTP.
import assert from "node:assert/strict";
import type { ApiClient, SuccessBody } from "../http/apiClient.ts";
import { expectSuccessData } from "../http/assertions.ts";

type IssuePage = {
    review_key: string;
    revision: string;
    image_version: number | null;
    issues: { id: string; variant: string; note: string; rect: null }[];
};
type SaveResult = { page: IssuePage; created_issue_ids: { local_id: string; issue_id: string }[] };

type Summary = { review_key: string; revision: string; variants: { variant: string; count: number }[] };

export async function verifyReviewEditor(api: ApiClient, chapterId: string, pageId: string): Promise<void> {
    const path = `/api/v1/page-artworks/${pageId}/issues`;
    const baseline = expectSuccessData(await api.get<SuccessBody<IssuePage>>(path), 200);
    const batch = crypto.randomUUID();
    const issue = { local_id: crypto.randomUUID(), variant: "editor category", rect: null, note: "" };
    const input = {
        review_key: baseline.review_key,
        revision: baseline.revision,
        image_version: baseline.image_version,
        issues: [issue],
    };
    const saved = expectSuccessData(
        await api.post<SuccessBody<SaveResult>>(`${path}/save?save_id=${batch}`, input),
        200,
    );
    assert.equal(saved.created_issue_ids[0]?.local_id, issue.local_id);
    assert.match(saved.created_issue_ids[0]!.issue_id, /^issue--[0-9A-Za-z]{11}$/);
    assert.equal(saved.page.issues[0]?.id, saved.created_issue_ids[0]?.issue_id);
    assert.notEqual(saved.page.revision, baseline.revision);
    assert.ok(!Object.hasOwn(saved.page.issues[0]!, "layer_name"));
    const replay = expectSuccessData(
        await api.post<SuccessBody<SaveResult>>(`${path}/save?save_id=${batch}`, input),
        200,
    );
    assert.equal(replay.page.revision, saved.page.revision);
    assert.deepEqual(replay.created_issue_ids, saved.created_issue_ids);
    const stale = await api.post(`${path}/save?save_id=${crypto.randomUUID()}`, input);
    assert.equal(stale.status, 409);
    await verifyRename(api, chapterId, pageId);
}

async function verifyRename(api: ApiClient, chapterId: string, pageId: string): Promise<void> {
    const path = `/api/v1/chapters/${chapterId}/issues/variants`;
    const before = expectSuccessData(await api.get<SuccessBody<Summary>>(path), 200);
    assert.equal(before.variants.find((item) => item.variant === "editor category")?.count, 1);
    const renamed = expectSuccessData(
        await api.post<SuccessBody<{ summary: Summary; pages: IssuePage[] }>>(`${path}/rename`, {
            review_key: before.review_key,
            revision: before.revision,
            from: "editor category",
            to: "renamed category",
        }),
        200,
    );
    assert.equal(renamed.pages.length, 1);
    assert.equal(renamed.summary.variants.find((item) => item.variant === "renamed category")?.count, 1);
    const page = expectSuccessData(
        await api.get<SuccessBody<IssuePage>>(`/api/v1/page-artworks/${pageId}/issues`),
        200,
    );
    assert.equal(page.issues[0]?.variant, "renamed category");
}

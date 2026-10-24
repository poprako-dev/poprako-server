// IS9–IS11: page CAS, replay and atomic chapter category rename over HTTP.
import assert from "node:assert/strict";
import type { ApiClient, SuccessBody } from "../http/apiClient.ts";
import { expectSuccessData } from "../http/assertions.ts";

type Page = {
    review_key: string;
    revision: string;
    image_version: number | null;
    issues: { id: string; variant: string; note: string; rect: null }[];
};
type Summary = { review_key: string; revision: string; variants: { variant: string; count: number }[] };

export async function verifyReviewEditor(api: ApiClient, chapterId: string, pageId: string): Promise<void> {
    const path = `/api/v1/page-artworks/${pageId}/issues`;
    const baseline = expectSuccessData(await api.get<SuccessBody<Page>>(path), 200);
    const batch = crypto.randomUUID();
    const issue = { id: crypto.randomUUID(), variant: "editor category", rect: null, note: "" };
    const input = {
        review_key: baseline.review_key,
        revision: baseline.revision,
        image_version: baseline.image_version,
        issues: [issue],
    };
    const saved = expectSuccessData(await api.post<SuccessBody<Page>>(`${path}/save?save_id=${batch}`, input), 200);
    assert.equal(saved.issues[0]?.id, issue.id);
    assert.notEqual(saved.revision, baseline.revision);
    assert.ok(!Object.hasOwn(saved.issues[0]!, "layer_name"));
    const replay = expectSuccessData(await api.post<SuccessBody<Page>>(`${path}/save?save_id=${batch}`, input), 200);
    assert.equal(replay.revision, saved.revision);
    const stale = await api.post(`${path}/save?save_id=${crypto.randomUUID()}`, input);
    assert.equal(stale.status, 409);
    await verifyRename(api, chapterId, pageId);
}

async function verifyRename(api: ApiClient, chapterId: string, pageId: string): Promise<void> {
    const path = `/api/v1/chapters/${chapterId}/issues/variants`;
    const before = expectSuccessData(await api.get<SuccessBody<Summary>>(path), 200);
    assert.equal(before.variants.find((item) => item.variant === "editor category")?.count, 1);
    const renamed = expectSuccessData(
        await api.post<SuccessBody<{ summary: Summary; pages: Page[] }>>(`${path}/rename`, {
            review_key: before.review_key,
            revision: before.revision,
            from: "editor category",
            to: "renamed category",
        }),
        200,
    );
    assert.equal(renamed.pages.length, 1);
    assert.equal(renamed.summary.variants.find((item) => item.variant === "renamed category")?.count, 1);
    const page = expectSuccessData(await api.get<SuccessBody<Page>>(`/api/v1/page-artworks/${pageId}/issues`), 200);
    assert.equal(page.issues[0]?.variant, "renamed category");
}

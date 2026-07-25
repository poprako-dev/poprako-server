import assert from "node:assert/strict";
import { createHash } from "node:crypto";

import { testEnv } from "../config/env.js";
import {
    expectNoContent,
    expectSuccessData,
    expectSuccessList,
} from "./assertions.js";
import { ApiClient, clientFor } from "./apiClient.js";
import type { SuccessBody } from "./apiClient.js";
import type {
    AnnouncementInfoVal,
    ArchiveComicVal,
    AssignmentInfoVal,
    AssignmentInvitationInfoVal,
    ChapterInfoVal,
    CodeVal,
    ComicInfoVal,
    ListComicInfosPayload,
    CommentInfoVal,
    CreateComicVal,
    IdVal,
    ListPageUnitInfosVal,
    LoginVal,
    MemberInfoVal,
    MemberInvitationInfoVal,
    ImageExtension,
    PageInfoVal,
    PageImageInput,
    PoprakoExportVal,
    ReserveChapterPagesVal,
    ReservedPageVal,
    ReserveImagePayload,
    SavePageUnitsVal,
    SystemMailInfoVal,
    TeamInfoVal,
    UnitInfoVal,
    UserInfoVal,
    WorksetInfoVal,
} from "./types.js";
import type { StageName, StageOper } from "../state/stages.js";

// ---------- timestamp / invariant helpers ----------

// Assert a value is a Unix-millisecond integer (the API's timestamp format).
export function assertTimestampMs(value: unknown): asserts value is number {
    assert.ok(typeof value === "number", "timestamp must be a number");
    assert.ok(Number.isInteger(value), "timestamp must be an integer");
}

// Assert `created_at <= updated_at` for a record carrying both fields.
export function assertCreatedBeforeUpdated(created: number, updated: number): void {
    assert.ok(
        created <= updated,
        `created_at (${created}) must be <= updated_at (${updated})`,
    );
}

// ---------- auth ----------

export async function login(
    api: ApiClient,
    qid: string,
    password: string,
): Promise<LoginVal> {
    const response = await api.post<SuccessBody<LoginVal>>("/api/v1/auth/login", {
        password,
        qid,
    });

    const val = expectSuccessData(response, 200);

    assert.ok(val.token.length > 20, "login token must be > 20 chars");

    api.setToken(val.token);

    return val;
}

// Register an invitee via a fresh client and return the authenticated client
// plus the new user id. The caller's `api` is NOT mutated.
export async function registerInvitee(
    code: string,
    qidValue: string,
    nicknameValue: string,
    passwordValue: string,
): Promise<{ api: ApiClient; userId: string; token: string }> {
    const fresh = new ApiClient(testEnv.apiBaseUrl);

    const response = await fresh.post<SuccessBody<LoginVal>>("/api/v1/auth/register", {
        code,
        nickname: nicknameValue,
        password: passwordValue,
        qid: qidValue,
    });

    const val = expectSuccessData(response, 201);

    assert.ok(val.token.length > 20, "register token must be > 20 chars");

    fresh.setToken(val.token);

    return { api: fresh, userId: val.user_id, token: val.token };
}

// Build an authenticated client for an already-registered user.
export async function userClient(qidValue: string, passwordValue: string): Promise<{
    api: ApiClient;
    userId: string;
    token: string;
}> {
    const fresh = new ApiClient(testEnv.apiBaseUrl);

    const { user_id, token } = await login(fresh, qidValue, passwordValue);

    return { api: fresh, userId: user_id, token };
}

export async function logout(api: ApiClient): Promise<void> {
    expectNoContent(await api.post<null>("/api/v1/auth/logout"));
    api.clearToken();
}

// ---------- user ----------

export async function getMyInfo(api: ApiClient): Promise<UserInfoVal> {
    return expectSuccessData(await api.get("/api/v1/users/me"), 200);
}

export async function getUserInfo(api: ApiClient, userId: string): Promise<UserInfoVal> {
    return expectSuccessData(await api.get(`/api/v1/users/${userId}`), 200);
}

// ---------- team ----------

export async function createTeam(
    api: ApiClient,
    name: string,
    description: string,
): Promise<TeamInfoVal> {
    return expectSuccessData(
        await api.post<SuccessBody<TeamInfoVal>>("/api/v1/teams", {
            description,
            name,
        }),
        201,
    );
}

export async function getTeam(api: ApiClient, teamId: string): Promise<TeamInfoVal> {
    return expectSuccessData(await api.get(`/api/v1/teams/${teamId}`), 200);
}

export async function listTeams(api: ApiClient, userId?: string): Promise<TeamInfoVal[]> {
    const query = userId
        ? `?user_id=${encodeURIComponent(userId)}&offset=0&limit=50`
        : "?offset=0&limit=50";

    return expectSuccessList(await api.get<SuccessBody<TeamInfoVal[]>>(`/api/v1/teams${query}`), 200);
}

export async function updateTeam(
    api: ApiClient,
    teamId: string,
    name: string,
    description: string,
): Promise<void> {
    expectNoContent(
        await api.put<null>(`/api/v1/teams/${teamId}`, {
            description,
            id: teamId,
            name,
        }),
    );
}

export async function reserveTeamAvatar(
    api: ApiClient,
    teamId: string,
    ext: ImageExtension,
): Promise<ReserveImagePayload> {
    return reserveAndUploadImage(
        api,
        `/api/v1/teams/${teamId}/avatar/reserve`,
        `poprako-team-avatar-${teamId}-${ext}`,
        ext,
    );
}

async function reserveAndUploadImage(
    api: ApiClient,
    path: string,
    content: string,
    ext: ImageExtension,
): Promise<ReserveImagePayload> {
    const imageBytes = new TextEncoder().encode(content);

    const imageHash = createHash("sha256").update(imageBytes).digest("base64");

    const reserved = expectSuccessData(
        await api.post<SuccessBody<ReserveImagePayload>>(path, {
            image_hash: imageHash,
            new_byte_len: imageBytes.byteLength,
            ext,
        }),
        200,
    );

    if (reserved.slot) {
        const response = await fetch(reserved.slot.put_url, {
            method: "PUT",
            headers: reserved.slot.headers,
            body: imageBytes,
        });

        assert.ok(response.ok, `image upload failed with status ${response.status}`);
    }

    return reserved;
}

export async function markTeamAvatarUploaded(
    api: ApiClient,
    teamId: string,
    avatarVersion: number,
): Promise<void> {
    expectNoContent(
        await api.post<null>(`/api/v1/teams/${teamId}/avatar/mark-uploaded`, {
            image_version: avatarVersion,
        }),
    );
}

// ---------- user avatar ----------

export async function reserveUserAvatar(
    api: ApiClient,
    userId: string,
    ext: ImageExtension,
): Promise<ReserveImagePayload> {
    return reserveAndUploadImage(
        api,
        `/api/v1/users/${userId}/avatar/reserve`,
        `poprako-user-avatar-${userId}-${ext}`,
        ext,
    );
}

export async function markUserAvatarUploaded(
    api: ApiClient,
    userId: string,
    avatarVersion: number,
): Promise<void> {
    expectNoContent(
        await api.post<null>(`/api/v1/users/${userId}/avatar/mark-uploaded`, {
            image_version: avatarVersion,
        }),
    );
}

// ---------- member ----------

export async function listMyMembers(api: ApiClient, incl = ""): Promise<MemberInfoVal[]> {
    return expectSuccessList(
        await api.get<SuccessBody<MemberInfoVal[]>>(`/api/v1/members/me?offset=0&limit=50${incl}`),
        200,
    );
}

export async function listTeamMembers(
    api: ApiClient,
    teamId: string,
    extraQuery = "",
): Promise<MemberInfoVal[]> {
    const query = `?team_id=${encodeURIComponent(teamId)}&offset=0&limit=50${extraQuery}`;

    return expectSuccessList(
        await api.get<SuccessBody<MemberInfoVal[]>>(`/api/v1/members${query}`),
        200,
    );
}

export async function updateMemberRoles(
    api: ApiClient,
    memberId: string,
    roles: number,
): Promise<void> {
    expectNoContent(
        await api.put<null>(`/api/v1/members/${memberId}/roles`, {
            id: memberId,
            roles,
        }),
    );
}

// ---------- member invitation ----------

export async function createMemberInvitation(
    api: ApiClient,
    teamId: string,
    inviteeQid: string,
    roles: number,
): Promise<CodeVal> {
    return expectSuccessData(
        await api.post<SuccessBody<CodeVal>>("/api/v1/member-invitations", {
            invitee_qid: inviteeQid,
            roles,
            team_id: teamId,
        }),
        201,
    );
}

export async function listMemberInvitations(
    api: ApiClient,
    teamId: string,
    pending: boolean,
): Promise<MemberInvitationInfoVal[]> {
    const query = `?pending=${pending}&offset=0&limit=100`;

    return expectSuccessList(
        await api.get<SuccessBody<MemberInvitationInfoVal[]>>(
            `/api/v1/teams/${teamId}/member-invitations${query}`,
        ),
        200,
    );
}

export async function updateMemberInvitationRoles(
    api: ApiClient,
    invitationId: string,
    roles: number,
): Promise<void> {
    expectNoContent(
        await api.put<null>(`/api/v1/member-invitations/${invitationId}/roles`, {
            id: invitationId,
            roles,
        }),
    );
}

export async function deleteMemberInvitation(api: ApiClient, invitationId: string): Promise<void> {
    expectNoContent(await api.delete<null>(`/api/v1/member-invitations/${invitationId}`));
}

// ---------- workset ----------

export async function createWorkset(
    api: ApiClient,
    teamId: string,
    name: string,
    description?: string,
): Promise<IdVal> {
    return expectSuccessData(
        await api.post<SuccessBody<IdVal>>("/api/v1/worksets", {
            description: description ?? null,
            name,
            team_id: teamId,
        }),
        201,
    );
}

export async function getWorkset(api: ApiClient, worksetId: string): Promise<WorksetInfoVal> {
    return expectSuccessData(await api.get(`/api/v1/worksets/${worksetId}`), 200);
}

export async function listTeamWorksets(api: ApiClient, teamId: string): Promise<WorksetInfoVal[]> {
    return expectSuccessList(
        await api.get<SuccessBody<WorksetInfoVal[]>>(`/api/v1/teams/${teamId}/worksets?offset=0&limit=100`),
        200,
    );
}

export async function updateWorkset(
    api: ApiClient,
    worksetId: string,
    name: string,
    description?: string,
): Promise<void> {
    expectNoContent(
        await api.put<null>(`/api/v1/worksets/${worksetId}`, {
            description: description ?? null,
            id: worksetId,
            name,
        }),
    );
}

export async function deleteWorkset(api: ApiClient, worksetId: string): Promise<void> {
    expectNoContent(await api.delete<null>(`/api/v1/worksets/${worksetId}`));
}

// ---------- comic ----------

export async function createComic(
    api: ApiClient,
    worksetId: string,
    title: string,
    author: string,
    firstChapterSubtitle?: string,
): Promise<CreateComicVal> {
    return expectSuccessData(
        await api.post<SuccessBody<CreateComicVal>>("/api/v1/comics", {
            author,
            description: null,
            first_chapter_subtitle: firstChapterSubtitle ?? null,
            title,
            workset_id: worksetId,
        }),
        201,
    );
}

export async function getComic(api: ApiClient, comicId: string): Promise<ComicInfoVal> {
    return expectSuccessData(await api.get(`/api/v1/comics/${comicId}`), 200);
}

export async function archiveComic(api: ApiClient, comicId: string): Promise<ArchiveComicVal> {
    return expectSuccessData(
        await api.post<SuccessBody<ArchiveComicVal>>(`/api/v1/comics/${comicId}/archive`),
        201,
    );
}

export async function listWorksetComics(
    api: ApiClient,
    worksetId: string,
    extraQuery = "",
): Promise<ComicInfoVal[]> {
    const payload = await listWorksetComicInfos(api, worksetId, extraQuery);

    return payload.comics;
}

export async function listWorksetComicInfos(
    api: ApiClient,
    worksetId: string,
    extraQuery = "",
): Promise<ListComicInfosPayload> {
    const query = `?offset=0&limit=100${extraQuery}`;

    return expectSuccessData(
        await api.get<SuccessBody<ListComicInfosPayload>>(`/api/v1/worksets/${worksetId}/comics${query}`),
        200,
    );
}

export async function updateComic(
    api: ApiClient,
    comicId: string,
    title: string,
    author: string,
    description?: string,
): Promise<void> {
    expectNoContent(
        await api.put<null>(`/api/v1/comics/${comicId}`, {
            author,
            description: description ?? null,
            id: comicId,
            title,
        }),
    );
}

export async function reserveComicCover(
    api: ApiClient,
    comicId: string,
    ext: ImageExtension,
): Promise<ReserveImagePayload> {
    return reserveAndUploadImage(
        api,
        `/api/v1/comics/${comicId}/cover/reserve`,
        `poprako-comic-cover-${comicId}-${ext}`,
        ext,
    );
}

export async function markComicCoverUploaded(
    api: ApiClient,
    comicId: string,
    coverVersion: number,
): Promise<void> {
    expectNoContent(
        await api.post<null>(`/api/v1/comics/${comicId}/cover/mark-uploaded`, {
            image_version: coverVersion,
        }),
    );
}

// ---------- chapter ----------

export async function createChapter(
    api: ApiClient,
    comicId: string,
    subtitle?: string,
): Promise<IdVal> {
    return expectSuccessData(
        await api.post<SuccessBody<IdVal>>("/api/v1/chapters", {
            comic_id: comicId,
            subtitle: subtitle ?? null,
        }),
        201,
    );
}

export async function getChapter(api: ApiClient, chapterId: string): Promise<ChapterInfoVal> {
    return expectSuccessData(await api.get(`/api/v1/chapters/${chapterId}`), 200);
}

export async function listComicChapters(
    api: ApiClient,
    comicId: string,
    extraQuery = "",
): Promise<ChapterInfoVal[]> {
    const query = `?offset=0&limit=100${extraQuery}`;

    return expectSuccessList(
        await api.get<SuccessBody<ChapterInfoVal[]>>(`/api/v1/comics/${comicId}/chapters${query}`),
        200,
    );
}

export async function getPinnedChapter(
    api: ApiClient,
    comicId: string,
): Promise<ChapterInfoVal | null> {
    const data = expectSuccessData(
        await api.get<SuccessBody<ChapterInfoVal | null>>(`/api/v1/comics/${comicId}/chapters/pinned`),
        200,
    );

    return data;
}

export async function patchChapter(
    api: ApiClient,
    chapterId: string,
    patch: { subtitle?: string; pin?: boolean },
): Promise<void> {
    const body: Record<string, unknown> = { id: chapterId };

    if (patch.subtitle !== undefined) {
        body.subtitle = patch.subtitle;
    }

    if (patch.pin !== undefined) {
        body.pin = patch.pin;
    }

    expectNoContent(await api.patch<null>(`/api/v1/chapters/${chapterId}`, body));
}

export async function advanceStage(
    api: ApiClient,
    chapterId: string,
    stage: StageName,
): Promise<void> {
    expectNoContent(
        await api.post<null>(`/api/v1/chapters/${chapterId}/stage/advance`, {
            id: chapterId,
            oper: "advance" as StageOper,
            stage,
        }),
    );
}

export async function revertStage(
    api: ApiClient,
    chapterId: string,
    stage: StageName,
): Promise<void> {
    expectNoContent(
        await api.post<null>(`/api/v1/chapters/${chapterId}/stage/advance`, {
            id: chapterId,
            oper: "revert" as StageOper,
            stage,
        }),
    );
}

// ---------- page ----------

const TEST_PAGE_BYTES = new TextEncoder().encode("poprako-page-integration");

const TEST_PAGE_HASH = createHash("sha256").update(TEST_PAGE_BYTES).digest("base64");

export function newPageManifest(pageCount: number, ext: ImageExtension): PageImageInput[] {
    return Array.from({ length: pageCount }, () => ({
        page_id: null,
        image_hash: TEST_PAGE_HASH,
        new_byte_len: TEST_PAGE_BYTES.byteLength,
        ext,
    }));
}

async function uploadReservedPages(reservedPages: ReservedPageVal[]): Promise<void> {
    for (const reservedPage of reservedPages) {
        if (!reservedPage.slot) continue;

        const response = await fetch(reservedPage.slot.put_url, {
            method: "PUT",
            headers: reservedPage.slot.headers,
            body: TEST_PAGE_BYTES,
        });

        assert.ok(response.ok, `page upload failed with status ${response.status}`);
    }
}

export async function reserveChapterPages(
    api: ApiClient,
    chapterId: string,
    pages: PageImageInput[],
): Promise<ReserveChapterPagesVal> {
    const reserved = expectSuccessData(
        await api.post<SuccessBody<ReserveChapterPagesVal>>(
            `/api/v1/chapters/${chapterId}/pages/reserve`,
            {
                chapter_id: chapterId,
                pages,
            },
        ),
        200,
    );

    await uploadReservedPages(reserved.pages);

    return reserved;
}

export async function listChapterPages(api: ApiClient, chapterId: string): Promise<PageInfoVal[]> {
    return expectSuccessList(
        await api.get<SuccessBody<PageInfoVal[]>>(`/api/v1/chapters/${chapterId}/pages?offset=0&limit=100`),
        200,
    );
}

export async function reservePageImage(
    api: ApiClient,
    pageId: string,
    ext: ImageExtension,
): Promise<ReservedPageVal> {
    const imageBytes = new TextEncoder().encode(`poprako-page-replacement-${pageId}-${ext}`);

    const imageHash = createHash("sha256").update(imageBytes).digest("base64");

    const reserved = expectSuccessData(
        await api.post<SuccessBody<ReservedPageVal>>(`/api/v1/pages/${pageId}/image/reserve`, {
            image_hash: imageHash,
            new_byte_len: imageBytes.byteLength,
            ext,
        }),
        200,
    );

    if (reserved.slot) {
        const response = await fetch(reserved.slot.put_url, {
            method: "PUT",
            headers: reserved.slot.headers,
            body: imageBytes,
        });

        assert.ok(response.ok, `page upload failed with status ${response.status}`);
    }

    return reserved;
}

export async function markPageImageUploaded(
    api: ApiClient,
    pageId: string,
    imageVersion: number,
): Promise<void> {
    expectNoContent(
        await api.post<null>(`/api/v1/pages/${pageId}/image/mark-uploaded`, {
            image_version: imageVersion,
        }),
    );
}

export async function deleteChapterPages(api: ApiClient, chapterId: string): Promise<void> {
    expectNoContent(await api.delete<null>(`/api/v1/chapters/${chapterId}/pages`));
}

// ---------- unit ----------

export interface UnitCreateOper {
    oper: "create";
    local_id: string;
    before_id?: string | null;
    is_bubble: boolean;
    is_proofread: boolean;
    x_coord: number;
    y_coord: number;
    translated_text?: string | null;
    last_translator_id?: string | null;
    proofread_text?: string | null;
    last_proofreader_id?: string | null;
}

export interface UnitSaveOper {
    oper: "save";
    id: string;
    before_id?: string | null;
    is_bubble: boolean;
    is_proofread: boolean;
    x_coord: number;
    y_coord: number;
    translated_text?: string | null;
    last_translator_id?: string | null;
    proofread_text?: string | null;
    last_proofreader_id?: string | null;
}

export interface UnitDeleteOper {
    oper: "delete";
    id: string;
}

export type UnitOper = UnitCreateOper | UnitSaveOper | UnitDeleteOper;

// Build a save oper for a brand-new bubble unit with no translation yet.
export function newBubbleUnit(
    localId: string,
    xCoord: number,
    yCoord: number,
): UnitCreateOper {
    return {
        oper: "create",
        local_id: localId,
        before_id: null,
        is_bubble: true,
        is_proofread: false,
        x_coord: xCoord,
        y_coord: yCoord,
        translated_text: null,
        last_translator_id: null,
        proofread_text: null,
        last_proofreader_id: null,
    };
}

// Build a save oper that updates an existing unit (by server id) without
// changing its position.
export function updateUnit(unitId: string, patch: Partial<Omit<UnitSaveOper, "oper" | "id">>): UnitSaveOper {
    return {
        oper: "save",
        id: unitId,
        is_bubble: patch.is_bubble ?? true,
        is_proofread: patch.is_proofread ?? false,
        x_coord: patch.x_coord ?? 0,
        y_coord: patch.y_coord ?? 0,
        translated_text: patch.translated_text ?? null,
        last_translator_id: patch.last_translator_id ?? null,
        proofread_text: patch.proofread_text ?? null,
        last_proofreader_id: patch.last_proofreader_id ?? null,
    };
}

export function deleteUnit(unitId: string): UnitDeleteOper {
    return { oper: "delete", id: unitId };
}

export async function savePageUnits(
    api: ApiClient,
    pageId: string,
    opers: UnitOper[],
): Promise<SavePageUnitsVal> {
    return expectSuccessData(
        await api.post<SuccessBody<SavePageUnitsVal>>(`/api/v1/pages/${pageId}/units/save`, {
            diff: {
                opers,
                page_id: pageId,
            },
            page_id: pageId,
        }),
        200,
    );
}

export async function listPageUnits(api: ApiClient, pageId: string): Promise<ListPageUnitInfosVal> {
    return expectSuccessData(
        await api.get<SuccessBody<ListPageUnitInfosVal>>(`/api/v1/pages/${pageId}/units?offset=0&limit=100`),
        200,
    );
}

// ---------- assignment ----------

export async function listChapterAssignments(
    api: ApiClient,
    chapterId: string,
    extraQuery = "",
): Promise<AssignmentInfoVal[]> {
    const query = `?chapter_id=${encodeURIComponent(chapterId)}&offset=0&limit=100${extraQuery}`;

    return expectSuccessList(
        await api.get<SuccessBody<AssignmentInfoVal[]>>(`/api/v1/assignments${query}`),
        200,
    );
}

export async function listOwnerAssignments(
    api: ApiClient,
    ownerId: string,
    extraQuery = "",
): Promise<AssignmentInfoVal[]> {
    const query = `?owner_id=${encodeURIComponent(ownerId)}&offset=0&limit=100${extraQuery}`;

    return expectSuccessList(
        await api.get<SuccessBody<AssignmentInfoVal[]>>(`/api/v1/assignments${query}`),
        200,
    );
}

export async function joinChapterAssignment(
    api: ApiClient,
    chapterId: string,
    roles: number,
): Promise<AssignmentInfoVal> {
    return expectSuccessData(
        await api.post<SuccessBody<AssignmentInfoVal>>("/api/v1/assignments/join", {
            chapter_id: chapterId,
            roles,
        }),
        201,
    );
}

export async function updateAssignmentRoles(
    api: ApiClient,
    chapterId: string,
    userId: string,
    roles: number,
): Promise<void> {
    expectNoContent(
        await api.put<null>(`/api/v1/chapters/${chapterId}/assignments/${userId}/roles`, {
            chapter_id: chapterId,
            roles,
            user_id: userId,
        }),
    );
}

export async function deleteAssignment(api: ApiClient, assignmentId: string): Promise<void> {
    expectNoContent(await api.delete<null>(`/api/v1/assignments/${assignmentId}`));
}

// ---------- assignment invitation ----------

export async function createAssignmentInvitation(
    api: ApiClient,
    chapterId: string,
    inviteeQid: string,
    roles: number,
): Promise<CodeVal> {
    return expectSuccessData(
        await api.post<SuccessBody<CodeVal>>("/api/v1/assignment-invitations", {
            chapter_id: chapterId,
            invitee_qid: inviteeQid,
            roles,
        }),
        201,
    );
}

export async function listChapterAssignmentInvitations(
    api: ApiClient,
    chapterId: string,
    pending: boolean,
): Promise<AssignmentInvitationInfoVal[]> {
    const query = `?pending=${pending}&offset=0&limit=100`;

    return expectSuccessList(
        await api.get<SuccessBody<AssignmentInvitationInfoVal[]>>(
            `/api/v1/chapters/${chapterId}/assignment-invitations${query}`,
        ),
        200,
    );
}

export async function joinAssignmentInvitation(
    api: ApiClient,
    code: string,
): Promise<AssignmentInfoVal> {
    return expectSuccessData(
        await api.post<SuccessBody<AssignmentInfoVal>>("/api/v1/assignment-invitations/join", {
            code,
        }),
        201,
    );
}

export async function deleteAssignmentInvitation(
    api: ApiClient,
    invitationId: string,
): Promise<void> {
    expectNoContent(await api.delete<null>(`/api/v1/assignment-invitations/${invitationId}`));
}

// ---------- system mail ----------

export async function listSystemMails(
    api: ApiClient,
    extraQuery = "",
): Promise<SystemMailInfoVal[]> {
    return expectSuccessList(
        await api.get<SuccessBody<SystemMailInfoVal[]>>(`/api/v1/system-mails?offset=0&limit=100${extraQuery}`),
        200,
    );
}

export async function markSystemMailsRead(api: ApiClient, ids: string[]): Promise<void> {
    expectNoContent(
        await api.post<null>("/api/v1/system-mails/mark-read", { ids }),
    );
}

// Poll `listSystemMails` until `predicate(mailCount)` is true, or `timeoutMs`
// elapses. The mail effect runs in a background tokio task, so mails are NOT
// guaranteed to be visible immediately after the workflow op returns 204.
// Throws if the predicate never holds within the timeout.
export async function waitForMails(
    api: ApiClient,
    predicate: (count: number) => boolean,
    timeoutMs = 2000,
): Promise<number> {
    const start = Date.now();

    while (true) {
        const mails = await listSystemMails(api);

        if (predicate(mails.length)) {
            return mails.length;
        }

        if (Date.now() - start >= timeoutMs) {
            throw new Error(
                `waitForMails timed out after ${timeoutMs}ms; last count=${mails.length}`,
            );
        }

        await sleep(50);
    }
}

function sleep(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
}

// ---------- announcement / comment ----------

export async function createAnnouncement(
    api: ApiClient,
    teamId: string,
    title: string,
    content: string,
): Promise<IdVal> {
    return expectSuccessData(
        await api.post<SuccessBody<IdVal>>("/api/v1/announcements", {
            content,
            team_id: teamId,
            title,
        }),
        201,
    );
}

export async function listTeamAnnouncements(
    api: ApiClient,
    teamId: string,
): Promise<AnnouncementInfoVal[]> {
    return expectSuccessList(
        await api.get<SuccessBody<AnnouncementInfoVal[]>>(`/api/v1/teams/${teamId}/announcements?offset=0&limit=100`),
        200,
    );
}

// Paged variant for pagination assertions.
export async function listTeamAnnouncementsPaged(
    api: ApiClient,
    teamId: string,
    offset: number,
    limit: number,
): Promise<AnnouncementInfoVal[]> {
    return expectSuccessList(
        await api.get<SuccessBody<AnnouncementInfoVal[]>>(`/api/v1/teams/${teamId}/announcements?offset=${offset}&limit=${limit}`),
        200,
    );
}

export async function createComment(
    api: ApiClient,
    teamId: string,
    content: string,
): Promise<IdVal> {
    return expectSuccessData(
        await api.post<SuccessBody<IdVal>>("/api/v1/comments", {
            content,
            team_id: teamId,
        }),
        201,
    );
}

export async function listTeamComments(api: ApiClient, teamId: string): Promise<CommentInfoVal[]> {
    return expectSuccessList(
        await api.get<SuccessBody<CommentInfoVal[]>>(`/api/v1/teams/${teamId}/comments?offset=0&limit=100`),
        200,
    );
}

// ---------- translation port (import / export) ----------

// Poprako JSON export (unenveloped). Returns parsed JSON.
export async function exportPoprako(api: ApiClient, chapterId: string): Promise<PoprakoExportVal> {
    const response = await api.get<PoprakoExportVal>(
        `/api/v1/chapters/${chapterId}/translations/export?format=poprako`,
    );

    if (response.status !== 200) {
        throw new Error(`poprako export failed: ${response.status} ${response.rawText}`);
    }

    const parsed = JSON.parse(response.rawText) as PoprakoExportVal;

    return parsed;
}

// Label-plus text export/download (raw text body).
export async function exportLabelPlus(api: ApiClient, chapterId: string): Promise<string> {
    const response = await api.get<string>(
        `/api/v1/chapters/${chapterId}/translations/export/download?format=label-plus`,
    );

    if (response.status !== 200) {
        throw new Error(`label-plus export failed: ${response.status} ${response.rawText}`);
    }

    return response.rawText;
}

// Poprako JSON import content built from a poprako export. The export and
// import shapes DIFFER (export is `ChapterTranslationExportVal`, import is
// `ChapterPoprakoProjectImport` with `image_filename`/`x`/`y`/`index_in_page`/
// `is_inbox`/`prooved_text`/`is_prooved`), so convert here.
export function buildPoprakoImportContent(
    exportVal: PoprakoExportVal,
    author: string,
): string {
    const project = {
        author,
        title: exportVal.comic_title,
        pages: exportVal.pages.map((page) => ({
            image_filename: page.page_id,
            units: page.units.map((unit) => ({
                id: unit.unit_id,
                x: unit.x_coord,
                y: unit.y_coord,
                index_in_page: unit.unit_index,
                is_inbox: unit.is_bubble,
                translated_text: unit.translated_text,
                prooved_text: unit.proofread_text,
                is_prooved: unit.is_proofread,
            })),
        })),
    };

    return JSON.stringify(project);
}

// Import translations into a chapter. `format` is "poprako" or "label-plus".
// Returns `{ imported_page_count, imported_unit_count }`.
export async function importTranslations(
    api: ApiClient,
    chapterId: string,
    format: "poprako" | "label-plus",
    content: string,
): Promise<{ imported_page_count: number; imported_unit_count: number }> {
    return expectSuccessData(
        await api.post<SuccessBody<{ imported_page_count: number; imported_unit_count: number }>>(
            `/api/v1/chapters/${chapterId}/translations/import`,
            { content, format },
        ),
        200,
    );
}

// Re-export the per-user client factory for convenience.
export { clientFor };

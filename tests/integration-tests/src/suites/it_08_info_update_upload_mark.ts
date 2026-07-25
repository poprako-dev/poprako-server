// it_08 — Info update, avatar/cover reserve+mark, announcements, comments,
// user profile.
//
// Preconditions:
//   - it_00 + it_01 have run. it_02 has run (need a team/workset/comic for
//     avatar/cover + C8 negatives).
//
// Postconditions:
//   - team avatar, trans_01 avatar, 星尘旅人 cover reserved+marked (URLs
//     non-null).
//   - announcement(s) + comments created on default team; ids recorded in
//     ctx.leftoverAnnouncementIds / ctx.leftoverCommentIds.
//   - throwaway user self-deleted (cascade member).
//
// Covers test-plan: C8, H1, H2, H3.
//
// Grounded pins:
//   - announcement create: team ADMIN; list: team member.
//   - comment create/list: team member.
//   - user profile update: SELF only (token.user_id == data.id) -> 403/4
//     otherwise. (sadmin cannot edit another user's profile.)
//   - user delete: SELF only (token.user_id == id). The plan's "sadmin
//     deletes a throwaway user" is WRONG; adjusted to self-delete.
//   - qid conflict on user update -> 422/2 (`error-already-exists` via
//     unique index on t_user.f_qid), NOT 409.
//   - team/workset/comic profile update perms: team ADMIN (sadmin only).
//   - avatar/cover reserve+mark perms: same as parent entity update
//     (team admin for team avatar; self for user avatar; team admin for
//     comic cover).
//
// Status: IMPLEMENTED.

import assert from "node:assert/strict";

import { testEnv } from "../config/env.js";
import { expectError, expectStatus } from "../http/assertions.js";
import type { ErrorBody } from "../http/apiClient.js";
import { ApiClient } from "../http/apiClient.js";
import {
    createAnnouncement,
    createComment,
    createMemberInvitation,
    getComic,
    getTeam,
    getUserInfo,
    listMyMembers,
    listTeamAnnouncements,
    listTeamAnnouncementsPaged,
    listTeamComments,
    listTeamMembers,
    login,
    markComicCoverUploaded,
    markTeamAvatarUploaded,
    markUserAvatarUploaded,
    registerInvitee,
    reserveComicCover,
    reserveTeamAvatar,
    reserveUserAvatar,
} from "../http/fixtures.js";
import { nickname, password, qid, titled } from "../state/prefix.js";
import { ROLE } from "../state/roles.js";
import type { RunCtx } from "../state/runCtx.js";

export const IMPLEMENTED = true as const;

export async function runIt08Module(ctx: RunCtx): Promise<void> {
    const teamId = ctx.ids.defaultTeamId;
    const trans01 = ctx.users.get("trans_01")!;
    const trans02 = ctx.users.get("trans_02")!;

    // ---------- C8. avatar / cover reserve + mark ----------

    // team avatar
    const teamAvatarReserve = await reserveTeamAvatar(ctx.sadmin, teamId, "png");

    assert.ok(teamAvatarReserve.slot?.put_url.startsWith("http"));
    assert.ok(Number.isInteger(teamAvatarReserve.slot?.image_version));

    await markTeamAvatarUploaded(ctx.sadmin, teamId, teamAvatarReserve.slot!.image_version);

    const teamAfterAvatar = await getTeam(ctx.sadmin, teamId);

    assert.ok(teamAfterAvatar.avatar_url, "team avatar_url must be non-null after mark");

    // stale avatar version -> 422/2
    expectError(
        await ctx.sadmin.post<ErrorBody>(`/api/v1/teams/${teamId}/avatar/mark-uploaded`, {
            image_version: teamAvatarReserve.slot!.image_version - 1,
        }),
        422,
        2,
    );

    // user avatar: self only. trans_01 reserves + marks own avatar.
    const trans01AvatarReserve = await reserveUserAvatar(trans01.api, trans01.userId, "png");

    assert.ok(trans01AvatarReserve.slot?.put_url.startsWith("http"));

    await markUserAvatarUploaded(trans01.api, trans01.userId, trans01AvatarReserve.slot!.image_version);

    const trans01AfterAvatar = await getUserInfo(ctx.sadmin, trans01.userId);

    assert.ok(trans01AfterAvatar.avatar_url, "trans_01 avatar_url non-null after mark");

    // non-owner reserve trans_01's avatar -> 403/4 (trans_02 tries)
    expectError(
        await trans02.api.post<ErrorBody>(`/api/v1/users/${trans01.userId}/avatar/reserve`, {
            image_hash: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
            new_byte_len: 1,
            ext: "png",
        }),
        403,
        4,
    );

    // comic cover: 星尘旅人 (team admin = sadmin)
    const xingchenId = ctx.ids.comicIds["星尘旅人"]!;

    const coverReserve = await reserveComicCover(ctx.sadmin, xingchenId, "png");

    assert.ok(coverReserve.slot?.put_url.startsWith("http"));
    assert.ok(Number.isInteger(coverReserve.slot?.image_version));

    await markComicCoverUploaded(ctx.sadmin, xingchenId, coverReserve.slot!.image_version);

    const comicAfterCover = await getComic(ctx.sadmin, xingchenId);

    assert.ok(comicAfterCover.cover_url, "comic cover_url non-null after mark");

    // non-admin (trans_01) reserve comic cover -> 403/4
    expectError(
        await trans01.api.post<ErrorBody>(`/api/v1/comics/${xingchenId}/cover/reserve`, {
            image_hash: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
            new_byte_len: 1,
            ext: "png",
        }),
        403,
        4,
    );

    // non-existent team avatar reserve -> 403/4 (permission check for non-existing team returns perm error)
    expectError(
        await ctx.sadmin.post<ErrorBody>("/api/v1/teams/team-does-not-exist/avatar/reserve", {
            image_hash: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
            new_byte_len: 1,
            ext: "png",
        }),
        403,
        4,
    );

    // ---------- H1. announcement ----------

    const ann1 = await createAnnouncement(ctx.sadmin, teamId, titled("ann-1"), "content-1");

    ctx.leftoverAnnouncementIds.push(ann1.id);

    const ann2 = await createAnnouncement(ctx.sadmin, teamId, titled("ann-2"), "content-2");

    ctx.leftoverAnnouncementIds.push(ann2.id);

    const annList = await listTeamAnnouncements(ctx.sadmin, teamId);

    assert.ok(annList.length >= 2);
    assert.ok(annList.find((a) => a.id === ann1.id));
    assert.ok(annList.find((a) => a.id === ann2.id));

    const ann1Full = annList.find((a) => a.id === ann1.id)!;

    assert.equal(ann1Full.team_id, teamId);
    assert.equal(ann1Full.user_id, ctx.ids.defaultUserId);
    assert.equal(ann1Full.title, titled("ann-1"));
    assert.equal(ann1Full.content, "content-1");

    // non-admin (trans_01) create announcement -> 403/4
    expectError(
        await trans01.api.post<ErrorBody>("/api/v1/announcements", {
            content: "x",
            team_id: teamId,
            title: "x",
        }),
        403,
        4,
    );

    // non-existent team -> 403/4 (admin check for non-existing team returns perm error)
    expectError(
        await ctx.sadmin.post<ErrorBody>("/api/v1/announcements", {
            content: "x",
            team_id: "team-does-not-exist",
            title: "x",
        }),
        403,
        4,
    );

    // pagination: limit=1 returns at most 1; offset=1 excludes the first
    const page1 = await listTeamAnnouncementsPaged(ctx.sadmin, teamId, 0, 1);

    assert.ok(page1.length <= 1, "limit=1 must return at most 1");

    const page2 = await listTeamAnnouncementsPaged(ctx.sadmin, teamId, 1, 50);

    assert.ok(!page2.find((a) => a.id === page1[0]?.id), "offset=1 must exclude the first");

    // ---------- H2. comment ----------

    const commenters = [ctx.sadmin, trans01, ctx.users.get("proof_01")!, ctx.users.get("type_01")!, ctx.users.get("review_01")!];
    const createdCommentIds: string[] = [];

    for (let i = 0; i < commenters.length; i++) {
        const commenter = commenters[i]!;

        // sadmin is ctx.sadmin (an ApiClient, not a UserClient); use it directly.
        const api = i === 0 ? ctx.sadmin : (commenter as { api: typeof ctx.sadmin }).api;

        const comment = await createComment(api, teamId, `comment-${i}`);

        createdCommentIds.push(comment.id);
        ctx.leftoverCommentIds.push(comment.id);
    }

    const commentList = await listTeamComments(ctx.sadmin, teamId);

    assert.ok(commentList.length >= 5);

    for (const id of createdCommentIds) {
        assert.ok(commentList.find((c) => c.id === id), `comment ${id} must be in list`);
    }

    // comment does not bump team.updated_at
    const teamBeforeComment = await getTeam(ctx.sadmin, teamId);
    const extraComment = await createComment(trans01.api, teamId, "no-bump-comment");

    ctx.leftoverCommentIds.push(extraComment.id);

    const teamAfterComment = await getTeam(ctx.sadmin, teamId);

    assert.equal(teamAfterComment.updated_at, teamBeforeComment.updated_at, "comment must not bump team.updated_at");

    // ---------- H3. user profile update ----------

    // self update nickname (keep qid stable so later logins still work)
    const trans01Before = await getUserInfo(ctx.sadmin, trans01.userId);
    const newNickname = nickname("trans_01_renamed");

    expectStatus(
        await trans01.api.put<null>(`/api/v1/users/${trans01.userId}`, {
            id: trans01.userId,
            nickname: newNickname,
            qid: trans01.qid,
        }),
        204,
    );

    const trans01After = await getUserInfo(ctx.sadmin, trans01.userId);

    assert.equal(trans01After.nickname, newNickname);
    assert.equal(trans01After.qid, trans01.qid);

    // restore nickname for downstream consistency
    await trans01.api.put<null>(`/api/v1/users/${trans01.userId}`, {
        id: trans01.userId,
        nickname: trans01Before.nickname,
        qid: trans01.qid,
    });

    // qid conflict: trans_02 sets qid to trans_01's qid -> 422/2 (error-already-exists)
    expectError(
        await trans02.api.put<ErrorBody>(`/api/v1/users/${trans02.userId}`, {
            id: trans02.userId,
            nickname: nickname(trans02.persona),
            qid: trans01.qid,
        }),
        422,
        2,
    );

    // trans_02 modifies trans_01's profile -> 403/4
    expectError(
        await trans02.api.put<ErrorBody>(`/api/v1/users/${trans01.userId}`, {
            id: trans01.userId,
            nickname: "hijack",
            qid: trans01.qid,
        }),
        403,
        4,
    );

    // path/body id mismatch -> 422 code 7
    expectError(
        await trans01.api.put<ErrorBody>(`/api/v1/users/${trans01.userId}`, {
            id: "not-the-path-id",
            nickname: "x",
            qid: trans01.qid,
        }),
        422,
        7,
    );

    // ---------- H3.5 self-delete a throwaway user ----------

    // create a throwaway invitation + register
    const throwawayQid = qid("throwaway_01");
    const throwawayInv = await createMemberInvitation(ctx.sadmin, teamId, throwawayQid, ROLE.RAW_PROVIDER);

    const throwawayClient = await registerInvitee(
        throwawayInv.code,
        throwawayQid,
        nickname("throwaway_01"),
        password("throwaway_01"),
    );

    // throwaway is a default-team member
    const throwawayMembers = await listMyMembers(throwawayClient.api);

    assert.ok(throwawayMembers.find((m) => m.team_id === teamId), "throwaway must be a default-team member");

    // sadmin tries to delete throwaway -> 403/4 (only self can delete)
    expectError(
        await ctx.sadmin.delete<ErrorBody>(`/api/v1/users/${throwawayClient.userId}`),
        403,
        4,
    );

    // throwaway self-deletes -> 204
    expectStatus(
        await throwawayClient.api.delete<null>(`/api/v1/users/${throwawayClient.userId}`),
        204,
    );

    // deleted user login -> 422/2 (qid lookup returns NotFound → Args)
    const loginClient = new ApiClient(testEnv.apiBaseUrl);

    expectError(
        await loginClient.post<ErrorBody>("/api/v1/auth/login", {
            password: password("throwaway_01"),
            qid: throwawayQid,
        }),
        422,
        2,
    );

    // deleted user's member row gone from default-team member list
    const teamMembersAfterDelete = await listTeamMembers(ctx.sadmin, teamId);

    assert.ok(
        !teamMembersAfterDelete.find((m) => m.user_id === throwawayClient.userId),
        "deleted user's membership must be gone",
    );

    // re-login trans_01 to confirm the suite's trans_01 client is still valid
    // (we did not touch trans_01's credentials).
    await login(trans01.api, trans01.qid, password("trans_01"));
}

// it_00 — Bootstrap, auth, and default seed discovery.
//
// Preconditions:
//   - Database has been reset to seed state by `main.ts` (sadmin user,
//     default team, default member).
//   - `ctx.sadmin` is a fresh unauthenticated ApiClient.
//   - `ctx.ids.defaultTeamId` / `defaultUserId` / `defaultMemberId` are set
//     from `seedIds`.
//
// Postconditions:
//   - `ctx.sadmin` is authenticated (bearer token set).
//   - `ctx.ids.defaultTeamId` confirmed via API.
//   - No extra rows created; safe to run before any other module.
//
// Covers test-plan: A1 (sadmin login + default-data discovery),
// A2 (unauthenticated-access protection).
//
// Status: IMPLEMENTED.

import assert from "node:assert/strict";

import { seedIds } from "../db/seed.js";
import {
    expectError,
    expectNoContent,
    expectStatus,
    expectSuccessData,
    expectSuccessList,
} from "../http/assertions.js";
import type { ErrorBody, SuccessBody } from "../http/apiClient.js";
import { ApiClient } from "../http/apiClient.js";
import {
    assertCreatedBeforeUpdated,
    assertTimestampMs,
    getMyInfo,
    getTeam,
    listMyMembers,
    listTeams,
    login,
    logout,
} from "../http/fixtures.js";
import type { LoginVal, MemberInfoView, TeamInfoView } from "../http/types.js";
import { testEnv } from "../config/env.js";
import type { RunCtx } from "../state/runCtx.js";

export const IMPLEMENTED = true as const;

export async function runIt00Module(ctx: RunCtx): Promise<void> {
    // ---- A1. sadmin login and default-data discovery ----

    const loginVal = await login(ctx.sadmin, "123456", "123456");

    assert.equal(loginVal.user_id, seedIds.defaultUserId);

    const me = await getMyInfo(ctx.sadmin);

    assert.equal(me.id, seedIds.defaultUserId);
    assert.equal(me.is_sadmin, true);

    // /members/me?incl=team&offset=0&limit=20 returns at least the seed member.
    const myMembers = await listMyMembers(ctx.sadmin, "&incl=team");

    assert.ok(myMembers.length >= 1, "sadmin must have at least one membership");

    const defaultMember = myMembers.find((member) => member.team_id === ctx.ids.defaultTeamId);

    assert.ok(defaultMember, "sadmin must be a member of the default team");
    assert.equal(defaultMember?.user_id, seedIds.defaultUserId);
    assert.ok(defaultMember?.team, "incl=team must embed the team on /members/me");

    // /teams (no user_id) for sadmin returns 200 and includes the default team.
    const teams = await listTeams(ctx.sadmin);

    const defaultTeam = teams.find((team) => team.id === ctx.ids.defaultTeamId);

    assert.ok(defaultTeam, "default team must be present in /teams");

    // default team exists and has valid timestamps.
    const teamInfo = await getTeam(ctx.sadmin, ctx.ids.defaultTeamId);

    assert.ok(teamInfo.id, "team id must be present");

    // Timestamp fields are Unix-ms integers and created_at <= updated_at.
    assertTimestampMs(teamInfo.created_at);
    assertTimestampMs(teamInfo.updated_at);
    assertCreatedBeforeUpdated(teamInfo.created_at, teamInfo.updated_at);

    for (const member of myMembers) {
        assertTimestampMs(member.last_active_at);
    }

    // ---- A2. unauthenticated-access protection ----

    const anon = new ApiClient(testEnv.apiBaseUrl);

    expectError(await anon.get<ErrorBody>("/api/v1/users/me"), 401, 3);
    expectError(await anon.get<ErrorBody>("/api/v1/members/me?offset=0&limit=20"), 401, 3);
    expectError(await anon.get<ErrorBody>("/api/v1/teams?offset=0&limit=20"), 401, 3);
    expectError(
        await anon.post<ErrorBody>("/api/v1/worksets", {
            description: "x",
            name: "x",
            team_id: ctx.ids.defaultTeamId,
        }),
        401,
        3,
    );

    // logout clears the token; subsequent /users/me is 401.
    // Use a throwaway client so we don't destroy ctx.sadmin's session.
    const throwaway = new ApiClient(testEnv.apiBaseUrl);

    await login(throwaway, "123456", "123456");
    expectSuccessData(await throwaway.get<SuccessBody<{ id: string }>>("/api/v1/users/me"), 200);

    await logout(throwaway);
    expectError(await throwaway.get<ErrorBody>("/api/v1/users/me"), 401, 3);

    // Sanity: ctx.sadmin is still authenticated after A2 (we used `anon` and
    // `throwaway`, not ctx.sadmin, for the negative cases).
    expectSuccessData(
        await ctx.sadmin.get<SuccessBody<{ id: string }>>("/api/v1/users/me"),
        200,
    );
}

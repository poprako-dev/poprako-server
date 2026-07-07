import assert from "node:assert/strict";

import { seedIds } from "../db/seed.js";
import type { ErrorBody, SuccessBody } from "../http/apiClient.js";
import { expectError, expectStatus, expectSuccessData } from "../http/assertions.js";
import type { TestContext } from "../state/context.js";

interface IdVal {
  id: string;
}

interface CodeVal extends IdVal {
  code: string;
}

interface CreateComicVal extends IdVal {
  chapter_id: string;
}

interface ReserveChapterPagesVal {
  creations: Array<{
    page_id: string;
    image_version: number;
  }>;
}

interface ReserveVersionVal {
  avatar_version?: number;
  cover_version?: number;
  image_version?: number;
}

export async function runAllApiSmokeSuite(context: TestContext): Promise<void> {
  assert.ok(context.auth);
  assert.ok(context.ids.worksetId);
  assert.ok(context.ids.comicId);
  assert.ok(context.ids.chapterId);
  assert.ok(context.ids.pageId);

  await smokeAuthRoutes(context);
  await smokeUserRoutes(context);
  await smokeTeamRoutes(context);
  await smokeMemberRoutes(context);
  await smokeMemberInvitationRoutes(context);
  await smokeWorksetRoutes(context);
  await smokeComicRoutes(context);
  await smokeChapterRoutes(context);
  await smokePageRoutes(context);
  await smokeUnitRoutes(context);
  await smokeAssignmentRoutes(context);
  await smokeAssignmentInvitationRoutes(context);
  await smokeSystemMailRoutes(context);
  await smokeAnnouncementRoutes(context);
  await smokeCommentRoutes(context);
}

async function smokeAuthRoutes(context: TestContext): Promise<void> {
  expectError(
    await context.api.post<ErrorBody>("/api/v1/auth/register", {
      code: "missing-code",
      nickname: "Smoke Register",
      password: "password",
      qid: "smoke-register-qid",
    }),
    422,
    2,
  );

  expectStatus(await context.api.post<null>("/api/v1/auth/logout"), 204);
}

async function smokeUserRoutes(context: TestContext): Promise<void> {
  const userId = context.auth?.userId;

  assert.ok(userId);

  expectSuccessData(await context.api.get(`/api/v1/users/${userId}`), 200);

  expectStatus(
    await context.api.put<null>(`/api/v1/users/${userId}`, {
      id: userId,
      nickname: "SuperAdmin-OvO",
      qid: "123456",
    }),
    204,
  );

  expectError(
    await context.api.post<ErrorBody>(`/api/v1/users/not-${userId}/avatar/reserve`, {
      file_ext: "png",
    }),
    403,
    4,
  );

  expectError(
    await context.api.post<ErrorBody>(`/api/v1/users/not-${userId}/avatar/mark-uploaded`, {
      avatar_version: 1,
    }),
    403,
    4,
  );

  expectError(await context.api.delete<ErrorBody>(`/api/v1/users/not-${userId}`), 403, 4);
}

async function smokeTeamRoutes(context: TestContext): Promise<void> {
  const listResponse = await context.api.get<SuccessBody<unknown[]>>("/api/v1/teams?offset=0&limit=20");

  assert.ok(expectSuccessData(listResponse, 200).length >= 1);

  const teamResponse = await context.api.post<SuccessBody<IdVal>>("/api/v1/teams", {
    description: "Temporary smoke team",
    name: "Smoke Team",
  });

  const team = expectSuccessData(teamResponse, 201);

  expectSuccessData(await context.api.get(`/api/v1/teams/${team.id}`), 200);

  expectStatus(
    await context.api.put<null>(`/api/v1/teams/${team.id}`, {
      description: "Temporary smoke team updated",
      id: team.id,
      name: "Smoke Team Updated",
    }),
    204,
  );

  expectSuccessData(
    await context.api.post<SuccessBody<ReserveVersionVal>>(`/api/v1/teams/${team.id}/avatar/reserve`, {
      file_ext: "png",
    }),
    200,
  );

  expectStatus(
    await context.api.post<null>(`/api/v1/teams/${team.id}/avatar/mark-uploaded`, {
      avatar_version: 1,
    }),
    204,
  );

  expectStatus(await context.api.delete<null>(`/api/v1/teams/${team.id}`), 204);
}

async function smokeMemberRoutes(context: TestContext): Promise<void> {
  const teamId = context.ids.teamId;
  const userId = context.auth?.userId;

  assert.ok(userId);

  expectSuccessData(
    await context.api.get(`/api/v1/members?team_id=${teamId}&offset=0&limit=20`),
    200,
  );

  expectSuccessData(await context.api.get("/api/v1/members/me?offset=0&limit=20"), 200);

  expectError(
    await context.api.post<ErrorBody>("/api/v1/members", {
      roles: 128,
      team_id: teamId,
      user_id: userId,
    }),
    422,
    2,
  );

  expectStatus(
    await context.api.put<null>(`/api/v1/members/${seedIds.defaultMemberId}/roles`, {
      id: seedIds.defaultMemberId,
      roles: 128,
    }),
    204,
  );

  expectError(await context.api.post<ErrorBody>("/api/v1/members/join", { code: "missing-code" }), 422, 2);
  expectError(await context.api.delete<ErrorBody>("/api/v1/members/missing-member"), 422, 2);
}

async function smokeMemberInvitationRoutes(context: TestContext): Promise<void> {
  const invitationResponse = await context.api.post<SuccessBody<CodeVal>>(
    "/api/v1/member-invitations",
    {
      invitee_qid: "smoke-member-invitee",
      roles: 2,
      team_id: context.ids.teamId,
    },
  );

  const invitation = expectSuccessData(invitationResponse, 201);

  expectSuccessData(
    await context.api.get(
      `/api/v1/teams/${context.ids.teamId}/member-invitations?pending=true&offset=0&limit=20`,
    ),
    200,
  );

  expectStatus(
    await context.api.put<null>(`/api/v1/member-invitations/${invitation.id}/roles`, {
      id: invitation.id,
      roles: 4,
    }),
    204,
  );

  expectStatus(await context.api.delete<null>(`/api/v1/member-invitations/${invitation.id}`), 204);
}

async function smokeWorksetRoutes(context: TestContext): Promise<void> {
  const worksetId = context.ids.worksetId;

  assert.ok(worksetId);

  expectSuccessData(await context.api.get(`/api/v1/teams/${context.ids.teamId}/worksets?offset=0&limit=20`), 200);
  expectSuccessData(await context.api.get(`/api/v1/worksets/${worksetId}`), 200);

  expectStatus(
    await context.api.put<null>(`/api/v1/worksets/${worksetId}`, {
      description: "API integration test workset updated",
      id: worksetId,
      name: "API Integration Updated",
    }),
    204,
  );

  expectError(await context.api.delete<ErrorBody>("/api/v1/worksets/missing-workset"), 422, 2);
}

async function smokeComicRoutes(context: TestContext): Promise<void> {
  const worksetId = context.ids.worksetId;
  const comicId = context.ids.comicId;

  assert.ok(worksetId);
  assert.ok(comicId);

  expectSuccessData(await context.api.get(`/api/v1/worksets/${worksetId}/comics?offset=0&limit=20`), 200);
  expectSuccessData(await context.api.get(`/api/v1/comics/${comicId}`), 200);

  expectStatus(
    await context.api.put<null>(`/api/v1/comics/${comicId}`, {
      author: "Integration Author Updated",
      description: "Smoke updated comic",
      id: comicId,
      title: "Integration Comic Updated",
    }),
    204,
  );

  expectSuccessData(
    await context.api.post<SuccessBody<ReserveVersionVal>>(`/api/v1/comics/${comicId}/cover/reserve`, {
      file_ext: "png",
    }),
    200,
  );

  expectStatus(
    await context.api.post<null>(`/api/v1/comics/${comicId}/cover/mark-uploaded`, {
      cover_version: 1,
    }),
    204,
  );

  expectStatus(
    await context.api.post<null>(`/api/v1/comics/${comicId}/mark-completed`, {
      is_completed: true,
    }),
    204,
  );

  expectError(await context.api.delete<ErrorBody>("/api/v1/comics/missing-comic"), 422, 2);
}

async function smokeChapterRoutes(context: TestContext): Promise<void> {
  const comicId = context.ids.comicId;
  const chapterId = context.ids.chapterId;

  assert.ok(comicId);
  assert.ok(chapterId);

  const extraChapterResponse = await context.api.post<SuccessBody<IdVal>>("/api/v1/chapters", {
    comic_id: comicId,
    subtitle: "Smoke Extra Chapter",
  });

  const extraChapter = expectSuccessData(extraChapterResponse, 201);

  expectSuccessData(await context.api.get(`/api/v1/comics/${comicId}/chapters?offset=0&limit=20`), 200);
  expectSuccessData(await context.api.get(`/api/v1/comics/${comicId}/chapters/pinned`), 200);
  expectSuccessData(await context.api.get(`/api/v1/chapters/${chapterId}`), 200);

  expectStatus(
    await context.api.patch<null>(`/api/v1/chapters/${chapterId}`, {
      id: chapterId,
      pin: true,
      subtitle: "Integration Chapter Updated",
    }),
    204,
  );

  expectStatus(
    await context.api.post<null>(`/api/v1/chapters/${chapterId}/stage/advance`, {
      id: chapterId,
      oper: "advance",
      stage: "translate",
    }),
    204,
  );

  expectError(
    await context.api.post<ErrorBody>(`/api/v1/chapters/${chapterId}/translations/import`, {
      content: "invalid-import-content",
      format: "label-plus",
    }),
    422,
    2,
  );

  expectStatus(
    await context.api.get(`/api/v1/chapters/${chapterId}/translations/export?format=poprako`),
    200,
  );
  expectStatus(
    await context.api.get(`/api/v1/chapters/${chapterId}/translations/export/download?format=label-plus`),
    200,
  );

  expectStatus(await context.api.delete<null>(`/api/v1/chapters/${extraChapter.id}`), 204);
}

async function smokePageRoutes(context: TestContext): Promise<void> {
  const chapterId = context.ids.chapterId;
  const pageId = context.ids.pageId;

  assert.ok(chapterId);
  assert.ok(pageId);

  expectSuccessData(await context.api.get(`/api/v1/chapters/${chapterId}/pages?offset=0&limit=20`), 200);

  expectError(
    await context.api.post<ErrorBody>(`/api/v1/chapters/${chapterId}/pages/reserve`, {
      chapter_id: chapterId,
      file_ext: "jpg",
      page_count: 1,
    }),
    422,
    2,
  );

  expectSuccessData(
    await context.api.post<SuccessBody<ReserveVersionVal>>(`/api/v1/pages/${pageId}/image/reserve`, {
      file_ext: "jpg",
    }),
    200,
  );

  expectError(
    await context.api.post<ErrorBody>(`/api/v1/pages/${pageId}/image/mark-uploaded`, {
      image_version: 1,
    }),
    422,
    2,
  );

  expectError(
    await context.api.delete<ErrorBody>("/api/v1/chapters/missing-chapter/pages"),
    422,
    2,
  );
}

async function smokeUnitRoutes(context: TestContext): Promise<void> {
  const pageId = context.ids.pageId;

  assert.ok(pageId);

  expectSuccessData(await context.api.get(`/api/v1/pages/${pageId}/units?offset=0&limit=20`), 200);

  expectError(
    await context.api.post<ErrorBody>(`/api/v1/pages/${pageId}/units/save`, {
      diff: {
        opers: [],
        page_id: "wrong-page",
      },
      page_id: pageId,
    }),
    422,
    7,
  );
}

async function smokeAssignmentRoutes(context: TestContext): Promise<void> {
  const chapterId = context.ids.chapterId;
  const userId = context.auth?.userId;

  assert.ok(chapterId);
  assert.ok(userId);

  const assignments = expectSuccessData<unknown[]>(
    await context.api.get(`/api/v1/assignments?chapter_id=${chapterId}&offset=0&limit=20`),
    200,
  );

  assert.ok(assignments.length >= 1);

  expectError(
    await context.api.post<ErrorBody>("/api/v1/assignments/join", {
      chapter_id: chapterId,
      roles: 2,
    }),
    403,
    4,
  );

  expectError(
    await context.api.put<ErrorBody>(`/api/v1/chapters/${chapterId}/assignments/${userId}/roles`, {
      chapter_id: chapterId,
      roles: 3,
      user_id: userId,
    }),
    403,
    4,
  );

  expectError(await context.api.delete<ErrorBody>("/api/v1/assignments/missing-assignment"), 422, 2);
}

async function smokeAssignmentInvitationRoutes(context: TestContext): Promise<void> {
  const chapterId = context.ids.chapterId;

  assert.ok(chapterId);

  const invitationResponse = await context.api.post<SuccessBody<CodeVal>>(
    "/api/v1/assignment-invitations",
    {
      chapter_id: chapterId,
      invitee_qid: "smoke-assignment-invitee",
      roles: 2,
    },
  );

  const invitation = expectSuccessData(invitationResponse, 201);

  expectSuccessData(
    await context.api.get(`/api/v1/chapters/${chapterId}/assignment-invitations?pending=true&offset=0&limit=20`),
    200,
  );

  expectError(
    await context.api.post<ErrorBody>("/api/v1/assignment-invitations/join", {
      code: invitation.code,
    }),
    422,
    2,
  );

  expectStatus(await context.api.delete<null>(`/api/v1/assignment-invitations/${invitation.id}`), 204);
}

async function smokeSystemMailRoutes(context: TestContext): Promise<void> {
  expectSuccessData(await context.api.get("/api/v1/system-mails?offset=0&limit=20"), 200);

  expectStatus(
    await context.api.post<null>("/api/v1/system-mails/mark-read", {
      ids: [],
    }),
    204,
  );
}

async function smokeAnnouncementRoutes(context: TestContext): Promise<void> {
  const announcementResponse = await context.api.post<SuccessBody<IdVal>>("/api/v1/announcements", {
    content: "Smoke announcement content",
    team_id: context.ids.teamId,
    title: "Smoke announcement",
  });

  const announcement = expectSuccessData(announcementResponse, 201);

  assert.ok(announcement.id);
  context.ids.announcementId = announcement.id;

  expectSuccessData(await context.api.get(`/api/v1/teams/${context.ids.teamId}/announcements?offset=0&limit=20`), 200);
}

async function smokeCommentRoutes(context: TestContext): Promise<void> {
  expectSuccessData(await context.api.get(`/api/v1/teams/${context.ids.teamId}/comments?offset=0&limit=20`), 200);
}

import assert from "node:assert/strict";

import { seedIds } from "../db/seed.js";
import { expectSuccessData } from "../http/assertions.js";
import type { SuccessBody } from "../http/apiClient.js";
import type { TestContext } from "../state/context.js";

interface LoginVal {
  user_id: string;
  token: string;
}

interface UserInfoVal {
  id: string;
  nickname: string;
  qid: string;
  is_sadmin: boolean;
}

export async function runAuthSuite(context: TestContext): Promise<void> {
  const loginResponse = await context.api.post<SuccessBody<LoginVal>>("/api/v1/auth/login", {
    password: "123456",
    qid: "123456",
  });

  const loginVal = expectSuccessData(loginResponse, 200);

  assert.equal(loginVal.user_id, seedIds.defaultUserId);
  assert.ok(loginVal.token.length > 20);

  context.api.setToken(loginVal.token);
  context.auth = {
    token: loginVal.token,
    userId: loginVal.user_id,
  };

  const meResponse = await context.api.get<SuccessBody<UserInfoVal>>("/api/v1/users/me");
  const me = expectSuccessData(meResponse, 200);

  assert.equal(me.id, loginVal.user_id);
  assert.equal(me.qid, "123456");
  assert.equal(me.is_sadmin, true);
}

import test from "node:test";

import { testEnv } from "./config/env.js";
import {
  assertDatabaseIsSeedOnly,
  cleanupLeftoverRows,
  resetDatabase,
  seedIds,
} from "./db/seed.js";
import { ApiClient } from "./http/apiClient.js";
import { runAllApiSmokeSuite } from "./suites/allApiSmoke.js";
import { runAuthSuite } from "./suites/auth.js";
import { runErrorCaseSuite } from "./suites/errorCases.js";
import { runHealthSuite } from "./suites/health.js";
import { runProjectFlowSuite } from "./suites/projectFlow.js";
import type { TestContext } from "./state/context.js";

await test("poprako HTTP API integration", async (t) => {
  await resetDatabase();

  const context: TestContext = {
    api: new ApiClient(testEnv.apiBaseUrl),
    auth: null,
    ids: {
      teamId: seedIds.defaultTeamId,
    },
  };

  try {
    await t.test("health and auth boundary", async () => {
      await runHealthSuite(context);
    });

    await t.test("super admin login and profile", async () => {
      await runAuthSuite(context);
    });

    await t.test("project workflow", async () => {
      await runProjectFlowSuite(context);
    });

    await t.test("all API smoke", async () => {
      await runAllApiSmokeSuite(context);
    });

    await t.test("error envelopes", async () => {
      await runErrorCaseSuite(context);
    });
  } finally {
    await runCleanup(context);

    try {
      await assertDatabaseIsSeedOnly();
    } finally {
      await resetDatabase();
    }
  }
});

async function runCleanup(context: TestContext): Promise<void> {
  if (context.ids.worksetId) {
    await context.api.delete(`/api/v1/worksets/${context.ids.worksetId}`);
  }

  await cleanupLeftoverRows({
    commentId: context.ids.commentId,
    announcementId: context.ids.announcementId,
  });
}

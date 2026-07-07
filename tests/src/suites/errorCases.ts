import { seedIds } from "../db/seed.js";
import { expectError } from "../http/assertions.js";
import type { ErrorBody } from "../http/apiClient.js";
import type { TestContext } from "../state/context.js";

export async function runErrorCaseSuite(context: TestContext): Promise<void> {
  const mismatchedUpdateResponse = await context.api.put<ErrorBody>(
    `/api/v1/worksets/${context.ids.worksetId}`,
    {
      description: "mismatch",
      id: "not-the-path-id",
      name: "mismatch",
    },
  );

  expectError(mismatchedUpdateResponse, 422, 7);

  const invalidComicListResponse = await context.api.get<ErrorBody>(
    `/api/v1/worksets/${context.ids.worksetId}/comics?is_completed=true&stages=2&offset=0&limit=20`,
  );

  expectError(invalidComicListResponse, 422, 2);

  const missingTeamResponse = await context.api.get<ErrorBody>(
    `/api/v1/teams/${seedIds.defaultTeamId}-missing`,
  );

  expectError(missingTeamResponse, 422, 2);
}

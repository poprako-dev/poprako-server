import type { TestContext } from "../state/context.js";
import { expectError, expectStatus } from "../http/assertions.js";
import type { ErrorBody } from "../http/apiClient.js";

export async function runHealthSuite(context: TestContext): Promise<void> {
  const healthResponse = await context.api.get<null>("/api/health");

  expectStatus(healthResponse, 204);

  const protectedResponse = await context.api.get<ErrorBody>("/api/v1/users/me");

  expectError(protectedResponse, 401, 3);
}

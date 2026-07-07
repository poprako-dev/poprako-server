import assert from "node:assert/strict";

import type { ApiResponse, ErrorBody, SuccessBody } from "./apiClient.js";

export function expectStatus<T>(response: ApiResponse<T>, status: number): void {
  assert.equal(response.status, status, responseText(response));
}

export function expectSuccessData<T>(
  response: ApiResponse<SuccessBody<T>> | ApiResponse<unknown>,
  status: number,
): T {
  expectStatus(response, status);
  const body = response.body as SuccessBody<T> | null;

  assert.ok(body, "response should contain a success body");
  assert.equal(body.code, 0);

  return body.data;
}

export function expectError(
  response: ApiResponse<ErrorBody>,
  status: number,
  code?: number,
): ErrorBody {
  expectStatus(response, status);
  assert.ok(response.body, "response should contain an error body");
  assert.ok(isErrorBody(response.body), "response error body should be JSON");

  if (code !== undefined) {
    assert.equal(response.body.code, code);
  }

  return response.body;
}

function isErrorBody(body: unknown): body is ErrorBody {
  if (!body || typeof body !== "object") {
    return false;
  }

  return typeof (body as ErrorBody).code === "number";
}

function responseText<T>(response: ApiResponse<T>): string {
  return JSON.stringify(
    {
      body: response.body,
      status: response.status,
    },
    null,
    2,
  );
}

import path from "node:path";
import { fileURLToPath } from "node:url";

import dotenv from "dotenv";

const currentFilePath = fileURLToPath(import.meta.url);
const testsSourceRoot = path.resolve(path.dirname(currentFilePath), "..");
const testsRoot = path.resolve(testsSourceRoot, "..");
const projectRoot = path.resolve(testsRoot, "..");

dotenv.config({ path: path.join(projectRoot, ".env") });

const databaseUrl = process.env.DATABASE_URL;

if (!databaseUrl) {
  throw new Error("DATABASE_URL must be set");
}

const apiBaseUrl = process.env.API_BASE_URL ?? "http://127.0.0.1:8888";

export const testEnv = {
  apiBaseUrl: apiBaseUrl.replace(/\/$/, ""),
  databaseUrl,
  projectRoot,
  testsRoot,
};

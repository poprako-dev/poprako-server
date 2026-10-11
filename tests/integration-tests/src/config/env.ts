const integrationDatabaseUrl = process.env.INTEGRATION_DATABASE_URL;

if (!integrationDatabaseUrl) {
    throw new Error("INTEGRATION_DATABASE_URL must be set");
}

const apiBaseUrl = process.env.API_BASE_URL ?? "http://127.0.0.1:8888";

export const testEnv = {
    apiBaseUrl: apiBaseUrl.replace(/\/$/, ""),
    databaseUrl: integrationDatabaseUrl,
};

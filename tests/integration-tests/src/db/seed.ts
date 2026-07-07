import { Client } from "pg";

import { testEnv } from "../config/env.js";

const DEFAULT_TEAM_ID = "team-00000000-0000-0000-0000-000000000001";
const DEFAULT_USER_ID = "user-00000000-0000-0000-0000-000000000001";
const DEFAULT_MEMBER_ID = "member-00000000-0000-0000-0000-000000000001";
const DEFAULT_PASSWORD_HASH =
  "$argon2id$v=19$m=65536,t=3,p=4$UrCPl9xY0hk3LpfQWl+ZVA$4d+zkTiD9ghoc6XtJJSHpcvfzUpAK1IiZ5MAQezLgrE";

async function withClient(run: (client: Client) => Promise<void>): Promise<void> {
  const client = new Client({ connectionString: testEnv.databaseUrl });

  await client.connect();

  try {
    await run(client);
  } finally {
    await client.end();
  }
}

export async function resetDatabase(): Promise<void> {
  await withClient(async (client) => {
    const tableResult = await client.query<{ tablename: string }>(`
      SELECT tablename
      FROM pg_tables
      WHERE schemaname = 'public'
        AND tablename LIKE 't\\_%' ESCAPE '\\'
      ORDER BY tablename
    `);

    const tableNames = tableResult.rows.map((row) => `"${row.tablename}"`);

    await client.query("BEGIN");

    try {
      if (tableNames.length > 0) {
        await client.query(`TRUNCATE TABLE ${tableNames.join(", ")} RESTART IDENTITY CASCADE`);
      }

      await client.query(
        `
          INSERT INTO "t_team" (
            "f_id",
            "f_name",
            "f_description"
          ) VALUES ($1, $2, $3)
        `,
        [DEFAULT_TEAM_ID, "PRTS", "Default team"],
      );

      await client.query(
        `
          INSERT INTO "t_user" (
            "f_id",
            "f_nickname",
            "f_qid",
            "f_is_sadmin",
            "f_password_hash"
          ) VALUES ($1, $2, $3, TRUE, $4)
        `,
        [DEFAULT_USER_ID, "SuperAdmin-OvO", "123456", DEFAULT_PASSWORD_HASH],
      );

      await client.query(
        `
          INSERT INTO "t_member" (
            "f_id",
            "f_user_id",
            "f_user_nickname",
            "f_team_id",
            "f_assigned_raw_provider_at",
            "f_assigned_translator_at",
            "f_assigned_proofreader_at",
            "f_assigned_typesetter_at",
            "f_assigned_redrawer_at",
            "f_assigned_reviewer_at",
            "f_assigned_publisher_at",
            "f_assigned_admin_at",
            "f_assigned_bot_at"
          ) VALUES (
            $1,
            $2,
            $3,
            $4,
            NOW(),
            NOW(),
            NOW(),
            NOW(),
            NOW(),
            NOW(),
            NOW(),
            NOW(),
            NOW()
          )
        `,
        [DEFAULT_MEMBER_ID, DEFAULT_USER_ID, "SuperAdmin-OvO", DEFAULT_TEAM_ID],
      );

      await client.query("COMMIT");
    } catch (error) {
      await client.query("ROLLBACK");

      throw error;
    }
  });
}

export async function assertDatabaseIsSeedOnly(): Promise<void> {
  await withClient(async (client) => {
    const tableResult = await client.query<{ tablename: string }>(`
      SELECT tablename
      FROM pg_tables
      WHERE schemaname = 'public'
        AND tablename LIKE 't\\_%' ESCAPE '\\'
      ORDER BY tablename
    `);

    const expectedCounts = new Map([
      ["t_member", "1"],
      ["t_team", "1"],
      ["t_user", "1"],
    ]);

    const mismatches: string[] = [];

    for (const row of tableResult.rows) {
      const countResult = await client.query<{ row_count: string }>(
        `SELECT COUNT(*)::text AS row_count FROM "${row.tablename}"`,
      );
      const rowCount = countResult.rows[0]?.row_count;
      const expectedCount = expectedCounts.get(row.tablename) ?? "0";

      if (rowCount !== expectedCount) {
        mismatches.push(
          `${row.tablename}: ${rowCount} rows, expected ${expectedCount}`,
        );
      }
    }

    if (mismatches.length > 0) {
      throw new Error(
        `database is not seed-only after suite:\n  - ${mismatches.join("\n  - ")}`,
      );
    }
  });
}

export async function grantChapterWorkerRoles(chapterId: string, userId: string): Promise<void> {
  await withClient(async (client) => {
    await client.query(
      `
        UPDATE "t_assignment"
        SET
          "f_assigned_raw_provider_at" = COALESCE("f_assigned_raw_provider_at", NOW()),
          "f_assigned_translator_at" = COALESCE("f_assigned_translator_at", NOW()),
          "f_updated_at" = NOW()
        WHERE "f_chapter_id" = $1
          AND "f_user_id" = $2
      `,
      [chapterId, userId],
    );
  });
}

export interface LeftoverIds {
  commentId?: string;
  announcementId?: string;
}

/// Removes suite-created rows that have no HTTP delete endpoint (comments and
/// announcements), and clears the `t_local_message` outbox populated by prom
/// for every image reservation. Business entities reachable via the API
/// (workset -> comic -> chapter -> page -> unit -> assignment) are deleted
/// through `DELETE /api/v1/worksets/{id}` by the caller, which cascades by FK.
export async function cleanupLeftoverRows(ids: LeftoverIds): Promise<void> {
  await withClient(async (client) => {
    await client.query("BEGIN");

    try {
      if (ids.commentId) {
        await client.query(`DELETE FROM "t_comment" WHERE "f_id" = $1`, [ids.commentId]);
      }

      if (ids.announcementId) {
        await client.query(`DELETE FROM "t_announcement" WHERE "f_id" = $1`, [
          ids.announcementId,
        ]);
      }

      await client.query(`TRUNCATE TABLE "t_local_message" RESTART IDENTITY`);

      await client.query("COMMIT");
    } catch (error) {
      await client.query("ROLLBACK");

      throw error;
    }
  });
}

export const seedIds = {
  defaultMemberId: DEFAULT_MEMBER_ID,
  defaultTeamId: DEFAULT_TEAM_ID,
  defaultUserId: DEFAULT_USER_ID,
};

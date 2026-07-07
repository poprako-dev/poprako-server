import assert from "node:assert/strict";

import { grantChapterWorkerRoles } from "../db/seed.js";
import { expectStatus, expectSuccessData } from "../http/assertions.js";
import type { SuccessBody } from "../http/apiClient.js";
import type { TestContext } from "../state/context.js";

interface IdVal {
  id: string;
}

interface CreateComicVal {
  id: string;
  chapter_id: string;
}

interface ReserveChapterPagesVal {
  creations: Array<{
    page_id: string;
    put_url: string;
    image_version: number;
  }>;
}

interface SavePageUnitsVal {
  local_id_mappers: Array<{
    local_id: string;
    unit_id: string;
  }>;
  total_unit_count: number;
  translated_unit_count: number;
  proofread_unit_count: number;
}

interface ListUnitInfosVal {
  unit_infos: Array<{
    id: string;
    translated_text: string | null;
    proofread_text: string | null;
  }>;
  total_unit_count: number;
}

export async function runProjectFlowSuite(context: TestContext): Promise<void> {
  const worksetResponse = await context.api.post<SuccessBody<IdVal>>("/api/v1/worksets", {
    description: "API integration test workset",
    name: "API Integration",
    team_id: context.ids.teamId,
  });

  const workset = expectSuccessData(worksetResponse, 201);

  context.ids.worksetId = workset.id;

  const comicResponse = await context.api.post<SuccessBody<CreateComicVal>>("/api/v1/comics", {
    author: "Integration Author",
    description: "Created by the TS API integration suite",
    first_chapter_subtitle: "Integration Chapter",
    title: "Integration Comic",
    workset_id: workset.id,
  });

  const comic = expectSuccessData(comicResponse, 201);

  context.ids.comicId = comic.id;
  context.ids.chapterId = comic.chapter_id;

  assert.ok(context.auth?.userId);

  await grantChapterWorkerRoles(comic.chapter_id, context.auth.userId);

  const pagesResponse = await context.api.post<SuccessBody<ReserveChapterPagesVal>>(
    `/api/v1/chapters/${comic.chapter_id}/pages/reserve`,
    {
      chapter_id: comic.chapter_id,
      file_ext: "jpg",
      page_count: 2,
    },
  );

  const pages = expectSuccessData(pagesResponse, 200);

  assert.equal(pages.creations.length, 2);
  assert.ok(pages.creations[0]);
  assert.ok(pages.creations[0].put_url.startsWith("http"));

  context.ids.pageId = pages.creations[0].page_id;

  const saveUnitsResponse = await context.api.post<SuccessBody<SavePageUnitsVal>>(
    `/api/v1/pages/${context.ids.pageId}/units/save`,
    {
      diff: {
        opers: [
          {
            is_bubble: true,
            is_proofread: true,
            last_proofreader_id: null,
            last_translator_id: null,
            local_id: "local-unit-1",
            oper: "save",
            proofread_text: "Proofread text",
            translated_text: "Translated text",
            x_coord: 0.25,
            y_coord: 0.75,
          },
        ],
        page_id: context.ids.pageId,
      },
      page_id: context.ids.pageId,
    },
  );

  const savedUnits = expectSuccessData(saveUnitsResponse, 200);

  assert.equal(savedUnits.total_unit_count, 1);
  assert.equal(savedUnits.translated_unit_count, 1);
  assert.equal(savedUnits.proofread_unit_count, 1);
  assert.equal(savedUnits.local_id_mappers.length, 1);

  const unitId = savedUnits.local_id_mappers[0]?.unit_id;

  assert.ok(unitId);
  context.ids.unitId = unitId;

  const listUnitsResponse = await context.api.get<SuccessBody<ListUnitInfosVal>>(
    `/api/v1/pages/${context.ids.pageId}/units?offset=0&limit=20`,
  );

  const listedUnits = expectSuccessData(listUnitsResponse, 200);

  assert.equal(listedUnits.total_unit_count, 1);
  assert.equal(listedUnits.unit_infos[0]?.id, unitId);

  const commentResponse = await context.api.post<SuccessBody<IdVal>>("/api/v1/comments", {
    content: "API integration comment",
    team_id: context.ids.teamId,
  });

  const comment = expectSuccessData(commentResponse, 201);

  assert.ok(comment.id);
}

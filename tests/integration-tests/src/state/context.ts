import type { ApiClient } from "../http/apiClient.js";

export interface TestContext {
  api: ApiClient;
  auth: {
    token: string;
    userId: string;
  } | null;
  ids: {
    teamId: string;
    worksetId?: string;
    comicId?: string;
    chapterId?: string;
    pageId?: string;
    unitId?: string;
    commentId?: string;
    announcementId?: string;
  };
}

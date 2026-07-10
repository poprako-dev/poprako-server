// Presentation-side value types mirroring `crate::data::*` `*Val` structs.
// Field names match the JSON serialization (snake_case) exactly.

export interface UserInfoVal {
    id: string;
    nickname: string;
    qid: string;
    is_sadmin: boolean;
    avatar_url: string | null;
    created_at: number;
    updated_at: number;
}

export interface TeamInfoVal {
    id: string;
    name: string;
    description: string;
    avatar_url: string | null;
    workset_next_index: number;
    created_at: number;
    updated_at: number;
}

export interface WorksetInfoVal {
    id: string;
    team_id: string;
    index: number;
    name: string;
    description: string | null;
    comic_count: number;
    comic_next_index: number;
    created_at: number;
    updated_at: number;
}

export interface ComicInfoVal {
    id: string;
    workset_id: string;
    index: number;
    title: string;
    author: string;
    description: string | null;
    cover_url: string | null;
    chapter_count: number;
    chapter_next_index: number;
    creator_id: string;
    workset: WorksetInfoVal | null;
    team: TeamInfoVal | null;
    creator: UserInfoVal | null;
    pinned_chapter: ChapterInfoVal | null;
    last_active_at: number;
    created_at: number;
    updated_at: number;
}

export interface ChapterInfoVal {
    id: string;
    comic_id: string;
    comic: ComicInfoVal | null;
    is_pinned: boolean;
    index: number;
    subtitle: string;
    page_count: number;
    total_unit_count: number;
    translated_unit_count: number;
    proofread_unit_count: number;
    stages: number;
    creator_id: string;
    creator: UserInfoVal | null;
    created_at: number;
    updated_at: number;
}

export interface PageInfoVal {
    id: string;
    chapter_id: string;
    index: number;
    image_url: string | null;
    image_version: number | null;
    total_unit_count: number;
    translated_unit_count: number;
    proofread_unit_count: number;
    created_at: number;
    updated_at: number;
}

export interface UnitInfoVal {
    id: string;
    page_id: string;
    index: number;
    is_bubble: boolean;
    is_proofread: boolean;
    x_coord: number;
    y_coord: number;
    translated_text: string | null;
    last_translator_id: string | null;
    proofread_text: string | null;
    last_proofreader_id: string | null;
    created_at: number;
    updated_at: number;
}

export interface MemberInfoVal {
    id: string;
    user_id: string;
    nickname: string;
    last_active_at: number;
    team_id: string;
    user: UserInfoVal | null;
    team: TeamInfoVal | null;
    roles: number;
}

export interface MemberInvitationInfoVal {
    id: string;
    team_id: string;
    invitor_id: string;
    invitor: UserInfoVal | null;
    invitee_qid: string;
    code: string;
    pending: boolean;
    roles: number;
}

export interface AssignmentInfoVal {
    id: string;
    chapter_id: string;
    user_id: string;
    roles: number;
    user: UserInfoVal | null;
    chapter: ChapterInfoVal | null;
}

export interface AssignmentInvitationInfoVal {
    id: string;
    chapter_id: string;
    invitor_id: string;
    invitee_qid: string;
    code: string;
    pending: boolean;
    roles: number;
}

export interface AnnouncementInfoVal {
    id: string;
    team_id: string;
    user_id: string;
    title: string;
    content: string;
    user: UserInfoVal | null;
    created_at: number;
}

export interface CommentInfoVal {
    id: string;
    team_id: string;
    user_id: string;
    content: string;
    user: UserInfoVal | null;
    created_at: number;
}

export interface SystemMailInfoVal {
    id: string;
    user_id: string;
    title: string;
    content: string;
    read: boolean;
    created_at: number;
}

// Create/result payloads.

export interface IdVal {
    id: string;
}

export interface CodeVal extends IdVal {
    code: string;
}

export interface CreateComicVal extends IdVal {
    chapter_id: string;
}

export interface ArchiveComicVal {
    archived_comic_id: string;
}

export interface PageCreationVal {
    page_id: string;
    put_url: string;
    image_version: number;
}

export interface ReserveChapterPagesVal {
    creations: PageCreationVal[];
}

export interface ReserveVersionVal {
    put_url: string;
    page_id?: string;
    avatar_version?: number;
    cover_version?: number;
    image_version?: number;
}

export interface UnitIdMapperVal {
    local_id: string;
    unit_id: string;
}

export interface SavePageUnitsVal {
    local_id_mappers: UnitIdMapperVal[];
    total_unit_count: number;
    translated_unit_count: number;
    proofread_unit_count: number;
}

export interface ListPageUnitInfosVal {
    unit_infos: UnitInfoVal[];
    total_unit_count: number;
    translated_unit_count: number;
    proofread_unit_count: number;
}

export interface LoginVal {
    user_id: string;
    token: string;
}

// Poprako JSON export shape (unenveloped). Field names mirror
// `ChapterTranslationExportVal` / `PageTranslationExportVal` /
// `UnitTranslationExportVal` in `src/data/chapter_port.rs` +
// `src/data/page_port.rs` + `src/data/unit_port.rs`.
export interface PoprakoExportUnit {
    unit_id: string;
    unit_index: number;

    page_id: string;
    page_index: number;

    x_coord: number;
    y_coord: number;

    is_bubble: boolean;

    translated_text: string | null;
    translator_id: string | null;

    is_proofread: boolean;

    proofread_text: string | null;
    proofreader_id: string | null;
}

export interface PoprakoExportPage {
    page_id: string;
    page_index: number;

    image_url: string | null;

    units: PoprakoExportUnit[];
}

export interface PoprakoExportVal {
    chapter_id: string;
    chapter_index: number;
    chapter_subtitle: string | null;

    comic_id: string;
    comic_title: string;

    pages: PoprakoExportPage[];
}

// Presentation-side value types mirroring `crate::data::*` `*Val` structs.
// Field names match the JSON serialization (snake_case) exactly.

export interface UserInfoView {
    id: string;
    nickname: string;
    qid: string;
    is_sadmin: boolean;
    avatar_url: string | null;
    avatar_thumbnail_url: string | null;
    created_at: number;
    updated_at: number;
}

export interface TeamInfoView {
    id: string;
    name: string;
    description: string;
    avatar_url: string | null;
    avatar_thumbnail_url: string | null;
    created_at: number;
    updated_at: number;
}

export interface WorksetInfoView {
    id: string;
    team_id: string;
    index: number;
    name: string;
    description: string | null;
    comic_count: number;
    created_at: number;
    updated_at: number;
}

export interface ComicInfoView {
    id: string;
    workset_id: string;
    index: number;
    title: string;
    author: string;
    description: string | null;
    cover_url: string | null;
    cover_thumbnail_url: string | null;
    chapter_count: number;
    creator_id: string;
    workset: WorksetInfoView | null;
    team: TeamInfoView | null;
    creator: UserInfoView | null;
    last_active_at: number;
    is_archived: boolean;
    archived_at?: number;
    created_at: number;
    updated_at: number;
}

export interface TermbaseInfoView {
    id: string;
    team_id?: string;
    comic_id?: string;
    name: string;
    description?: string;
    term_count: number;
    creator_id: string;
    created_at: number;
    updated_at: number;
}

export interface TermInfoView {
    id: string;
    termbase_id: string;
    source: string;
    targets: string[];
    comment?: string;
    creator_id: string;
    created_at: number;
    updated_at: number;
}

export interface ListComicInfosVal {
    comics: ComicInfoView[];
    pinned_chapters: (ChapterInfoView | null)[];
    pinned_chapter_assignments: AssignmentInfoView[][];
}

export interface ChapterInfoView {
    id: string;
    comic_id: string;
    comic: ComicInfoView | null;
    is_pinned: boolean;
    index: number;
    subtitle: string;
    page_count: number;
    total_unit_count: number;
    translated_unit_count: number;
    proofread_unit_count: number;
    stages: number;
    creator_id: string;
    creator: UserInfoView | null;
    created_at: number;
    updated_at: number;
}

export interface PageInfoView {
    id: string;
    chapter_id: string;
    index: number;
    image_url?: string;
    image_thumbnail_url?: string;
    image_hash?: string;
    ext?: ImageExtension;
    total_unit_count: number;
    translated_unit_count: number;
    proofread_unit_count: number;
    created_at: number;
    updated_at: number;
}

export interface UnitInfoView {
    id: string;
    page_id: string;
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

export interface MemberInfoView {
    id: string;
    user_id: string;
    nickname: string;
    last_active_at: number;
    team_id: string;
    user: UserInfoView | null;
    team: TeamInfoView | null;
    roles: number;
}

export interface MemberInvitationInfoView {
    id: string;
    team_id: string;
    invitor_id: string;
    invitor: UserInfoView | null;
    invitee_qid: string;
    code: string;
    pending: boolean;
    roles: number;
}

export interface AssignmentInfoView {
    id: string;
    chapter_id: string;
    user_id: string;
    roles: number;
    user: UserInfoView | null;
    chapter: ChapterInfoView | null;
}

export interface AssignmentInvitationInfoView {
    id: string;
    chapter_id: string;
    invitor_id: string;
    invitee_qid: string;
    code: string;
    pending: boolean;
    roles: number;
}

export interface AnnouncementInfoView {
    id: string;
    team_id: string;
    user_id: string;
    title: string;
    content: string;
    user: UserInfoView | null;
    created_at: number;
}

export interface CommentInfoView {
    id: string;
    team_id: string;
    user_id: string;
    content: string;
    user: UserInfoView | null;
    created_at: number;
}

export interface SystemMailInfoView {
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
    archived_id: string;
}

export type ImageExtension = "jpg" | "jpeg" | "png" | "gif" | "webp" | "svg" | "avif" | "bmp" | "tif" | "tiff";

export interface PageImageInput {
    page_id: string | null;
    image_hash: string;
    new_byte_len?: number;
    ext: ImageExtension;
}

export interface UploadSlotVal {
    put_url: string;
    image_version: number;
    headers: Record<string, string>;
}

export interface ReservedPageVal {
    page_id: string;
    index: number;
    image_hash: string;
    ext: ImageExtension;
    slot: UploadSlotVal | null;
}

export interface ReserveChapterPagesVal {
    pages: ReservedPageVal[];
}

export interface ReserveImageVal {
    slot: UploadSlotVal | null;
}

export interface ListPageUnitInfosVal {
    unit_infos: UnitInfoView[];
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

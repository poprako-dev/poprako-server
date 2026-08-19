// Workflow stage constants. Values MUST match `crate::value::chapter::Stage`
// and `StageOper` in `src/value/chapter.rs`. JSON body DTO stages serialize as
// snake_case strings; `StageOper` serializes as `advance` / `revert`.
//
// Stage phase model (from `is_valid_stage_phase` / `try_modify_stage`):
//   raw-provide    : one-shot  Pending -> Completed          (1 advance)
//   translate      : Pending -> Active -> Completed          (2 advances)
//   proofread      : Pending -> Active -> Completed          (2 advances)
//   typeset-redraw : Pending -> Active -> Completed          (2 advances)
//   review         : one-shot  Pending -> Completed          (1 advance)
//   publish        : one-shot  Pending -> Completed          (1 advance, no revert)
//
// Revert rules:
//   - publish cannot revert (422).
//   - raw-provide / review revert Completed -> Pending.
//   - three-phase stages revert Completed -> Active -> Pending.

export type StageName =
    | "raw-provide"
    | "translate"
    | "proofread"
    | "typeset-redraw"
    | "review"
    | "publish";

export const STAGE: Record<StageName, StageName> = {
    "raw-provide": "raw-provide",
    translate: "translate",
    proofread: "proofread",
    "typeset-redraw": "typeset-redraw",
    review: "review",
    publish: "publish",
};

// Ordered pipeline. Each stage must be Completed before the next may advance.
export const STAGE_PIPELINE: readonly StageName[] = [
    "raw-provide",
    "translate",
    "proofread",
    "typeset-redraw",
    "review",
    "publish",
];

// Number of `advance` calls needed to move a stage from Pending to Completed.
export const ADVANCES_TO_COMPLETE: Record<StageName, number> = {
    "raw-provide": 1,
    translate: 2,
    proofread: 2,
    "typeset-redraw": 2,
    review: 1,
    publish: 1,
};

// Stages that accept an `Active` phase (three-phase). Others are one-shot.
export const THREE_PHASE_STAGES: readonly StageName[] = [
    "translate",
    "proofread",
    "typeset-redraw",
];

export type StageOper = "advance" | "revert";

export type ChapterStageInstr =
    | "raw_provide"
    | "translate"
    | "proofread"
    | "typeset_redraw"
    | "review"
    | "publish";

const CHAPTER_STAGE_INSTR: Record<StageName, ChapterStageInstr> = {
    "raw-provide": "raw_provide",
    translate: "translate",
    proofread: "proofread",
    "typeset-redraw": "typeset_redraw",
    review: "review",
    publish: "publish",
};

export function chapterStageInstr(stage: StageName): ChapterStageInstr {
    return CHAPTER_STAGE_INSTR[stage];
}

// Bit offset of each stage inside the `stages` mask (2 bits per stage).
// raw-provide=0, translate=2, proofread=4, typeset-redraw=6, review=8, publish=10.
export const STAGE_BIT_OFFSET: Record<StageName, number> = {
    "raw-provide": 0,
    translate: 2,
    proofread: 4,
    "typeset-redraw": 6,
    review: 8,
    publish: 10,
};

// Phase encoding inside each 2-bit stage slot: 0 = Pending, 1 = Active, 2 = Completed.
export const PHASE = {
    PENDING: 0,
    ACTIVE: 1,
    COMPLETED: 2,
} as const;

// Decode a stage's phase from the chapter `stages` mask.
export function stagePhase(stagesMask: number, stage: StageName): number {
    const offset = STAGE_BIT_OFFSET[stage];

    return (stagesMask >> offset) & 0b11;
}

// Build a stage-advance request body for `POST /chapters/{id}/stage/advance`.
export function stageAdvanceBody(chapterId: string, stage: StageName): {
    id: string;
    oper: "advance";
    stage: ChapterStageInstr;
} {
    return { id: chapterId, oper: "advance", stage: chapterStageInstr(stage) };
}

export function stageRevertBody(chapterId: string, stage: StageName): {
    id: string;
    oper: "revert";
    stage: ChapterStageInstr;
} {
    return { id: chapterId, oper: "revert", stage: chapterStageInstr(stage) };
}

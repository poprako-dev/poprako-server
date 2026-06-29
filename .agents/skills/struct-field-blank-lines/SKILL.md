---
name: struct-field-blank-lines
description: "Enforces blank-line-separated logical field groups in Rust struct fields — model/*.rs, data/*.rs. All structs with ≥5 fields must group related fields with blank lines, following the codebase convention. Detects and fixes missing blank-line grouping."
---

# Struct Field Blank-Line Grouping

Rust structs in `model/*.rs` and `data/*.rs` must group their fields into
logical blocks separated by blank lines. This applies to **any struct with 5 or
more fields**. Smaller structs (≤4 fields) may omit grouping at author's
discretion.

## Rules

1. **`id` stands alone.** Always followed by a blank line — it is the entity
   identifier and is never grouped with other fields.

2. **Group fields by logical concern.** Fields that belong to the same concept
   (e.g. "positioning", "image lifecycle", "progress counters") share a block
   with no blank lines between them.

3. **`created_at` / `updated_at` are the last block.** Always grouped together
   at the end, with a blank line before them.

4. **A struct with ≤4 fields** may use blank-line grouping if the fields
   naturally split into clear groups; otherwise it is acceptable to leave them
   contiguous. Use judgment — if one or two fields logically stand apart,
   separate them.

5. **Data-layer DTOs** (`data/*.rs`) mirror the model grouping, omitting fields
   that don't appear in the DTO (e.g. `image_version` is usually dropped from
   the DTO; the remaining image fields still form their own group).

## Concrete examples

### PageInfo (model) — ✅ Correct

```rust
pub struct PageInfo {
    pub id: String,

    pub chapter_id: String,
    pub index: i32,

    pub image_key: Option<String>,
    pub image_uploaded: bool,
    pub image_version: i64,

    pub total_unit_count: i32,
    pub translated_unit_count: i32,
    pub proofread_unit_count: i32,

    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}
```

### UserInfo (model) — ✅ Correct (reference)

```rust
pub struct UserInfo {
    pub id: String,

    pub qid: String,
    pub nickname: String,

    pub avatar_key: Option<String>,
    pub avatar_uploaded: bool,
    pub avatar_version: i64,

    pub is_sadmin: bool,

    pub last_active_at: OffsetDateTime,

    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}
```

### ChapterInfo (model) — ✅ Correct (reference)

```rust
pub struct ChapterInfo {
    pub id: String,
    pub comic_id: String,

    pub is_pinned: bool,
    pub index: i32,
    pub subtitle: String,

    pub page_count: i32,
    // ... more counters

    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}
```

### PageInfoVal (data) — ✅ Correct

```rust
pub struct PageInfoVal {
    pub id: String,

    pub chapter_id: String,
    pub index: i32,

    pub image_url: Option<String>,
    pub image_uploaded: bool,

    pub total_unit_count: i32,
    pub translated_unit_count: i32,
    pub proofread_unit_count: i32,

    pub created_at: i64,
    pub updated_at: i64,
}
```

### UserInfoVal (data) — ✅ Correct (reference)

```rust
pub struct UserInfoVal {
    pub id: String,

    pub nickname: String,
    pub qid: String,

    pub avatar_url: Option<String>,
    pub is_sadmin: bool,
    pub last_active_at: i64,

    pub created_at: i64,
    pub updated_at: i64,
}
```

## ❌ Common mistakes

All fields jammed together with no blank lines:

```rust
// WRONG
pub struct PageInfo {
    pub id: String,
    pub chapter_id: String,
    pub index: i32,
    pub image_key: Option<String>,
    pub image_uploaded: bool,
    pub image_version: i64,
    pub total_unit_count: i32,
    pub translated_unit_count: i32,
    pub proofread_unit_count: i32,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}
```

## When to apply

- When reviewing or writing `model/*.rs` or `data/*.rs` files.
- When the user says "missing blank lines", "add blank lines between fields",
  "fix struct formatting", "field grouping", or similar.
- Any time a struct has 5+ contiguous fields without blank-line separation.

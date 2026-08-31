---
name: struct-field-blank-lines
description: "Enforces blank-line-separated logical field groups in Rust struct fields across the entire workspace. Use whenever creating, changing, moving, or reviewing Rust structs; all structs with 5 or more fields must group related fields with blank lines, following the codebase convention."
---

# Struct Field Blank-Line Grouping

Rust structs throughout this workspace must group their fields into logical
blocks separated by blank lines. This applies to any struct with 5 or more
fields. Smaller structs may omit grouping unless their fields naturally split
into clear groups.

## Rules

1. **`id` stands alone.** Always follow it with a blank line when other fields
   remain. It is the entity identifier and is never grouped with other fields.

2. **Group fields by logical concern.** Fields that belong to the same concept
   share a block with no blank lines between them. Separate unrelated concepts
   with one blank line.

3. **`created_at` / `updated_at` are the last block.** Keep them together at
   the end, with a blank line before them when preceding fields exist.

4. **Small structs use judgment.** Structs with four or fewer fields may omit
   grouping when the fields form one coherent concept. Separate fields that
   clearly belong to different concerns.

5. **Data-layer DTOs mirror model grouping.** Omit fields that do not appear in
   the DTO, while preserving the remaining logical groups.

## Scope and exceptions

- Apply this rule to named-field structs in every workspace crate, including
  nested modules, macro implementation structs, generated source templates,
  and test-only structs. Review generated output when a macro emits a struct.
- Do not force semantic groups into tuple structs, unit structs, struct
  literals, enum variant field lists, or unrelated structs outside these
  layers.
- Preserve documentation comments with the field they describe; place the
  blank line between field groups, not between a field and its comment.
- Do not reorder fields solely to create groups. Follow the established domain
  order unless the task explicitly changes the data contract.
- When a struct's grouping is ambiguous, inspect the corresponding model,
  DTO, conversion, and nearby sibling types before choosing boundaries.

## Reference shape

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

## Review procedure

When reviewing a change:

1. Identify every named-field struct added or modified anywhere in the
   workspace.
2. For each struct with five or more fields, identify its semantic groups from
   field names, types, corresponding projections, and nearby conventions.
3. Check that `id`, lifecycle timestamps, identity fields, image fields,
   positioning fields, and progress/counter fields are separated where they
   represent distinct concerns.
4. Add or remove only blank lines needed to express those groups; do not make
   unrelated formatting or field-order changes.
5. Report any ambiguous grouping for human review instead of inventing a new
   domain convention.

## Trigger phrases

Use this skill for requests involving struct field grouping, semantic field
layout, blank lines between struct fields, field-formatting cleanup, or review
of model/data struct changes—even when the user does not name this skill.

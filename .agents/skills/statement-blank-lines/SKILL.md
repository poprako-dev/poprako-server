---
name: statement-blank-lines
description: "Enforces blank-line separation between every statement in Rust function bodies — `let` bindings, function calls, `if let`/`if` conditionals, final `Ok(…)`/return expressions, and between adjacent `#[async_trait] impl` blocks. Detects and fixes missing blank-line spacing across the entire Rust codebase."
---

# Statement Blank-Line Spacing

Every **Rust function body** across the entire codebase must have a blank line
between **every two statements**. This rule also applies between consecutive
`#[async_trait] impl` blocks.

## Motivation

Without blank lines, consecutive statements blur together — `let now = …; let
aspect = …; db.update(…); Ok(())` is unreadable when all contiguous. A blank
line between each statement makes the function structure explicit: each step
(setup → build → execute → return) is its own paragraph.

## Rules

1. **Every statement gets its own paragraph.** A `let` binding, a function call
   ending in `?` or `;`, an `if`/`if let` conditional — each is followed by a
   blank line before the next statement.
2. **The final `Ok(…)` / return expression** always has a blank line before it,
   even if it is the only statement following a `let` chain.
3. **Multi-line method chains** (e.g. `db.update(…).set(…).execute(…).await
   .map_err(diesel)?;`) count as one statement — the blank line goes after the
   semicolon, not between chain links.
4. **Consecutive `#[async_trait] impl` blocks** — each block is separated by a
   blank line from the next.
5. **Exception: a function with exactly one statement** needs no blank line
   (there is nothing to separate). This covers thin delegation wrappers like
   `get_by_id_tx(conn, id).await` or `submit_query!(…)` in impl blocks.

## ✅ Correct examples

```rust
fn format_user(user: &User) -> String {
    let display_name = user.nickname.trim();

    let formatted = format!("{} <{}>", display_name, user.email);

    formatted
}
```

```rust
fn calculate_offset(page: u32, size: u32) -> u64 {
    let page = page.max(1);

    let offset = ((page - 1) * size) as u64;

    offset
}
```

```rust
async fn create_comic(conn: &mut AsyncPgConnection, form: &ComicForm) -> RegularResult<ComicInfo> {
    let entry = ComicEntry::from(form);

    let row: ComicRow = diesel::insert_into(t_comic)
        .values(&entry)
        .returning(ComicRow::as_returning())
        .get_result(conn)
        .await
        .map_err(diesel)?;

    Ok(row.into())
}
```

### Impl blocks — ✅ Correct

```rust
#[async_trait]
impl<'a> Execute<GetInfoById<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(&self, s: &GetInfoById<'a>) -> RegularResult<ComicInfo> {
        submit_query!(self.shared, get_comic_by_id, s.id)
    }
}

#[async_trait]
impl<'a> Execute<ListInfos<'a>> for RdbRepo {
    type Error = RegularError;

    async fn execute(&self, s: &ListInfos<'a>) -> RegularResult<Vec<ComicInfo>> {
        submit_query!(self.shared, list_comics, s.spec)
    }
}
```

## ❌ Common mistakes

```rust
// WRONG — statements jammed together
fn update_comic(update: &ComicInfoUpdate) -> RegularResult<()> {
    let now = OffsetDateTime::now_utc();
    let aspect = ComicAspect::new(now).title(&update.title);
    db.update(t_comic.filter(f_id.eq(update.id.as_str())))
        .set(&aspect)
        .execute(conn)
        .await
        .map_err(diesel)?;
    Ok(())
}
```

```rust
// WRONG — impl blocks not separated
#[async_trait]
impl<'a> Execute<Create<'a>> for RdbRepo {
    type Error = RegularError;
    async fn execute(&self, s: &Create<'a>) -> RegularResult<ComicInfo> {
        submit_query!(self.shared, create_comic, s.form)
    }
}
#[async_trait]
impl<'a> Execute<GetInfoById<'a>> for RdbRepo {
    type Error = RegularError;
    async fn execute(&self, s: &GetInfoById<'a>) -> RegularResult<ComicInfo> {
        submit_query!(self.shared, get_comic_by_id, s.id)
    }
}
```

## When to apply

- When writing, reviewing, or linting any `.rs` file in the project.
- When the user says "missing blank lines", "add blank lines between statements",
  "code is clumped together", "every two statements must have a blank line",
  "readability is bad", or similar.
- Any time a Rust function body has two or more consecutive statements without
  a blank line between them.

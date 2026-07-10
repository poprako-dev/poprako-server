---
name: statement-blank-lines
description: "Enforces Rust statement spacing after cargo fmt: exactly two one-line statements in one block stay adjacent; multi-line statement pairs and longer statement sequences use blank-line spacing."
---

# Statement Blank-Line Spacing

Rust statement spacing is based on the formatted physical line shape:

- If a block has exactly two statements and those statements occupy exactly two
  formatted lines total, keep them adjacent with no blank line.
- If a block has exactly two statements but either statement spans multiple
  formatted lines, insert a blank line between them.
- If a block has three or more statements, separate statement paragraphs with
  blank lines.
- This rule also applies between consecutive `#[async_trait] impl` blocks.

## Motivation

Short two-line blocks should stay compact. Multi-line statements and longer
statement sequences need spacing so each step remains visually distinct after
`cargo fmt`.

## Rules

1. **Exactly two one-line statements stay adjacent.** Do not put a blank line
   between them.
2. **Exactly two statements need spacing when formatted length exceeds two
   lines.** If either statement spans multiple lines after `cargo fmt`, put a
   blank line between the two statements.
3. **Three or more statements get paragraph spacing.** A `let` binding, a
   function call ending in `?` or `;`, an `if`/`if let` conditional — each
   statement paragraph is separated from the next by a blank line.
4. **The final `Ok(…)` / return expression** has a blank line before it when it
   follows a multi-line statement or when the block contains three or more
   statements.
5. **Multi-line method chains** (e.g. `db.update(…).set(…).execute(…).await
   .map_err(diesel)?;`) count as one statement — the blank line goes after the
   semicolon, not between chain links.
6. **Consecutive `#[async_trait] impl` blocks** — each block is separated by a
   blank line from the next.
7. **A function with exactly one statement** needs no blank line. This covers
   thin delegation wrappers like
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
fn resolve_team_id(workset_info: &WorksetInfo) -> RegularResult<String> {
    let team_id = workset_info.team_id.clone();
    accept(team_id)
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
// WRONG — three statements jammed together
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
// WRONG — two one-line statements split by a blank line
fn resolve_team_id(workset_info: &WorksetInfo) -> RegularResult<String> {
    let team_id = workset_info.team_id.clone();

    accept(team_id)
}
```

```rust
// WRONG — formatted block has more than two statement lines and needs spacing
async fn get_info(repo: &Repo, id: &str) -> RegularResult<Info> {
    let info = repo
        .execute(&Step::get_info_by_id(id))
        .await?;
    Ok(info)
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
  "code is clumped together", "readability is bad", or similar.
- Any time a formatted block has exactly two one-line statements separated by a
  blank line.
- Any time a formatted block has exactly two statements spanning more than two
  total lines without a blank line between them.
- Any time a Rust function body has three or more consecutive statements
  without blank-line spacing.

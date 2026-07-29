# Write Model 构造规范

在 CQRS 中所有的写模型的 mod。

## 对标准 CRUD 等价模型进行迁移时的命名规范

### create => XxxEntry：UserEntry、ComicEntry

例：

```rust
pub struct UserEntry {
    pub id: String,
    pub nickname: String,
}
```

### update(put) => XxxRepl：UserRepl、ComicRepl

put 通常语义表现为所有字段必须强制更新。比如 Option::is_none 的字段要求数据库必须更新为 NULL，而不是保持不变。

例：

```rust
pub enum UserRepl {
    Info {
        pub id: String,
        pub nickname: String,
    },
    Creds {
        pub id: String,
        pub password_hash: String,
    }
}
```

### update(patch) => XxxAspect

patch 通常语义表现为 Option::is_none 的字段会省略而不更新。

例：

```rust
pub struct ComicAspect {
    pub id: String,
    pub title: Option<String>,
    pub description: Option<String>,
}
```

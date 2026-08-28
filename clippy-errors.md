# Clippy 错误清单

## 当前状态

本文件由当前代码重新检查后整理，检查命令与 CI 最终 Clippy 步骤一致：

```sh
cargo clippy --workspace \
  --features rdb,prom_impl,repo_impl,swagger \
  --lib --bins -- -D warnings
```

| 检查项 | 状态 |
| --- | --- |
| `cargo fmt --all --check` | 通过 |
| `cargo check --all-features` | 通过 |
| `cargo test -p poprako-server` | 最近一次通过（404/404） |
| `sh linters-extra/run-check.sh` | 本次未验证：沙箱无法访问 `.uv-cache` |
| CI 最终 Clippy | 失败：Cargo 汇总 18 个错误 |

Clippy 输出中可逐项定位的主诊断为 18 条；Cargo 最后的汇总也显示为 18 个错误。表中数量用于定位规则分布。

## 规则分布

| 规则 | 数量 | 代表位置 | 说明 | 建议处理 |
| --- | ---: | --- | --- | --- |
| `struct-field-names` | 2 | `src/model/read/proj/unit.rs:68` | 非 RDB model 的字段统一带有相同后缀 | 业务 model 仍需单独判断，不能因为 RDB entity 的命名约束而全局放宽 |
| `too-many-lines` | 18 | `src/part_impl/repo/rdb_impl/comic_archive.rs:142` | 函数超过 100 行 | 按职责拆分；仅在确有必要时按模块拆分，不用随意加 lint 豁免 |

## 处理边界

- `struct-field-names` 和 `option-option` 已在需要的 RDB entity 模块局部允许：`src/part_impl/repo/rdb_impl/entity.rs` 同时允许两项，`src/part_impl/prom/rdb_impl/entity.rs` 允许 `struct-field-names`。这是数据库字段映射边界的局部例外，不是全局关闭规则。
- `ref-option-ref` 仅在 `src/part_impl/repo/rdb_impl/entity/{comic,page,unit,workset}.rs` 文件范围内允许。12 条诊断均来自 Diesel `AsChangeset` 派生代码；相关嵌套 `Option` 用于区分未更新、清空和设置，不能拍平。
- 允许后，RDB entity 中的 `struct-field-names` 和 `option-option` 诊断已从 Clippy 输出中消失；剩余 `struct-field-names` 是 `src/model/read/proj/unit.rs` 的 2 项，`option-option` 和 `ref-option-ref` 已无剩余项。
- `future-not-send` 已在根 `Cargo.toml` 的 workspace Clippy 配置中设为 `allow`，因此当前清单不再统计这 15 项。
- `unnecessary-wraps` 已在根 `Cargo.toml` 的 workspace Clippy 配置中设为 `allow`；HTTP 成功边界需要保留 `Result` 包装，因此不改动实现。
- RDB entity 中的整数转换错误属于持久化数据无效，应使用 `try_from` 和 `?` 传播为 `BaseError`；不能先使用 `as` 丢失符号或范围信息。
- `cast-possible-truncation` 在 worker 哈希映射处已改为 `BaseRest<usize>`，通过受检转换和 `?` 传播内部边界错误。
- `UnitCountMetrics::calc_delta` 的 6 个 `expect-used` 已改为 `BaseRest<UnitCountDelta>`，通过受检转换和 `?` 传播内部不变量错误。
- `needless-pass-by-value` 的 14 项和 `trivially-copy-pass-by-ref` 的 13 项已全部修复；前者改为借用或 `&str`，后者将 `Copy` 值对象的方法接收者改为按值传递。
- `match-same-arms`、`missing-errors-doc`、`option-if-let-else` 已全部消除。
- 这些结果来自 `--lib --bins`，不会把 test/benchmark 目标作为 Clippy lint 目标；它们仍由 `cargo check --workspace --all-targets --all-features` 编译检查。
- 不修改 `linters` submodule，也不把新规则写进 submodule。`replacement` 词的禁止规则应放在 submodule 外的配置中；本文件只记录 Clippy 结果。
- RDB entity/schema 是有符号数据库适配边界；除该边界外，业务层按项目约定优先使用 `usize` 或 `u32`，不添加无意义的中间变量或类型转换。
- 第一批直接修复项（包括 5 个 `needless-borrow`、2 个 `or-fun-call`、2 个 `useless-conversion`、1 个 `single-match-else`、1 个 `too-long-first-doc-paragraph`、1 个 `missing-const-for-fn`、1 个 `items-after-statements` 和 1 个 `assigning-clones`）已全部归零。

## 验证记录

最后一次检查中，格式化、全特性编译、rust-style-lint 和 linters-extra 通过；生产目标 Clippy 仍失败，当前错误汇总为 18，剩余全部为 `too-many-lines`。待清单归零后再以 `sh scripts/ci-check.sh` 验证 CI 全部通过。

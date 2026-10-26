# PopRaKo 分配器与常驻内存评估

2026 年 10 月 18 日。结论：这台 2C2G 主机值得约束内存预算，但目前更明确的优化对象是业务缓存与密码计算并发。直接切换到 `tikv-jemallocator` 预计增大发布二进制，无法保证降低 RSS；是否采用应以相同 Linux musl 环境的负载对比为依据。

## 二进制与 RSS 的预期影响

`tikv-jemalloc-sys` 的通常构建路径静态链接 jemalloc，因此会将分配器代码加入程序。预计二进制增大；具体增加多少取决于版本、特性、平台与链接结果，不能从依赖名称计算。默认符号前缀隔离也意味着 Rust 全局分配器的替换不等于替换全部 C 库的 `malloc`。默认特性仅提供后台线程运行时支持，不会自动开启后台清理。[TiKV 构建源码](https://github.com/tikv/jemallocator/blob/main/jemalloc-sys/build.rs)、[0.7.0 特性](https://docs.rs/crate/tikv-jemallocator/0.7.0/features)。

| 阶段 | 可以预期的机制 | 对本服务的判断 |
| --- | --- | --- |
| 刚启动且没有业务 | 分配器需要自己的元数据、arena 与线程缓存 | 不应预期自动降低空载 RSS，可能升高 |
| 持续负载 | 内存布局、碎片与缓存复用改变 | 峰值 RSS 与吞吐的方向均需实测 |
| 负载结束后 | 释放对象与归还物理页是不同步骤 | 调整清理策略可能改善回落，仍然活着的业务缓存不会因此消失 |

jemalloc 的 arena 降低分配锁竞争，同时产生独立的管理开销与碎片；线程缓存用内存换取更快分配。低并发时减少 arena、降低 tcache 范围、缩短 decay 是官方列出的内存优化方向，代价可能是更多 CPU 与竞争。后台线程可以避免应用空闲导致的清理延迟。这些机制支持进行实验，不能证明默认 jemalloc 比当前分配器更省内存。[官方调优说明](https://github.com/jemalloc/jemalloc/blob/dev/TUNING.md)。

jemalloc 文档中的 `stats.allocated`、`stats.active`、`stats.resident` 分别描述应用分配、活动页、分配器管理的潜在驻留页；它们不等于整个进程 RSS。`retained` 主要描述保留的虚拟地址空间，也不能直接计入物理内存。arena 的最大数量不代表启动时全部填满；映射或栈的虚拟尺寸同样不能直接当作 RSS。[jemalloc 手册](https://jemalloc.net/jemalloc.3.html)。

## 当前构建基线

[Dockerfile](../Dockerfile) 第 1、63 行采用 Alpine 构建与运行镜像，应按 musl 基线评估。不能套用其他项目从 glibc 切换 jemalloc 的节省比例。glibc 有自己的 arena、tcache 与 trim 配置，它们不是 musl 或 jemalloc 的配置接口。[glibc 官方配置说明](https://sourceware.org/glibc/manual/latest/html_node/Memory-Allocation-Tunables.html)。Rust 的 `System` 在 Unix 使用 `malloc` 及相关函数；全局分配器替换作用于 Rust 的默认堆分配，部分标准库内部仍可直接使用 `System`。[Rust 分配器文档](https://doc.rust-lang.org/std/alloc/index.html)、[System 文档](https://doc.rust-lang.org/std/alloc/struct.System.html)。

[Cargo.toml](../Cargo.toml) 第 14 行起的 release 配置已使用符号剥离、thin LTO 和 `codegen-units = 1`；Dockerfile 第 8 行覆盖为 16。比较发布体积时必须沿用同一条生产构建路径，不能用本地默认 release 或 macOS 构建代替。

## 常驻业务内存

“暂时没有请求”与“刚启动、没有业务数据”不同。当前代码存在会保留业务数据的结构，不能把长期空闲阶段解释为几乎没有业务层内存。

- [UnitSaveCache](../src/part_impl/cache/mem_impl/unit_save.rs) 使用 16 段缓存，默认有 256 MiB 估算权重预算、500,000 个 token 预算和 30 天 TTL；成功重试不延长 TTL。它保存成功提交的单元 ID 映射。过期条目无法命中，但空闲缓存可能直到后续维护才释放分配。估算权重不是预分配或精确 RSS 上限。当前契约见 [Unit Save API](unit-save-api.md)。
- [OnlineUserCache](../src/part_impl/cache/mem_impl/online_user.rs) 第 20 行设置十分钟租期，第 66 行只在列出对应团队时删除过期记录。租期到期不等于立即释放内存；长期没有再次查询的团队仍可留有记录。
- [密码计算](../src/complex/user.rs) 第 32、58 行通过 `spawn_blocking` 执行，第 95 行使用默认 Argon2 参数。Argon2 0.5.3 的默认内存参数为 `19 * 1024` KiB，即每个运行中的默认参数计算约 19 MiB 工作区；验证历史 hash 时参数取决于该 hash。32 个这样的计算仅工作区就约 608 MiB，这是并发估算，不是测量结果。[Argon2 参数源码](https://docs.rs/argon2/0.5.3/src/argon2/params.rs.html)。

Tokio 的 blocking 线程上限默认是 512，按需创建，不能当成启动后已有 512 个线程，也不能当成适合密码计算的并发预算。密码计算需要独立的在途工作上限；整体请求速率限制不能直接给出密码工作区上限。[Tokio Builder](https://docs.rs/tokio/latest/tokio/runtime/struct.Builder.html#method.max_blocking_threads)。

[现有部署审计](diesel-usage-audit.md) 第 126 行记录生产主机为 2C2G，共享数据库连接池最多四条。基于这一预算，应优先核对保存 token 缓存的实际必要容量与保留时间、在线租约清理方式，以及密码计算并发，而不是首先以分配器替换解决所有 RSS。

## 对比方法与决策条件

后续在相同 CPU、容器内存限制、Linux musl、编译配置与数据集下，对比当前分配器、默认 jemalloc，以及一组经过核实生效的低内存 jemalloc 配置。使用正式 CI 构建产物；本次评估不要求执行本地 release 或镜像构建。

分别记录二进制文件字节数、启动稳定后的空载 RSS、相同流量下峰值与稳态 RSS、停止流量后 10 秒及 60 秒的 RSS，同时记录 CPU、吞吐和 p95/p99 延迟。覆盖密码认证、单元批量保存、普通列表请求与缓存逐渐填充；每组重复执行，避免把不同缓存热度当作分配器收益。

Linux 的 `/proc/PID/smaps_rollup` 提供进程各映射的汇总 RSS/PSS，`smaps` 可区分文件映射与匿名内存。进程数据还应与容器、数据库及整机可用内存分开观察；只看一个 RSS 数字无法区分业务对象、分配器留存、代码映射和其他组件。[Linux proc 文档](https://docs.kernel.org/filesystems/proc.html)。

若主要增长来自活跃缓存或并发工作区，先调整对应预算；若活跃对象已经减少而匿名 RSS 长期不回落，再比较 allocator 与清理配置。最终以实际节省的 MiB 和可接受的延迟、CPU 成本决定是否切换，不预先承诺比例。

# 白杨子 S · PopRaKo Server

白杨子的服务端，使用 Rust 构建，为客户端提供项目管理、团队协作与数据存储支持。
具体功能请参考 [白杨子 W](https://github.com/poprako-dev/poprako-web)。

## 本地运行

需要 Rust 1.95、PostgreSQL、`diesel_cli` 和 OSS（Cloudflare R2 或阿里云 OSS）。

```sh
cp .env.example .env
```

填写 `.env` 中的数据库连接、JWT 密钥和 OSS 配置，开发时使用独立的开发数据库。
随后初始化数据库并启动服务：

```sh
diesel database setup
cargo run -p poprako-server --bin poprako-server
```

默认监听 `127.0.0.1:8888`，接口前缀为 `/api/v1`。

## 配置

- [`.env.example`](.env.example)：数据库连接、JWT、OSS 等环境变量。
- [`app_config.toml`](app_config.toml)：监听地址、跨域来源、上传限制等运行参数，各项均有注释。

当前启动入口接入 R2，接入阿里云 OSS 时需调整对应配置与实现。

缺少 TOML 文件或省略配置项时使用默认值，未知字段和无效值会导致启动失败。
配置修改后需重启服务；本地联调时，将 `http.cors_origin` 设为客户端的实际访问来源。

## 部署

生产部署通过 GitHub Actions 完成，需要准备 Docker 主机、PostgreSQL 和 OSS。
按 [部署变量模板](deploy/poprako-server/github-production-secrets.env.example)
配置 GitHub `production` 环境中的 Secrets；运行参数见
[生产配置](deploy/poprako-server/app_config.toml)。

代码合入 `main` 后，工作流完成检查和镜像构建，经生产环境审批后部署。
数据库迁移由部署流程执行，应用启动时不会自动迁移。
已有数据库升级请先查看 [升级说明](docs/database-upgrade-support.md)，发布规则见 [RELEASE.md](RELEASE.md)。

## 开发

常用检查命令（需安装 `just`）：

```sh
just fmt-check
just check
just clippy
sh scripts/ci-test.sh
```

测试需要 Docker。完整贡献与验证流程见 [贡献指南](CONTRIBUTING.md)，
HTTP 集成测试见 [测试说明](tests/integration-tests/TESTCASES.md)，
接口定义见 [OpenAPI](docs/swagger.json)。

问题反馈见 [SUPPORT.md](SUPPORT.md)，安全问题请按 [安全策略](SECURITY.md) 报告。

## 许可证

Copyright (c) 2026 PopRaKo contributors.

采用 GNU AGPL v3（`AGPL-3.0-only`），详见 [LICENSE](LICENSE)。

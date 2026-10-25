# poprako-server

PopRaKo manga translation project management backend, built with Rust, Axum,
Diesel, and PostgreSQL.

The service provides APIs for teams, comics, chapters, pages, translation
units, assignments, archives, and termbases. It is currently under active
development.

## Requirements

- Rust 1.95 with Rust 2024 edition support
- PostgreSQL and `diesel_cli`
- Cloudflare R2 credentials

Docker is additionally required for container builds. Node.js 26 and standalone pnpm 12.11.2 are required for
the TypeScript HTTP integration tests. Install both independently (without npm):

```sh
sh scripts/setup-js.sh
. "$HOME/.local/share/poprako-js/env"
cd tests/integration-tests
pnpm install --frozen-lockfile
pnpm run check
```

The runtime is pinned in `.node-version`; pnpm is pinned in the test package and
installer. Node.js 26 is currently Current, with LTS targeted for 2026-10-28.
CI uses the same installer. Add the activation command to your shell startup
file for persistent use. See the test inventory for API execution commands.

## Run locally

Create the local environment file:

```sh
cp .env.example .env
```

Set the PostgreSQL, JWT, and Cloudflare R2 values in `.env`, then initialize the
database and start the server:

```sh
diesel database setup
cargo run -p poprako-server --bin poprako-server
```

The API is served under `/api/v1` and listens on the address configured in
`app_config.toml`.

Application configuration is loaded once at startup. Every `AppConfig` field
has a default: a missing file, empty file, missing section, or omitted field
uses that default. Explicit values override defaults; invalid values, malformed
TOML, and read failures other than a missing file prevent startup. Restart the
server after changing configuration. Existing environment variables remain
required where applicable and are loaded independently.

Defaults are `127.0.0.1:8888`, CORS origin `https://poprako.com`, and one shared
HTTP quota of 20 requests/second with a burst of 80 per application instance.
`http.cors_origin` is a single canonical HTTP(S) origin without a trailing slash,
credentials, path, query, or fragment. Allowed methods and headers stay fixed.
Image limits in MiB are 1 for either avatar, 2 for comic covers, 25 for page
images, and 512 for chapter artwork. The local file overrides page images to
30 MiB; the deployment file overrides the listener to `0.0.0.0`.
Image and artwork limit keys consistently use the explicit `_mib` suffix.
Unknown configuration keys, including obsolete names, are rejected.

`effect.queue_capacity` defaults to 1024 events. `sched.shutdown_grace_secs`
defaults to 30 seconds and permits zero for immediate cancellation of pending
workers. `subtree_delete` defaults to concurrency 2, `timeout_secs = 300`,
`idle_interval_secs = 5`, and `retry_interval_secs = 5`. These task durations,
queue capacity, concurrency, and rate-limit values must be positive and fit
their runtime representation. See the checked-in TOML for all configuration
keys. Defaults are maintained in the corresponding Rust configuration types.

`cache` defaults to a 256 MiB estimated Unit-save budget, 500000 retained
save tokens, and `online_user_ttl_secs = 600`. The token budget is translated
into a minimum cache-entry weight; it is not a separate token store. `repo`
defaults to `subtree_sweep_batch_size = 64` eligible Comics or Worksets and
`unit_order_query_batch_size = 512` Unit links, with one derived sentinel row.
`invitation.member_ttl_secs = 432000` and `assignment_ttl_secs = 259200`
control cleanup delays for newly created pending invitations. Changing them
does not reschedule existing tasks; cleanup may run later according to worker
availability. All these budgets, batch sizes, and durations must be positive.

`comic_archive_export` defaults to an 8 MiB stored-payload batch budget, a
256 MiB serialized month-map temporary file, and one active export. The slot
remains occupied until transfer finishes or the response is dropped. Selected
months and stored JSON strings are exported completely; exhausted capacity or
budgets return 503. The file is completed before HTTP transmission, so slow
clients do not hold database connections. These are application budgets rather
than an exact process RSS bound.

User profiles support reads, updates and avatar operations. Account deletion
is outside the 1.0.0 interface; `DELETE /api/v1/users/{user_id}` returns 405.

## Common commands

```sh
# Apply pending migrations
diesel migration run

# Run all workspace tests, including PostgreSQL regressions (requires Docker)
sh scripts/ci-test.sh

# Run the complete basic CI entry point
sh scripts/ci-check.sh

# Run canonical Rust checks individually
just fmt-check
just check
just clippy

# Generate docs/swagger.json
cargo run -p poprako-swagger > docs/swagger.json
```

The `justfile` provides optional shortcuts for local development. It is not a
project requirement and is not used as the CI/CD interface.

The HTTP integration suite requires a dedicated disposable database:

```sh
INTEGRATION_DATABASE_URL=postgres://USER:PASSWORD@localhost:5432/db_poprako_integration \
  scripts/api-integration-test.sh
```

The integration script drops its target database after the run. Do not point it
at a database containing data that must be preserved.

## Documentation

- [Contributing](CONTRIBUTING.md)
- [Security policy](SECURITY.md)
- [Release policy](RELEASE.md)
- [HTTP integration test inventory](tests/integration-tests/TESTCASES.md)
- [1.0.0 release preparation](docs/1.0.0-release-preparation.md)
- [Agent and project conventions](AGENTS.md)
- [OpenAPI specification](docs/swagger.json)

## License

Copyright (c) 2026 PopRaKo contributors.

Licensed under the GNU Affero General Public License, version 3 only
(`AGPL-3.0-only`). See [LICENSE](LICENSE) for the full license text.

# Rust backend

Axum HTTP application using Diesel 2.3, PostgreSQL/PostGIS, and an r2d2 connection pool. Synchronous ORM queries run on Tokio blocking threads. Default address: `127.0.0.1:8000` (OTP continues to use port 8080).

## Run

Requires Rust 1.86+ and a PostgreSQL database that supports PostGIS. Native libpq and OpenSSL are bundled at build time, so a C compiler, CMake, make and Perl must be available; a separate libpq installation or Diesel CLI is not needed.

From the repository root, start the existing database service if needed:

```sh
docker compose up -d db
cd backend
cp .env.example .env
```

Edit `backend/.env`: set `DATABASE_URL` to your database credentials and the published database port from the root `.env` (`POSTGRES_PORT`, default 5432). URL-encode special characters in credentials. Use another free database port if 5432 is already occupied.

```sh
cargo run -- migrate-status
cargo run -- migrate
cargo run -- serve
```

`serve` is the default command. It refuses to start with pending migrations. Migrations are an explicit step and are embedded into the binary; `build.rs` ensures SQL edits trigger recompilation. Never log or commit `.env`. `DB_POOL_SIZE` defaults to 8; `BIND_ADDRESS` defaults to `127.0.0.1:8000`.

## Migrations

| Order | Source | Tables |
| --- | --- | --- |
| 1 | `scripts/sql/insert_zsj.sql` | `ZSJ`, PostGIS extension |
| 2 | `scripts/sql/insert_skoly.sql` | `stredni_skoly` |
| 3 | `scripts/insert_obory_nabidka_oboru.sql` | `OBORY`, `NABIDKA_OBORU` |
| 4 | `scripts/sql/insert_zamestnavatele.sql` | `ZAMESTNAVATELE` |
| 5 | `scripts/sql/insert_profesni_skupiny.sql` | `PROFESNI_SKUPINY` |
| 6 | `scripts/sql/insert_poptavka_profesi.sql` | `POPTAVKA_PROFESI` |
| 7 | `scripts/sql/insert_obor_profese.sql` | `OBOR_PROFESE` |

The migrations preserve the original schema, constraints, indexes and seed data. Only outer `BEGIN` / `COMMIT` statements are removed because Diesel manages each migration transaction. The program/offer import is a prerequisite for `OBOR_PROFESE` and therefore included despite being outside `scripts/sql`. The SELECT examples and JSON verification reports are not migrations. The separate demographic import is not part of this backend's initial migrations.

Use a fresh database for initial setup. The existing scripts replace demand and program/profession mapping snapshots and remove employers absent from the snapshot; migrating an already populated database will retain that behavior. `down.sql` drops the corresponding tables and data in reverse dependency order and retains the shared PostGIS extension. Existing tables are not automatically marked as already migrated. Back up an existing database before adopting this migration history. Do not run standalone seed imports and migrations as competing initialization mechanisms.

After a migration has been applied, add a new migration for subsequent schema/data changes instead of editing its SQL. Diesel records completed versions in `__diesel_schema_migrations` and will not rerun them.

## OpenAPI contract and stubs

The root `openapi.yaml` is the public API contract. All nine GET operations are mounted under `http://localhost:8000/api/v1`:

- `/skoly`, `/skoly/{redizo}`
- `/student/skoly`, `/student/trasa`
- `/zsj`, `/obory`, `/obory/{kod}`, `/obory/{kod}/zamestnavatele`, `/simulace`

These handlers are **stubs**: valid requests return documented HTTP **501** with the `Chyba` error schema and `error.kod = neimplementovano`. Invalid parameters return **422** in the same error format. Parameters are validated directly against the YAML schemas, including required values, identifier patterns, enums and numeric bounds. Domain checks such as whether a school/scenario exists or whether a point lies within the region require the future business implementation.

`src/requests.rs` defines query structs for every operation, plus Redizo/KodOboru identifier newtypes and Scenar/Uroven/Forma/Format/Razeni enums. The `ContractQuery<T>` extractor validates raw values against OpenAPI, then deserializes them and applies defaults: `max_min = 120`, `scenar = rano`, `kandidatu = 5`, `razeni = nazev`, and `format = slovnik` on the corresponding endpoints. `stupen=H,M` becomes `Stupne(Vec<Stupen>)`; `signal=pretlak,spatna_dostupnost` becomes a SignalFilter list. Signals remain extensible according to their string schema; Scenar is now an enum allowing only rano. Forma defaults to den, and Uroven defaults to obec. The employer endpoint uses numeric Vhodnost (1 or 2, default 2) and jen_ss (default true). The school, program and employer detail routes use typed path identifiers separately from their query structs.

`src/types.rs` defines numeric NSP suitability values shared by requests and employer response models. The simulation dictionary is keyed by `jednotky` (4–6 digit area codes); GeoJSON map/simulation responses use territory levels and travel bands. Required nullable fields preserve explicit JSON null and reject missing keys.

`src/contract.rs` contains typed handler signatures and JSON/GeoJSON response wrappers. `src/dto.rs` contains public API structs independent of Diesel models. Implement each handler by naming its `ContractQuery(_params)` argument `ContractQuery(params)` and replacing its final unimplemented error with database/OTP logic returning the declared DTO; do not return the database model directly. Both simulation response formats have separate DTOs.

Swagger UI: `http://localhost:8000/docs`. The specification is served at `/openapi.yaml`; Swagger loads that URL instead of embedding a duplicate. Both files are embedded into the compiled backend and edits trigger rebuilding.

```sh
curl -i http://localhost:8000/api/v1/skoly
curl -i 'http://localhost:8000/api/v1/student/skoly?lat=50.2312&lon=12.8711'
cargo test --test openapi_contract
```

The fourteen contract tests run without PostgreSQL or OTP. They compare route/operation coverage and every success response media type; compare all response DTO properties and query fields, Rust field types, enums, required fields and query defaults with the specification; validate independent representative fixtures before and after Rust serialization; and exercise actual stub/error responses and documentation routes. Negative cases check missing fields, nullable/non-nullable values, identifiers, coordinate dimensions, dates, enums, dictionary keys and array limits. Numeric bounds, formats, patterns and nullability are checked by the YAML JSON Schema validator; plain Rust String/Vec types do not encode every value constraint. These tests verify structural contracts and representative payloads, not business calculations or correctness for every possible future response.

## Database inspection API

```sh
curl http://127.0.0.1:8000/health
curl 'http://127.0.0.1:8000/api/skoly?limit=100&offset=0'
curl http://127.0.0.1:8000/api/skoly/600008975
curl http://127.0.0.1:8000/api/zsj/000540
```

Read-only JSON list endpoints: `/api/skoly`, `/api/zsj`, `/api/obory`, `/api/nabidky`, `/api/zamestnavatele`, `/api/profesni-skupiny`, `/api/poptavka-profesi`, `/api/obor-profese`. Lists return arrays sorted by primary key, with `limit` (1–1000, default 100) and `offset` (nonnegative, default 0). School and ZSJ detail endpoints return 404 for unknown identifiers. Health verifies a database query; unavailable pool connections return 503. Invalid pagination returns 400.

Identifiers remain strings with leading zeros. Nullable database values remain JSON null. School decimal coordinates serialize as decimal strings to retain precision; ZSJ and employer float coordinates serialize as JSON numbers. ZSJ polygon boundaries are stored and indexed in PostGIS and represented by a custom Diesel SQL type, but omitted from the basic JSON model. Demand timestamps represent import time, following the original SQL.

## Verification

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
```

`cargo test` runs the entire suite, including the database integration test; no tests are ignored. Docker must be running. The database test automatically creates a unique temporary PostGIS container with an ephemeral localhost port and temporary storage, verifies migrations and API behavior, then removes the container even if an assertion fails. It never reads `DATABASE_URL` or `TEST_DATABASE_URL` and never connects to your project database. The first run may download the PostGIS image.

For contract tests alone, Docker is not needed:

```sh
cargo test --test openapi_contract
```

`diesel.toml` also supports an optional Diesel CLI for schema generation and migration operations. `Cargo.lock` is included for reproducible application dependencies.

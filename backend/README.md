# Rust backend

Axum HTTP application using Diesel 2.3, PostgreSQL/PostGIS, and an r2d2 connection pool. Synchronous ORM queries run on Tokio blocking threads. Default address: `127.0.0.1:8000` (OTP continues to use port 8080).

## Run with Docker Compose

From the repository root, copy `.env.example` to `.env` if you have not configured it yet, then run:

```sh
docker compose up -d backend
```

Compose builds the Rust application, starts PostgreSQL, waits for database health, and applies pending embedded migrations before serving the API. Rust and native build dependencies are provided by the image. Database data persists in the existing `postgres_data` volume. The container uses the root `.env` credentials; `backend/.env` is only for local Cargo runs and is not copied into the image. Passwords with URL-special characters work through libpq's separate credential environment variables.

Swagger UI: `http://localhost:8000/docs`. Set `BACKEND_PORT` in the root `.env` to use another host port; the container always listens on 8000. If a different database already occupies 5432, set `POSTGRES_PORT` to a free host port (for example, 5433). Containers connect to `db:5432` regardless of that host port. Only the backend and its database dependency start with this command; OTP is not needed for the implemented database reads.

```sh
docker compose logs -f backend
docker compose exec backend obor-backend migrate-status
# Rebuild after changing Rust code, migrations, OpenAPI, or Swagger:
docker compose up -d --build backend
```

The existing six analytical endpoints still return 501. Automatic migrations apply only to container startup; local Cargo runs retain the explicit migration step below.

## Run locally with Cargo

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

## OpenAPI endpoints

The root `openapi.yaml` is the public API contract. The server uses `contract::router` directly and exposes only its API routes, plus `/docs` and `/openapi.yaml` for documentation. All nine GET operations are mounted under `http://localhost:8000/api/v1`:

- `/skoly`, `/skoly/{redizo}`
- `/student/skoly`, `/student/trasa`
- `/zsj`, `/obory`, `/obory/{kod}`, `/obory/{kod}/zamestnavatele`, `/simulace`

Database reads are implemented for:

- `GET /skoly`: GeoJSON school points, filtered offerings (`obor`, CSV `stupen`, `forma`), and offering/capacity/application totals. Without `obor` or `stupen`, schools without matching offerings remain included. Municipality names and pressure indices are omitted until the supporting lookup/calculation is implemented.
- `GET /skoly/{redizo}`: school details and study offerings. Catchment data is still uncomputed (`spadovost: {}`); `max_min` and `scenar` do not yet affect this response.
- `GET /obory/{kod}/zamestnavatele`: profession mappings joined to demand and workplaces, filtered by `vhodnost` and `jen_ss`. Education categories are summed per profession/workplace without double-counting. Metadata includes distinct company/workplace counts, jobs, and workplaces without coordinates grouped by municipality. No mapping means `existuje: null`; a mapping with no matching demand means `existuje: false`. Workplace IDs are strings to retain integer precision.

The other six operations remain **stubs**: valid requests return documented HTTP **501** with the `Chyba` error schema and `error.kod = neimplementovano`. They require travel-time, demographic, bilance, or simulation logic. The read handlers are in `src/contract.rs`, with catalog queries in `src/catalog.rs`; synchronous Diesel work runs on blocking threads and does not call OTP.

Invalid parameters return **422** in the same error format. Parameters are validated directly against the YAML schemas, including required values, identifier patterns, enums and numeric bounds. Implemented detail routes return **404** for missing records. Database connection failures return **503**; query or required-data failures return **500**. Geographic/domain checks for analytical routes still require their business implementation.

`src/requests.rs` defines query structs for every operation, plus Redizo/KodOboru identifier newtypes and Scenar/Uroven/Forma/Format/Razeni enums. The `ContractQuery<T>` extractor validates raw values against OpenAPI, then deserializes them and applies defaults: `max_min = 120`, `scenar = rano`, `kandidatu = 5`, `razeni = nazev`, and `format = slovnik` on the corresponding endpoints. `stupen=H,M` becomes `Stupne(Vec<Stupen>)`; `signal=pretlak,spatna_dostupnost` becomes a SignalFilter list. Signals remain extensible according to their string schema; Scenar is now an enum allowing only rano. Forma defaults to den, and Uroven defaults to obec. The employer endpoint uses numeric Vhodnost (1 or 2, default 2) and jen_ss (default true). The school, program and employer detail routes use typed path identifiers separately from their query structs.

`src/types.rs` defines numeric NSP suitability values shared by requests and employer response models. The simulation dictionary is keyed by `jednotky` (4–6 digit area codes); GeoJSON map/simulation responses use territory levels and travel bands. Required nullable fields preserve explicit JSON null and reject missing keys.

`src/contract.rs` contains typed handler signatures and JSON/GeoJSON response wrappers. `src/dto.rs` contains public API structs independent of Diesel models. Implement each handler by naming its `ContractQuery(_params)` argument `ContractQuery(params)` and replacing its final unimplemented error with database/OTP logic returning the declared DTO; do not return the database model directly. Both simulation response formats have separate DTOs.

Swagger UI: `http://localhost:8000/docs`. The specification is served at `/openapi.yaml`; Swagger loads that URL instead of embedding a duplicate. Both files are embedded into the compiled backend and edits trigger rebuilding. When serving `swagger.html` as a static file, keep `openapi.yaml` in the same directory. When opening the HTML directly from disk, select `openapi.yaml` using the file picker; browsers block automatic local-file requests. Swagger UI assets still require an internet connection.

```sh
curl -i http://localhost:8000/api/v1/skoly
curl -i 'http://localhost:8000/api/v1/student/skoly?lat=50.2312&lon=12.8711'
cargo test --test openapi_contract
```

The fifteen contract tests run without PostgreSQL or OTP. They compare route/operation coverage and every success response media type; compare all response DTO properties and query fields, Rust field types, enums, required fields and query defaults with the specification; validate independent representative fixtures before and after Rust serialization; and exercise actual stub/error responses, database-unavailable responses, and documentation routes. The disposable-database test additionally validates real read responses against OpenAPI and checks catalog filters, counts, missing coordinates, mapping semantics, and required-data failures. Negative cases check missing fields, nullable/non-nullable values, identifiers, coordinate dimensions, dates, enums, dictionary keys and array limits. Numeric bounds, formats, patterns and nullability are checked by the YAML JSON Schema validator; plain Rust String/Vec types do not encode every value constraint. These tests verify structural contracts and representative payloads, not business calculations or correctness for every possible future response.

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

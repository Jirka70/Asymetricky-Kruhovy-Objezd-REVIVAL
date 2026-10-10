# Rust backend

Axum HTTP application using Diesel 2.3, PostgreSQL/PostGIS, and an r2d2 connection pool. Synchronous ORM queries run on Tokio blocking threads. Default address: `127.0.0.1:8000` (OTP continues to use port 8080).

## Run with Docker Compose

To start the complete application (frontend, backend and database) from the
repository root with the root `.env` configured:

```sh
docker compose up -d --build --wait
```

Frontend defaults to http://localhost:3000; API documentation is at
http://localhost:8000/docs. `FRONTEND_PORT`, `BACKEND_PORT` and `POSTGRES_PORT`
in the root `.env` override the host ports. OTP is optional under the `otp`
profile and is not started by the default command. The frontend reads schools,
programs, school details and employers through its Next.js proxy with a 24-hour
upstream cache. Geography, demographics and stored journeys still use the
bundled snapshot.

To run only the backend and database:

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

The remaining four analytical endpoints still return 501. Automatic migrations apply only to container startup; local Cargo runs retain the explicit migration step below.

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

`serve` is the default command. It refuses to start with pending migrations. Migrations are an explicit step and are embedded into the binary; `build.rs` ensures SQL edits trigger recompilation. After updating to include migrations 8–9, run `cargo run -- migrate` before local serving, or rebuild the Compose backend to apply them at container startup. Never log or commit `.env`. `DB_POOL_SIZE` defaults to 8; `BIND_ADDRESS` defaults to `127.0.0.1:8000`.

## Request logging

Every request logs `Request received` on arrival and `Response ready` after the handler
produces a response. Both INFO events include the same process-local `request_id`,
HTTP `method`, and `path`; the response event also includes `status` and `elapsed_ms`.
Timing covers request processing, including database waits and JSON serialization,
up to response readiness. Paths are logged without query strings. This also covers
documentation routes, validation errors, and 404/405 responses.

Logging is enabled by the default `RUST_LOG=obor_backend=info`. View it with
`docker compose logs -f backend`, or in the terminal running `cargo run -- serve`.
Example event fields (the normal formatter also includes timestamps and targets):

```text
INFO Request received request_id=1 method=GET path=/api/v1/zsj/seznam
INFO Response ready request_id=1 method=GET path=/api/v1/zsj/seznam status=200 elapsed_ms=42.7
```

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
| 8 | `scripts/insert_demografie_zsj.sql` | `DEMO_SKUPINA`, `DATA_DEMOGRAFIE_ZSJ`, `ZSJ.kod_obce` |
| 9 | `scripts/sql/insert_dojezdy.sql` | `DOJEZDOVE_DOBY` |

The first seven migrations preserve the original schema, constraints, indexes and seed data. Only outer `BEGIN` / `COMMIT` statements are removed because Diesel manages each migration transaction. The program/offer import is a prerequisite for `OBOR_PROFESE` and therefore included despite being outside `scripts/sql`. The SELECT examples and JSON verification reports are not migrations. Demographic and travel-time imports are embedded as migrations 8 and 9. An existing backend database with migrations 1–7 applied receives only the two new migrations.

The demographic seed includes 21 age groups and 17,619 estimates for 839 ZSJ in 2021 (279,103 people), plus municipality codes for aggregation. Its source checks preserve totals by ZSJ and municipality. These are modeled age estimates, not current observed population. The `10-14` group is `1300100014`; estimating a school-entry cohort will require dividing its population by five.

The travel-time seed includes all 28,526 ZSJ × school pairs for arrival slot `07:00-08:00`. Migration 9 converts the source's legacy integer ZSJ identifiers to six-character text codes and corrects the school foreign key to `stredni_skoly`. Times are minutes; NULL preserves the source's missing result (no qualifying route or a routing error), rather than a zero-minute journey. Negative and non-finite times are rejected. The table retains `slot_prijezdu`; the API's `rano` scenario will use the morning slot when analytical handlers are implemented. No `SCENAR` table is introduced by these imports.

Both tables have typed Diesel schemas/models and foreign-key joins. Municipality and year/age-group indexes support later catchment queries. The imports support future accessibility calculations; analytical endpoints are implemented separately.

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
- `GET /obory`: catalog of programs with at least one offering in the selected `forma` (default `den`), optionally filtered by CSV `stupen`. Sums capacity and applications over all matching offerings and counts distinct schools. Ratios return explicit null for zero capacity. Pressure index is applications per place divided by the contract's fixed regional baseline of 2.66. Job totals use both NSP suitability levels and exclude tertiary-only education categories, matching the employer endpoint defaults; companies are counted by distinct IČO, including workplaces without coordinates. No profession mapping gives null job counts; a mapping with no matching demand gives zero. Signals are `pretlak` (pressure index ≥ 1.5), `nizky_zajem` (index ≤ 0.5), and `poptavka_trhu` (jobs per place ≥ 2); CSV signal filters match any requested signal. Supports sorting by name, pressure index, or jobs per place; numeric sorts put null last in both directions with name/code tie-breaks. `max_min` and `scenar` are ignored, including invalid or repeated values. The response uses catalog-specific DTOs and contains no accessibility fields or travel metadata. Queries run in a consistent read-only database snapshot and need no demographic or travel-time tables.
- `GET /zsj?uroven=zsj`: GeoJSON accessibility polygons, nearest-school times, travel bands, child estimates, and travel-limit counts; see the implementation details below.
- `GET /student/skoly`: schools ordered by estimated morning travel time from the student’s ZSJ; see the implementation details below.
- `GET /obory/{kod}`: admissions and job totals across all study forms, union accessibility coverage, per-offering school coverage, and candidate schools ranked by additional children reached. `max_min`, `scenar`, and `kandidatu` are active; unknown program codes return 404.
- `GET /obory/{kod}/zamestnavatele`: profession mappings joined to demand and workplaces, filtered by `vhodnost` and `jen_ss`. Education categories are summed per profession/workplace without double-counting. Metadata includes distinct company/workplace counts, jobs, and workplaces without coordinates grouped by municipality. No mapping means `existuje: null`; a mapping with no matching demand means `existuje: false`. Workplace IDs are strings to retain integer precision.

Two operations (`/student/trasa`, `/simulace`) and the municipality/ORP levels of `/zsj` remain **stubs**: valid requests return documented HTTP **501** with the `Chyba` error schema and `error.kod = neimplementovano`. They require travel-time, demographic, bilance, or simulation logic. The read handlers are in `src/contract.rs`, with catalog queries in `src/catalog.rs` and `src/programs.rs`; synchronous Diesel work runs on blocking threads and does not call OTP.

`GET /student/skoly` is implemented using the precomputed matrix. `student::load_school_data` resolves the origin ZSJ using PostGIS `ST_Covers` with bound longitude/latitude; shared boundaries choose the smallest ZSJ code. It loads schools, offerings joined with program names, and travel times in a consistent read-only snapshot. `student::build_response` assembles the public DTO without further database queries. `forma` and optional `obor` restrict offerings; with `obor`, all matching schools are returned, including schools outside the travel limit. Without `obor`, only reachable schools are returned (including reachable schools without offerings). `scenar=rano` selects arrival slot `07:00-08:00`; reachability is the original duration ≤ `max_min`. Results are sorted by original duration with missing times last, then case-insensitive name and REDIZO. Display durations are rounded to whole minutes after comparison/sorting. Metadata counts matching schools before reachability filtering and identifies the origin ZSJ and approximate precision. NULL durations use `spoj.stav=data_nedostupna`, omit `cas_min`, and set `v_dosahu=false`; the source cannot distinguish a missing route from a routing error. Missing matrix rows or required school data produce 500. Points outside all known ZSJ polygons produce 422 (`mimo_uzemi`). Departure/arrival times, transfers, walking distance, and lines are omitted because the matrix does not store them. No demographic data or live OTP calls are needed.

`GET /zsj?uroven=zsj` returns all ZSJ polygons with the shortest known morning travel time to a matching school. With `obor`, schools are selected by program and `forma`; without it, all schools are candidates. Equal durations choose the smallest REDIZO. `scenar=rano` uses the `07:00-08:00` matrix. Original durations determine travel bands, reachability, and sorted unique 30/45/60/max_min counts; only displayed minutes are rounded. Demographics use the 2021 age 10–14 group divided by five and rounded per ZSJ. Reachable-child counts equal the cohort estimate or zero; percentages are 100 or zero, with zero for an empty cohort. No matching schools gives `bez_spojeni`; all candidate durations NULL gives `data_nedostupna`, without minutes or nearest school. Incomplete matrix/demographic inputs return 500, and an unavailable database returns 503. Geometry, demographics, school selection, and travel times are read in one consistent snapshot. Municipality (the default) and ORP levels still return 501.

`GET /obory/{kod}` reads program, offerings, schools, demographics, the morning travel-time matrix, profession mappings, and job demand in one read-only repeatable-read snapshot. It shares the per-ZSJ cohort rounding with `/zsj` and the catalog admissions/job thresholds. Offer totals include daily and distance study; school and regional coverage count children once regardless of overlapping offerings. Candidate schools do not teach the exact program in any form. Their score counts children outside the program's known current coverage who can reach that candidate within the original, unrounded time limit. Candidates sort by gain descending, case-insensitive name, then REDIZO; `kandidatu` limits the result. Related programs share the first two KKOV digits. NULL travel times are unknown and contribute no known coverage; missing matrix or demographic rows produce 500. The `spatna_dostupnost` signal uses coverage below 50% when total children are positive. Job counts use suitability 1/2 and exclude tertiary-only demand, include workplaces without coordinates, count distinct company IČO, and sum each demand row once. Unmapped programs return explicit null market data; mapped programs without demand return zero counts. Ratios with zero denominators return null; coverage percentage with zero children is zero. Candidate municipality names remain omitted because no municipality-name lookup is integrated.

Invalid parameters return **422** in the same error format. Parameters are validated directly against the YAML schemas, including required values, identifier patterns, enums and numeric bounds. Implemented detail routes return **404** for missing records. Database connection failures return **503**; query or required-data failures return **500**. Geographic/domain checks for analytical routes still require their business implementation.

`src/requests.rs` defines query structs for every operation, plus Redizo/KodOboru identifier newtypes and Scenar/Uroven/Forma/Format/Razeni enums. The `ContractQuery<T>` extractor validates raw values against OpenAPI, then deserializes them and applies defaults: `max_min = 120`, `scenar = rano`, `kandidatu = 5`, `razeni = nazev`, and `format = slovnik` on the corresponding endpoints. `stupen=H,M` becomes `Stupne(Vec<Stupen>)`; `signal=pretlak,poptavka_trhu` becomes a SignalFilter list. The program catalog validates its three supported signals; Scenar is now an enum allowing only rano. Forma defaults to den, and Uroven defaults to obec. The employer endpoint uses numeric Vhodnost (1 or 2, default 2) and jen_ss (default true). The school, program and employer detail routes use typed path identifiers separately from their query structs.

`src/types.rs` defines numeric NSP suitability values shared by requests and employer response models. The simulation dictionary is keyed by `jednotky` (4–6 digit area codes); GeoJSON map/simulation responses use territory levels and travel bands. Required nullable fields preserve explicit JSON null and reject missing keys.

`src/contract.rs` contains typed handler signatures and JSON/GeoJSON response wrappers. `src/dto.rs` contains public API structs independent of Diesel models. Implement each handler by naming its `ContractQuery(_params)` argument `ContractQuery(params)` and replacing its final unimplemented error with database/OTP logic returning the declared DTO; do not return the database model directly. Both simulation response formats have separate DTOs.

Swagger UI: `http://localhost:8000/docs`. The specification is served at `/openapi.yaml`; Swagger loads that URL instead of embedding a duplicate. Both files are embedded into the compiled backend and edits trigger rebuilding. When serving `swagger.html` as a static file, keep `openapi.yaml` in the same directory. When opening the HTML directly from disk, select `openapi.yaml` using the file picker; browsers block automatic local-file requests. Swagger UI assets still require an internet connection.

```sh
curl -i http://localhost:8000/api/v1/skoly
curl -i 'http://localhost:8000/api/v1/obory?stupen=H,M&razeni=-index_pretlaku'
curl -i 'http://localhost:8000/api/v1/student/skoly?lat=50.2312&lon=12.8711'
cargo test --test openapi_contract
```

The eighteen contract tests run without PostgreSQL or OTP. They compare route/operation coverage and every success response media type; compare all response DTO properties and query fields, Rust field types, enums, required fields and query defaults with the specification; validate independent representative fixtures before and after Rust serialization; and exercise actual stub/error responses, database-unavailable responses, and documentation routes. The disposable-database test verifies upgrading from seven to nine migrations, demographic totals, typed travel-time/demographic joins, normalized identifiers, NULL times, invalid-time rejection, rollback and reapplication. It also verifies student input loading, spatial boundary lookup, form/program/slot filtering, preservation of missing and long travel times, exact/fractional cutoff behavior, stable ordering, metadata counts, and actual student school responses. It also verifies ZSJ accessibility against independent SQL results, polygon preservation, program/form/slot selection, travel bands and fractional cutoffs, cohort rounding, limit counts, and missing-input failures. Program-detail checks cover union counts against independent SQL, both study forms, candidate ranking and limits, related programs, fractional cutoffs, unknown times, missing inputs, zero denominators, unmapped and mapped-empty market data, and distinct-company job totals. It additionally validates real read responses against OpenAPI and checks catalog filters, counts, missing coordinates, mapping semantics, and required-data failures. Negative cases check missing fields, nullable/non-nullable values, identifiers, coordinate dimensions, dates, enums, dictionary keys and array limits. Numeric bounds, formats, patterns and nullability are checked by the YAML JSON Schema validator; plain Rust String/Vec types do not encode every value constraint. These tests verify structural contracts and representative catalog/travel-time calculations, not routing accuracy or correctness for every possible future response.

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

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

Swagger UI: `http://localhost:8000/docs`. Set `BACKEND_PORT` in the root `.env` to use another host port; the container always listens on 8000. If a different database already occupies 5432, set `POSTGRES_PORT` to a free host port (for example, 5433). Containers connect to `db:5432` regardless of that host port. Only the backend and its database dependency start with this command; OTP is needed for student routing; other database reads work without it.

```sh
docker compose logs -f backend
docker compose exec backend obor-backend migrate-status
# Rebuild after changing Rust code, migrations, OpenAPI, or Swagger:
docker compose up -d --build backend
```

Unimplemented analytical levels still return 501. Automatic migrations apply only to container startup; local Cargo runs retain the explicit migration step below.

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
| 10 | `datasety/simulation_municipalities.csv` | `SIMULATION_OBCE` |

The first seven migrations preserve the original schema, constraints, indexes and seed data. Only outer `BEGIN` / `COMMIT` statements are removed because Diesel manages each migration transaction. The program/offer import is a prerequisite for `OBOR_PROFESE` and therefore included despite being outside `scripts/sql`. The SELECT examples and JSON verification reports are not migrations. Demographic and travel-time imports are embedded as migrations 8 and 9. An existing backend database with migrations 1–7 applied receives migrations 8–10; existing 1–9 databases receive only migration 10.

The demographic seed includes 21 age groups and 17,619 estimates for 839 ZSJ in 2021 (279,103 people), plus municipality codes for aggregation. Its source checks preserve totals by ZSJ and municipality. These are modeled age estimates, not current observed population. The `10-14` group is `1300100014`; estimating a school-entry cohort will require dividing its population by five.

The travel-time seed includes all 28,526 ZSJ × school pairs for arrival slot `07:00-08:00`. Migration 9 converts the source's legacy integer ZSJ identifiers to six-character text codes and corrects the school foreign key to `stredni_skoly`. Times are minutes; NULL preserves the source's missing result (no qualifying route or a routing error), rather than a zero-minute journey. Negative and non-finite times are rejected. The table retains `slot_prijezdu`; the API's `rano` scenario uses the morning slot in implemented analytical handlers. No `SCENAR` table is introduced by these imports.

Both tables have typed Diesel schemas/models and foreign-key joins. Municipality and year/age-group indexes support later catchment queries. The imports support future accessibility calculations; analytical endpoints are implemented separately.

Use a fresh database for initial setup. The existing scripts replace demand and program/profession mapping snapshots and remove employers absent from the snapshot; migrating an already populated database will retain that behavior. `down.sql` drops the corresponding tables and data in reverse dependency order and retains the shared PostGIS extension. Existing tables are not automatically marked as already migrated. Back up an existing database before adopting this migration history. Do not run standalone seed imports and migrations as competing initialization mechanisms.

After a migration has been applied, add a new migration for subsequent schema/data changes instead of editing its SQL. Diesel records completed versions in `__diesel_schema_migrations` and will not rerun them.

## OpenAPI endpoints

The root `openapi.yaml` is the public API contract. The server uses `contract::router` directly and exposes only its API routes, plus `/docs` and `/openapi.yaml` for documentation. All API operations are mounted under `http://localhost:8000/api/v1`:

- `/skoly`, `/skoly/{redizo}`
- `/student/skoly`, `/student/trasa`
- `/zsj`, `/obory`, `/obory/{kod}`, `/obory/{kod}/zamestnavatele`, `/simulace`

Database reads are implemented for:

- `GET /skoly`: GeoJSON school points, filtered offerings (`obor`, CSV `stupen`, `forma`), and offering/capacity/application totals. Without `obor` or `stupen`, schools without matching offerings remain included. Municipality names and pressure indices are omitted until the supporting lookup/calculation is implemented.
- `GET /skoly/{redizo}`: school details, study offerings, and catchment data from a consistent read-only snapshot. `max_min` filters each ZSJ using its unrounded precomputed travel time to this school; `scenar=rano` selects `07:00-08:00`. Children are the rounded per-ZSJ 10–14 population / 5 estimate. Reachable ZSJ are grouped by municipality with summed children and shortest travel time, sorted by municipality code. The catchment includes every ZSJ able to reach this school, independent of competing schools. NULL times contribute no known reachability; missing matrix/demographic rows return 500. Empty catchments return zero children and an empty municipality list.
- `GET /obory`: catalog of programs with at least one offering in the selected `forma` (default `den`), optionally filtered by CSV `stupen`. Sums capacity and applications over all matching offerings and counts distinct schools. Ratios return explicit null for zero capacity. Pressure index is applications per place divided by the contract's fixed regional baseline of 2.66. Job totals use both NSP suitability levels and exclude tertiary-only education categories, matching the employer endpoint defaults; companies are counted by distinct IČO, including workplaces without coordinates. No profession mapping gives null job counts; a mapping with no matching demand gives zero. Signals are `pretlak` (pressure index ≥ 1.5), `nizky_zajem` (index ≤ 0.5), and `poptavka_trhu` (jobs per place ≥ 2); CSV signal filters match any requested signal. Supports sorting by name, pressure index, or jobs per place; numeric sorts put null last in both directions with name/code tie-breaks. `max_min` and `scenar` are ignored, including invalid or repeated values. The response uses catalog-specific DTOs and contains no accessibility fields or travel metadata. Queries run in a consistent read-only database snapshot and need no demographic or travel-time tables.
- `GET /zsj?uroven=zsj`: GeoJSON accessibility polygons, nearest-school times, travel bands, child estimates, and travel-limit counts; see the implementation details below.
- `GET /student/skoly`: schools ordered by live OTP morning travel time from the student’s point, including walking distances and journey details.
- `GET /student/trasa`: up to three live OTP journeys with GeoJSON geometry for each walking or transit leg.
- `GET /obory/{kod}`: admissions and job totals across all study forms, union accessibility coverage, per-offering school coverage, and candidate schools ranked by additional children reached. `max_min`, `scenar`, and `kandidatu` are active; unknown program codes return 404.
- `GET /simulace`: daily-study capacity simulation following `core_job_spec/simulace_oboru.md` and its Python reference; supports ZSJ, municipality, and ORP dictionaries and GeoJSON.
- `POST /simulace/zmeny`: simultaneous additions, reductions and transfers of daily-study capacity for one program; see the batch capacity simulation example below.
- `GET /obory/{kod}/zamestnavatele`: profession mappings joined to demand and workplaces, filtered by `vhodnost` and `jen_ss`. Education categories are summed per profession/workplace without double-counting. Metadata includes distinct company/workplace counts, jobs, and workplaces without coordinates grouped by municipality. No mapping means `existuje: null`; a mapping with no matching demand means `existuje: false`. Workplace IDs are strings to retain integer precision.

The municipality/ORP levels of `/zsj` remain **stubs**: valid requests return documented HTTP **501** with the `Chyba` error schema and `error.kod = neimplementovano`. They require travel-time, demographic, bilance, or simulation logic. The read handlers are in `src/contract.rs`, with catalog queries in `src/catalog.rs` and `src/programs.rs`; synchronous Diesel work runs on blocking threads and does not call OTP.

`GET /student/skoly` selects schools and program offerings from a repeatable-read database snapshot, then calculates journeys using OTP GTFS GraphQL `planConnection`. PostGIS `ST_Covers` validates the origin and records its ZSJ; route calculations use the exact requested coordinates, not the ZSJ representative point. `forma` and optional `obor` restrict offerings. Without a program, only schools within `max_min` are returned; with a program, all matching schools are returned, including unreachable schools. Reachability and sorting use unrounded journey duration; display minutes are rounded afterward. Ties use case-insensitive school name and REDIZO. Responses include departure, arrival, transfers, walking distance in metres (`chuze_m`), total journey distance in metres (`vzdalenost_m`, sum of OTP leg distances), and route names. A valid result without a qualifying route uses `spoj.stav=bez_spojeni`. An upstream failure returns 502 for the whole response, without silently switching to the matrix. Metadata records the routing date, counts before reachability filtering, and the origin ZSJ. Neither student endpoint depends on the travel-time or demographic tables.

`GET /student/trasa` looks up the school’s coordinates, validates the origin, and returns up to three shortest distinct journeys among OTP’s returned candidates. Every leg uses the decoded OTP encoded polyline as a WGS84 GeoJSON LineString; no straight-line replacement is invented. Leg times prefer real-time estimates and fall back to scheduled times when estimates are unavailable. Walking legs include metres; transit legs include their route name. A valid search without journeys returns an empty FeatureCollection. Unknown schools return 404; points outside known ZSJ return 422; missing required database data returns 500 and connection failures 503; OTP transport, GraphQL, malformed-response, timeout and out-of-service-period failures return 502.

Both student endpoints share a client and a bounded in-memory cache of successful results, including no-route results, for 24 hours. Keys contain exact origin and destination coordinates and the routing day. Exact coordinates keep route geometry tied to the requested point. Concurrent requests for the same key share a calculation; failures are retried rather than cached. The client limits OTP concurrency to four, bounds HTTP calls to 20 seconds and total routing waits to 60 seconds, and caps response bodies at 8 MB. The `rano` scenario selects arrivals in `[07:00,08:00)` in Europe/Prague with same-day departure; it requests 20 candidates over a two-hour search window and chooses by shortest duration, breaking ties by latest arrival. This is the best returned candidate, not a guaranteed global optimum. The default service date is the next weekday whose morning window has not elapsed; holidays and school vacations are not modeled. `meta.den` makes that date visible.

Start OTP for student routing with `docker compose --profile otp up -d --build backend otp`. `OTP_URL` defaults to `http://localhost:8080/otp/gtfs/v1` locally and `http://otp:8080/otp/gtfs/v1` in Compose. Optional `OTP_SERVICE_DATE=YYYY-MM-DD` pins the service day for the loaded timetable; blank means automatic selection. Pinning a day does not refresh an expired GTFS feed. Other catalog and regional endpoints continue to work without OTP.

`GET /zsj?uroven=zsj` returns all ZSJ polygons with the shortest known morning travel time to a matching school. With `obor`, schools are selected by program and `forma`; without it, all schools are candidates. Equal durations choose the smallest REDIZO. `scenar=rano` uses the `07:00-08:00` matrix. Original durations determine travel bands, reachability, and sorted unique 30/45/60/max_min counts; only displayed minutes are rounded. Demographics use the 2021 age 10–14 group divided by five and rounded per ZSJ. Reachable-child counts equal the cohort estimate or zero; percentages are 100 or zero, with zero for an empty cohort. No matching schools gives `bez_spojeni`; all candidate durations NULL gives `data_nedostupna`, without minutes or nearest school. Incomplete matrix/demographic inputs return 500, and an unavailable database returns 503. Geometry, demographics, school selection, and travel times are read in one consistent snapshot. Municipality (the default) and ORP levels still return 501.

`GET /obory/{kod}` reads program, offerings, schools, demographics, the morning travel-time matrix, profession mappings, and job demand in one read-only repeatable-read snapshot. It shares the per-ZSJ cohort rounding with `/zsj` and the catalog admissions/job thresholds. Offer totals include daily and distance study; school and regional coverage count children once regardless of overlapping offerings. Candidate schools do not teach the exact program in any form. Their score counts children outside the program's known current coverage who can reach that candidate within the original, unrounded time limit. Candidates sort by gain descending, case-insensitive name, then REDIZO; `kandidatu` limits the result. Related programs share the first two KKOV digits. NULL travel times are unknown and contribute no known coverage; missing matrix or demographic rows produce 500. The `spatna_dostupnost` signal uses coverage below 50% when total children are positive. Job counts use suitability 1/2 and exclude tertiary-only demand, include workplaces without coordinates, count distinct company IČO, and sum each demand row once. Unmapped programs return explicit null market data; mapped programs without demand return zero counts. Ratios with zero denominators return null; coverage percentage with zero children is zero. Candidate municipality names remain omitted because no municipality-name lookup is integrated.

`GET /simulace` follows the Python reference with daily-study offers. It estimates demand from the program's share of daily applications and unrounded 10–14 population / 5, assigns demand to the nearest school within `max_min` before and after added capacity, and recomputes every participating school's catchment. Equal times choose the smallest REDIZO; dictionary output includes only strictly improved ZSJ (including improvements still outside the limit). It reports catchments, relief from deficit schools, transfers from other schools, newly reachable demand, utilization, verdict, and largest pre-existing deficit excluding the simulated school. A utilization of at least 0.5 is `dobre_misto`; otherwise transfers exceeding relief mean `spatne_misto`; other results are `neutralni`. An existing school with applications/capacity ≤ 2.66 returns an empty result and required `souhrn: null` with `meta.duvod=kapacita_staci`; zero regional capacity skips this early exit. Existing programs without daily offerings return 422. Missing schools/programs return 404. Travel NULLs are treated as infinite for this calculation (not proof of no service); missing required matrix rows or demographics return 500. All data is read in a single read-only repeatable-read transaction, with no live routing or writes.

Simulation DTOs preserve fractional estimates and travel times: displayed travel times/improvements use two decimals, children one decimal, and demand/summary rounding follows the reference. Improvements from unknown previous times are null; unchanged GeoJSON areas have zero improvement. Municipality/ORP aggregation sums demand/children and weights finite before/after/target times by the complete 2021 ZSJ population (all 21 age groups); zero-weight averages are null. A dictionary aggregate appears when any constituent ZSJ strictly improves. Summary area counts and limits refer to the requested level, while catchments/placement metrics remain calculated at ZSJ level. Aggregate `novy_dosah` means at least one constituent ZSJ is newly reached. Geometry is the union of existing ZSJ polygons. The default level remains municipality, as declared in OpenAPI.

Migration 10 seeds 134 municipality/military-area names and membership in seven ORP, with ČSÚ ORP codes converted from official RÚIAN lookups. Source URLs and SHA-256 hashes are recorded in `datasety/simulation_municipalities.source.md` and the migration. No downloads are needed at runtime or to apply the migration. Existing `/zsj` and `/obory/{kod}` retain their previous cohort calculations and responses.

The simulation tests compare 32 deterministic input/output cases generated by the supplied Python reference, including all verdicts, early exits, capacity increases, zero capacities/applications, ties, and unknown/fractional times. Regenerate the golden fixture from the repository root with `python3 scripts/generate_simulation_reference.py`; normal Cargo tests do not require Python. PostGIS tests exercise both formats at all three levels, geography seeding/rollback, response contracts, and required-input failures.

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

The contract tests run without PostgreSQL or OTP. They compare route/operation coverage and every success response media type; compare all response DTO properties and query fields, Rust field types, enums, required fields and query defaults with the specification; validate independent representative fixtures before and after Rust serialization; and exercise actual stub/error responses, database-unavailable responses, and documentation routes. The disposable-database test verifies upgrading from seven to ten migrations, demographic totals, typed travel-time/demographic joins, normalized identifiers, NULL times, invalid-time rejection, rollback and reapplication. It also verifies student input loading, spatial boundary lookup, form/program/slot filtering, live OTP summaries and routes through a deterministic HTTP service, no dependency on matrix rows, exact/fractional cutoff behavior, stable ordering, metadata counts, and actual student school responses. It also verifies ZSJ accessibility against independent SQL results, polygon preservation, program/form/slot selection, travel bands and fractional cutoffs, cohort rounding, limit counts, and missing-input failures. Program-detail checks cover union counts against independent SQL, both study forms, candidate ranking and limits, related programs, fractional cutoffs, unknown times, missing inputs, zero denominators, unmapped and mapped-empty market data, and distinct-company job totals. It additionally validates real read responses against OpenAPI and checks catalog filters, counts, missing coordinates, mapping semantics, and required-data failures. Negative cases check missing fields, nullable/non-nullable values, identifiers, coordinate dimensions, dates, enums, dictionary keys and array limits. Numeric bounds, formats, patterns and nullability are checked by the YAML JSON Schema validator; plain Rust String/Vec types do not encode every value constraint. These tests verify structural contracts and representative catalog/travel-time calculations, not routing accuracy or correctness for every possible future response.

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

## Batch capacity simulation

`POST /api/v1/simulace/zmeny` simulates multiple capacity changes for one daily-study program. The existing `GET /simulace` contract remains unchanged. This POST only reads the database and never persists the scenario.

```sh
curl -X POST http://localhost:8000/api/v1/simulace/zmeny \
  -H 'Content-Type: application/json' \
  -d '{"obor":"23-68-H/01","zmeny":[{"redizo":"600009084","zmena_kapacity":-30},{"redizo":"600009271","zmena_kapacity":30}],"max_min":120,"scenar":"rano","uroven":"obec","format":"slovnik"}'
```

Positive deltas add places; negative deltas remove them. The request allows 1–100 changes, each nonzero and between -300 and 300. Duplicate schools are summed before validating final capacity, so operation order has no effect. Final capacity must be nonnegative; zero removes that school from the simulated offering set, including existing zero-capacity offers. Removing the last offering is valid. A partial reduction that leaves positive capacity affects school balances without changing travel times or nearest-school catchments. Applications and estimated interest stay fixed at their database baseline; this models catchment demand, not admissions allocation constrained by capacity.

Defaults are `max_min=120`, `scenar=rano`, `uroven=obec`, and `format=slovnik`. The dictionary contains units whose travel time or nearest school changes, including worsened access. GeoJSON returns the entire layer. Responses include before/after capacities, catchments and balances for schools; demand transfers (null source/destination means outside the limit); gained/lost children and estimated applicants; and improved/worsened units. `zlepseni_min` is signed (negative for worsening), or null when either time is unknown. ZSJ children and interest remain fractional until display rounding. Municipality and ORP averages use complete population weights over finite times; aggregate flags mean any constituent ZSJ changed in that direction, so improvements and losses may coexist. School metrics and child totals always come from individual ZSJ. No single-school placement verdict applies to a combined scenario.

Unknown schools/programs return 404. Invalid JSON/body parameters, negative final capacity, and programs without daily offers return 422. Missing required matrix rows or demographic data return 500; explicit NULL travel times remain valid unknown journeys. Reads and geometry unions share one repeatable-read, read-only transaction. The OpenAPI specification includes both response formats and the complete JSON body schema.

Travel-limit parameters accept `max_min=0` for unlimited duration in student searches, accessibility, school/program catchments, and both simulation endpoints. Other accepted limits remain 10–180 minutes, default 120. Unlimited duration preserves the morning arrival window and excludes unknown/missing connections. Metadata keeps `max_min=0`; travel-limit count entries with `limit_min=0` count all known finite journeys.

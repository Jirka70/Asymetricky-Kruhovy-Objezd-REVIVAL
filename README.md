# Obor na dosah

Obor na dosah pomáhá žákům 9. tříd a jejich rodičům v Karlovarském kraji najít střední školy a obory, kam se z domova dostanou veřejnou dopravou, a ukazuje, kolik se o obor uchází zájemců a kteří zaměstnavatelé v regionu hledají jeho absolventy. Kraji a školám zároveň dává podklad pro plánování oborové nabídky: spojuje dojezdové časy, demografii základních sídelních jednotek, kapacity škol a poptávku na trhu práce.

## Použitá data

### 🎓 Vzdělávání
- [DATA ZÁPAD – Záměr počtu přijímaných uchazečů středních škol v Karlovarském kraji pro školní rok 2026/2027](https://www.datazapad.cz/datasets/9332e5a45e0d4dd999bef99a6f51fd40), CC0 1.0 – seznam středních škol, adresy, souřadnice a weby
- [CERMAT – Přihlášky do škol-oborů, 1. kolo přijímacích zkoušek 2026](https://data.cermat.cz/data-a-analyticke-vystupy-jednotna-prijimaci-zkouska/agregovana-data-jpz.html) ([XLSX](https://data.cermat.cz/files/files/JPZ/agregovana_data_skoly/PZ2026_kolo1_skolobory_prihlasky.xlsx)) – 140 nabídek, 81 oborů, 30 škol v Karlovarském kraji; kapacity a počty přihlášek

### 💼 Trh práce
- [MPSV – Volná místa za celou ČR](https://data.mpsv.cz/web/data/volna-mista-za-celou-cr) ([JSON](https://data.mpsv.cz/od/soubory/volna-mista/volna-mista.json)) – 744 nabídek v Karlovarském kraji
- [Národní soustava povolání – API v1.2](https://nsp.cz/api/v1.2) – mapování oborů na profese
- [ČÚZK RÚIAN – adresní místa (ArcGIS REST)](https://ags.cuzk.gov.cz/arcgis/rest/services/RUIAN/MapServer) – souřadnice pracovišť

### 🏘️ Území a obyvatelstvo
- [ČSÚ – Základní sídelní jednotky (RSO, Statistický geoportál)](https://geodata.csu.gov.cz/server/rest/services/Hosted/Open_data_RSO/FeatureServer/20) ([GeoJSON](https://geodata.csu.gov.cz/as/data/distribuce/Hosted/Open_data_RSO/FeatureServer/20/geojson.zip)) – polygony ZSJ, vybráno 839 ZSJ v Karlovarském kraji
- [ČSÚ – Sčítání 2021: obyvatelé a byty za ZSJ](https://csu.gov.cz/produkty/vysledky-scitani-2021-otevrena-data)
- [ČSÚ – Sčítání 2021: obyvatelstvo podle pětiletých věkových skupin a pohlaví](https://csu.gov.cz/docs/107508/669330fa-7201-a927-a7c8-16e45db63de0/sldb2021_vek5_pohlavi.csv?version=1.0)

### 🚌 Doprava
- [Jízdní řády vlaků – CZPTT → GTFS (JrUtil, 8. 10. 2026)](https://data.jr.ggu.cz/results/2026-10-08/CZPTT_GTFS.zip)
- [Jízdní řády autobusů – CIS JŘ / JDF → GTFS (JrUtil, 8. 10. 2026)](https://data.jr.ggu.cz/results/2026-10-08/JDF_merged_GTFS.zip)
- [OpenStreetMap – Karlovarský kraj (Geofabrik)](https://download.geofabrik.de/europe/czech-republic/karlovarsky-261008.osm.pbf) – © přispěvatelé OpenStreetMap, ODbL
- [Hranice kraje – OpenStreetMap / Nominatim](https://nominatim.openstreetmap.org/) – © přispěvatelé OpenStreetMap, ODbL

### 🧮 Vlastní výstup
- **Matice dojezdů ZSJ → škola** – 839 ZSJ × 30 škol, příjezd 7:00–8:00, výpočet v [OpenTripPlanneru](https://www.opentripplanner.org/) nad daty z jízdních řádů a OSM

Licenci najdete u každé datové sady v [Katalogu otevřených dat Karlovarského kraje](https://www.datazapad.cz/search?collection=dataset&layout=grid).

## Použití AI
Koncept aplikace, výběr a propojení datových sad i metodiku výpočtů (dojezdové časy ze ZSJ ke školám v ranním časovém okně, odhad věkové struktury ZSJ z dat obcí, párování oborů s profesemi a poptávkou zaměstnavatelů) navrhl tým. AI asistenti pro programování (mj. Claude) nám pomáhali s implementací – psaním a laděním skriptů pro zpracování dat, backendu v Rustu a API – a s přípravou dokumentace včetně dohledání zdrojů dat. Výstupy AI jsme průběžně kontrolovali, testovali a ověřovali proti zdrojovým datům.

## Spuštění

### Celá aplikace: frontend + backend + databáze

Z kořene repozitáře, po nastavení `.env` podle `.env.example`:

```bash
docker compose up -d --build --wait
```

Příkaz sestaví a spustí `frontend`, `backend` a `db` a počká na jejich kontroly
dostupnosti. Backend počká na databázi a před startem API aplikuje migrace.
Výchozí adresy jsou frontend http://localhost:3000 a API dokumentace
http://localhost:8000/docs. Porty lze změnit v `.env` pomocí `FRONTEND_PORT`,
`BACKEND_PORT` a `POSTGRES_PORT` (výchozí databázový port je 5432).
Pokud už běží lokální vývojový server nebo jiná databáze, použijte například
`FRONTEND_PORT=3001` a `POSTGRES_PORT=5433`.

```bash
docker compose ps
docker compose logs -f frontend backend
docker compose stop
```

Frontend načítá školy, obory a zaměstnavatele z REST API. Geometrie, demografie
a předpočítané dojezdy zůstávají v lokálním snímku. OTP je volitelný profil `otp`; spustí se výslovně
přes `docker compose up -d otp` nebo spolu s aplikací přes
`docker compose --profile otp up -d`. Studentské endpointy `/student/skoly` a `/student/trasa` vyžadují běžící OTP. Katalogové a krajské endpointy jej nepotřebují.

### Frontend (Next.js)

Produkční frontend v Dockeru spustíte z kořene projektu (s kořenovým `.env`
stejně jako u ostatních Compose služeb):

```bash
docker compose up -d --build frontend
```

Aplikace běží na http://localhost:3000. Port lze změnit pomocí `FRONTEND_PORT`
v `.env`. Pokud na portu běží vývojový server, nejprve jej ukončete, nebo použijte
například `FRONTEND_PORT=3001 docker compose up -d --build frontend`.
Příkaz spustí také backend a databázi, potřebné pro načítání katalogů.
Logy: `docker compose logs -f frontend`; zastavení: `docker compose stop frontend`.

Webová aplikace je ve složce `frontend/`. Používá Next.js App Router,
TypeScript, TanStack Query, vlastní CSS a Apache ECharts. Doporučený Node.js 22.18+.

```bash
cd frontend
npm ci
npm run dev
```

Otevřete adresu vypsanou v terminálu, standardně http://localhost:3000.
Jiný port lze zvolit příkazem `npm run dev -- --port 3001`.
Backend má výchozí adresu `http://127.0.0.1:8000`; jinou nastavte pomocí
`BACKEND_URL` v `frontend/.env.local`. Spusťte jej příkazem `docker compose up -d backend`.
Kontroly: `npm run lint`, `npm test` a `npm run build`.

Stránky `/kraj` a `/rodiny` obsahují interaktivní kartogram, simulaci nabídky
oborů, grafy a hledání uložených dojezdů. Katalogy a statistiky načítají z API
přes serverovou cache s obnovou po 24 hodinách; dojezdy počítají z lokálního
exportu. OpenTripPlanner při běhu nepotřebují.
Podíly přijatých a jednotlivé úseky cest jsou výslovně označené modelové ukázky,
protože v podkladech chybí. Podrobnosti a postup obnovy exportu jsou ve
[frontend/README.md](frontend/README.md).

### Lokální databáze PostgreSQL + PostGIS

Vyžaduje běžící Docker Desktop (nebo Docker Engine s Compose). `compose.yaml`
spouští PostgreSQL 18 s PostGIS 3.6. Image je `linux/amd64`; na Apple Silicon
poběží přes emulaci Docker Desktopu.

Při prvním nastavení zkopírujte `.env.example` do `.env` a nastavte
`POSTGRES_PASSWORD` na vlastní náhodné heslo. Existující `.env` nepřepisujte.
Soubor `.env` je ignorovaný Gitem. Pro tento lokální checkout již bylo heslo
vygenerováno a uloženo do `.env`.

Spuštění a kontrola:

```bash
docker compose up -d --wait db
docker compose ps
docker compose exec db psql -U obor -d obor_na_dosah -c 'SELECT PostGIS_Full_Version();'
```

Připojení z počítače: host `127.0.0.1`, port `5432`, databáze
`obor_na_dosah`, uživatel `obor`, heslo z `.env`. Port lze změnit přes
`POSTGRES_PORT`. Z jiného kontejneru ve stejném Compose používejte host `db`
a port `5432`. Uvedené příkazy předpokládají výchozí jméno databáze a uživatele.

SQL konzole:

```bash
docker compose exec db psql -U obor -d obor_na_dosah
```

PostGIS se v úvodní databázi aktivuje automaticky při prvním spuštění image.
Pro další nově založené databáze použijte `CREATE EXTENSION IF NOT EXISTS postgis;`.

Kompletní smazání a nové naplnění projektové databáze:

```bash
python3 scripts/drop_db.py
python3 scripts/load_db.py
```

`drop_db.py` smaže celou projektovou databázi včetně dat a ukončí její aktivní
připojení. `load_db.py` databázi případně vytvoří a postupně načte ZSJ, školy,
obory s nabídkami a demografii. Samostatný load lze spustit opakovaně.
Každý import má vlastní transakci; při chybě se načítání ihned zastaví a předchozí
dokončené importy zůstanou uložené. Loader nepřidává kontroly počtů nabídek.

Skripty vyžadují Python 3.9+ a běžící databázový kontejner. Vyhledají službu `db`
podle adresáře tohoto projektu, takže fungují i po změně názvu Compose projektu.
Jméno databáze a uživatele převezmou z nastavení kontejneru. Konkrétní cíl lze
zadat pomocí `--container JMENO --database DATABAZE`. Skripty lze spouštět
z libovolného pracovního adresáře. Lokální zálohy v `backups/` Git ignoruje.

Data jsou v pojmenovaném volume `postgres_data`, pro PostgreSQL 18 připojeném
na `/var/lib/postgresql`. `docker compose stop db` databázi zastaví a data zachová.
`docker compose down` data rovněž zachová; parametr `--volumes` by je odstranil.
Změny jména databáze, uživatele nebo hesla v `.env` samy nezmění již inicializovanou
databázi — inicializační proměnné se použijí jen pro prázdný volume.

Konfigurace je určena pro lokální vývoj a port zpřístupňuje pouze na localhostu.

## Tým
- <Jméno Příjmení> (@[login]) – [role]

## Licence
Kód: [MIT](LICENSE). Ostatní obsah: CC BY 4.0.

---
Prototyp z Hackathonu otevřených dat Karlovarského kraje 2026. Není oficiální službou Karlovarského kraje ani KIC KK.

## OpenTripPlanner

Place OSM data (`*.osm.pbf`) and GTFS archives (`*gtfs*.zip`) in `location_data/`.

Build or rebuild the graph after changing source data:

```bash
docker compose run --rm otp-build
```

Start the routing server using the saved graph:

```bash
docker compose up -d otp
```

Open http://localhost:8080. View logs with `docker compose logs -f otp`;
stop the server with `docker compose down`.
After rebuilding, load the updated graph with `docker compose restart otp`.
Both services use an 8 GB maximum Java heap; Docker needs additional memory
for the JVM and other running containers.

The downloaded train feed contained 124 stop-time rows referencing 10 trip IDs
absent from `trips.txt`. These rows were removed using
`python3 scripts/repair_gtfs.py location_data/vlaky.gtfs.zip`.
The original archive is preserved as `location_data/vlaky.gtfs.zip.original`;
the repair script always uses that backup when it exists.
The old invalid OSM download is retained as `location_data/kvk.redirect.html`.

## Test the routing API

Generate X random points inside the OpenStreetMap administrative boundary of
Karlovarský kraj, then query every ordered pair (X × (X − 1) requests):

```bash
python3 scripts/test_otp_pairs.py --points 20 --seed 42 --workers 6
```

Use `--workers X` to limit concurrent API requests (default: 1).
Output rows remain in deterministic pair order and the run summary records
the worker count and elapsed time.

Requires Python 3.9+ and a running OTP server; no extra Python packages.
The boundary is downloaded once from Nominatim and cached as
`location_data/karlovarsky_kraj.geojson` (© OpenStreetMap contributors, ODbL 1.0).
Sampling is reproducible with the same seed and cached boundary. Points are
sampled across the region, including rural or inaccessible locations; a point
inside the region is not guaranteed to have a transit connection.

Default target: Monday 12 October 2026, with arrival between 07:30:00 and
07:59:59 in Europe/Prague. Only same-day departures qualify. Each pair selects
the shortest-duration qualifying itinerary returned by OTP, breaking ties by
latest arrival. OTP returns filtered candidates, so this does not guarantee an
exhaustive global optimum. Walking-only itineraries are allowed alongside
transit with walking. Errors and missing connections remain in the results.

```bash
python3 scripts/test_otp_pairs.py --points 10 --max-early-minutes 15
python3 scripts/test_otp_pairs.py --points 20 --candidates 50 --search-window-minutes 180
```

Outputs are saved under `location_data/api_tests/<run timestamp>/`:
`points.geojson`, `routes.csv`, `routes.jsonl` (full API responses and selected
itineraries), and `summary.json`. Override this with `--output-dir PATH`.
The CSV contains one row per directed pair, with travel times, departure,
arrival, transfer counts, modes, route names and errors.

## Large source datasets (Git LFS)

`datasety/volna-mista.json` and `location_data/busy.gtfs.zip` are tracked
with Git LFS. Install Git LFS before cloning; for an existing checkout, run:

```bash
git lfs install --local
git lfs pull
```

## ZSJ-to-school travel times

Calculate every ZSJ-to-school pair for Monday 12 October 2026:

```bash
python3 scripts/calculate_zsj_school_times.py --date 2026-10-12 --slot 07:00-08:00 --workers 6
```

Repeat `--slot` to add arrival periods:

```bash
python3 scripts/calculate_zsj_school_times.py --date 2026-10-12 --slot 07:00-08:00 --slot 09:00-10:00 --workers 6 --output datasety/zsj_skoly_vice_slotu.csv
```

The default origins are all 839 ZSJ polygons embedded in
`scripts/sql/insert_zsj.sql`. One representative point inside each polygon
is used (a scanline midpoint accounting for polygon holes); this is not an
official definition point or a population-weighted origin. No SQL is executed.
The calculated origins are saved next to the matrix as `*_origins.csv`.
For explicit origin coordinates, use `--zsj-csv PATH` with columns
`kod_zsj,lat,lon`. Schools come from `datasety/skoly.csv`; override with
`--schools PATH` containing `redizo,lat,lon`.

The main CSV has exactly `kod_zsj,redizo,slot_prijezdu,doba_jizdy`.
A slot such as `07:00-08:00` includes arrival at 07:00 and excludes arrival
at 08:00, in Europe/Prague. Departure must be on the specified date.
Travel time in minutes is the shortest door-to-door duration among the
qualifying itineraries OTP returns, including walking and transfer waits.
Walking-only itineraries may qualify. OTP filters and limits candidates;
this is not a guarantee of a global optimum. Adjust `--candidates` (default 50)
to change the requested number of alternatives.

A blank `doba_jizdy` means no qualifying route or an API error; zero is never
used as a missing-data marker. Reasons and actual departure/arrival times are
stored in `*_diagnostics.csv`; settings, source hashes, counts and completion
status are stored in `*_summary.json`.
The default output is `datasety/zsj_skoly_2026-10-12.csv`.
For an interrupted run, repeat the same command with `--resume`.
Resume checks the input hashes, date, slots and routing settings.
Use a different output path for a new calculation; existing results are
not overwritten. `--limit-zsj 2` allows a small smoke test.

### Rust backend

Frontend vývojáři mohou spustit API a jeho databázi z kořene repozitáře pomocí `docker compose up -d backend` (s nakonfigurovaným kořenovým `.env`). Migrace se aplikují automaticky; Swagger UI je na `http://localhost:8000/docs`.

Backend s Axum, Diesel ORM a PostgreSQL/PostGIS je v `backend/`. Spuštění, migrace ze SQL skriptů a API jsou popsané v [backend/README.md](backend/README.md).

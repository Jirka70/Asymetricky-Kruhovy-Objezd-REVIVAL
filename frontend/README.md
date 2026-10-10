# Obor na dosah — frontend

Next.js 16 / React 19 / TypeScript / TanStack Query. Dvě propojené stránky nad REST API a lokální maticí dojezdů:

- `/kraj`: kartogram ZSJ, seskupené školy a zaměstnavatelé, přidání i odebrání oboru ve stávající škole, srovnání scénářů a grafy.
- `/rodiny`: doporučení oboru, hledání výchozí ZSJ, filtrování uložených cest, školy, zaměstnavatelé a časová osa.
- `/metodika`: původ, význam a omezení jednotlivých údajů.
- `/`: přesměrování na `/kraj`.

## Spuštění

Doporučený Node.js 22.18+ (testy používají nativní načtení TypeScriptu).

```bash
npm ci
npm run dev -- --port 3000
```

Otevřete http://localhost:3000. Spusťte také backend s databází (`docker compose up -d backend` z kořene projektu).
Serverový `BACKEND_URL` má výchozí hodnotu `http://127.0.0.1:8000`; jinou adresu lze nastavit v `frontend/.env.local`.
OTP není potřeba.

```bash
npm run lint
npm test
npm run build
npm start
```

## Docker Compose

Z kořene repozitáře, s nastaveným kořenovým `.env` pro Compose:

```bash
docker compose up -d --build --wait
```

Tím se spustí frontend, backend i databáze. Backend čeká na připravenou databázi
a aplikuje migrace. OTP se spouští pouze volitelně. Frontend volá backend přes
`BACKEND_URL=http://backend:8000`, nastavené v Compose.

Pro sestavení frontendu a spuštění jeho závislostí:

```bash
docker compose up -d --build frontend
docker compose ps frontend
docker compose logs -f frontend
```

Otevřete http://localhost:3000. Volitelný `FRONTEND_PORT` v kořenovém `.env`
mění hostitelský port; uvnitř kontejneru aplikace poslouchá na portu 3000.
Při souběhu s `npm run dev` lze použít
`FRONTEND_PORT=3001 docker compose up -d --build frontend`.
Zastavení: `docker compose stop frontend`.

Dockerfile používá více fází, Node.js 22 a Next.js
[`standalone` výstup](https://nextjs.org/docs/app/api-reference/config/next-config-js/output).
Běhový image obsahuje pouze potřebné závislosti, sestavenou aplikaci a `public/`
včetně datového snímku. Proces běží jako neprivilegovaný uživatel `node`.
Healthcheck kontroluje `/kraj`; nekontroluje dostupnost dat. Pro načtení katalogů je potřeba backend s databází, OTP potřeba není.
Po změně kódu nebo novém exportu dat zopakujte příkaz s `--build`.

## Data a modely

`public/data/` obsahuje export existujících dat: 839 ZSJ, 134 území obcí,
34 škol, 81 oborů, nabídku a poptávku profesí, demografii a předpočítané
cesty z 12. 10. 2026. Geometrie jsou zjednodušené pro zobrazení. Používají se
lokální soubory pro geografii, demografii a uložené cesty; katalogy a obchodní
statistiky se čtou z API. Aplikace nevolá OTP.

## Načítání a cache

- `GET /skoly`: všechny body škol; s `obor` a `forma` aktuální nabídka a souhrny přihlášek/kapacit.
- `GET /obory?forma=den` a `forma=dal`: společný katalog včetně oborů nabízených pouze dálkově, poptávka trhu práce.
- `GET /skoly/{redizo}`: informace o škole až po otevření jejího detailu.
- `GET /obory/{kod}/zamestnavatele`: pracoviště a profese po výběru oboru, s `jen_ss=true` a `vhodnost=2` stejně jako katalog.

Prohlížeč volá `/api/backend/...`. Next Route Handler předává jen tyto hotové
čtecí endpointy a používá `fetch(..., { next: { revalidate: 86400 } })`.
Serverová cache je oddělená podle URL včetně filtrů a sdílená mezi požadavky na
stejné instanci. Po zestárnutí ji obnoví následující požadavek; nejde o denní úlohu.
Změna filtrů znamená vlastní cache. Budoucí živé trasy se zde necachují.

TanStack Query uchovává data jako čerstvá 5 minut a neaktivní dotazy v prohlížeči
24 hodin. Cache není ukládána do localStorage. Skeleton se zobrazuje při prvním
načtení daného výběru; obnova téhož výběru ponechá data na obrazovce. Chyby mají
opakování a nikdy potichu neobnovují statistiky ze snímku. Chyba zaměstnavatelů
neblokuje školy. Zkratky škol a názvy obcí dočasně doplňuje lokální snímek.
V produkci může serverová cache přežít klientský reload; vývojový hard refresh
s vypnutou cache v DevTools se může chovat odlišně.

## Testy napojení

`npm test` ověřuje adaptéry, kombinace cache klíčů, deduplikaci, rušení požadavků,
obnovu dat, chyby, proxy a původní lokální výpočty.

```bash
npx playwright install chromium
npm run test:e2e
```

E2E sada sama sestaví produkční standalone frontend na portu 3107 a spustí
deterministický testovací backend na 8107. Ověřuje skutečný HTTP průchod a
sdílení serverové cache, skeleton, přepínání forem, školní a zaměstnavatelský
detail, rodinné hledání, simulaci a obnovu po výpadku. Projektovou DB nemění.
S již instalovaným Chrome lze použít `PLAYWRIGHT_CHANNEL=chrome npm run test:e2e`.

## Obnova snímku a omezení dat

Pro aktualizaci exportu z již naplněné místní DB a existujících CSV:

```bash
npm run data:export
```

Exportér používá `BEGIN READ ONLY` a společný `scripts/db_common.py` z kořene
repozitáře. Názvy obcí v `scripts/municipality-names.json` pocházejí ze stejného
lokálního RÚIAN podkladu jako `insert_zsj.sql`. Po novém exportu obnovte stránku.

- Dojezdy jsou minima vrácených kandidátů OTP v okně 7:00–8:00, ne univerzální
  optimum. Rodinný časový filtr pouze filtruje uloženou variantu každé dvojice.
- Podíly dostupnosti jsou vážené odhadem dětí 10–14 let ze SLDB 2021.
- Scénáře jsou dočasné v paměti prohlížeče. Změna oboru nebo formy je resetuje.
- **Acceptance rate je ilustrativní**: skutečné přijaté DB neobsahuje.
  Nabízená kapacita / přihlášky je samostatný skutečný údaj, nikoli acceptance rate.
- **Úseky cest a přestupy jsou ilustrativní**. Celkové odjezdy a příjezdy pocházejí
  z diagnostického CSV. Detail zatím nelze použít jako skutečný cestovní itinerář.
- Pracovní nabídky mají vazby na více oborů; sloupce se nesčítají.

## Rozhraní

Vlastní CSS v `src/app/globals.css`, systémová Helvetica Neue / Arial,
Phosphor ikony a modulárně načítané Apache ECharts (SVG). Záměr je střídmá mapová
aplikace inspirovaná ONS Census Maps a principy veřejných datových služeb,
bez převzatých cizích CSS souborů. Tailwind není potřeba.

Výpočetní funkce jsou v `src/lib/data.ts` a `src/lib/journeys.ts`. Mapy,
analytické grafy, obecní detail a cesta jsou samostatné komponenty.
Testy ověřují vážení populace, přidání/odebrání oboru, chybějící spojení,
časové filtry a konzistenci snímku. Stav vizuální kontroly je v `design-qa.md`.

## UX audit

Průchod aplikací a opravy jsou popsány v [ux-audit.md](ux-audit.md). Rodinné
hledání má přednost před volitelnými grafy; dlouhé seznamy lze prohledávat,
prázdné výsledky nabízí konkrétní nápravu a krajský scénář lze po resetu či
změně oboru jednou vrátit. Návrat nepřetrvává reload stránky.

# Obor na dosah — frontend

Next.js 16 / React 19 / TypeScript. Dvě propojené stránky nad lokálním datovým snímkem:

- `/kraj`: kartogram ZSJ / obcí, přidání i odebrání oboru ve stávající škole, srovnání scénářů, detail obce a grafy.
- `/rodiny`: doporučení oboru, hledání výchozí ZSJ, filtrování uložených cest, školy, zaměstnavatelé a časová osa.
- `/metodika`: původ, význam a omezení jednotlivých údajů.
- `/`: přesměrování na `/kraj`.

## Spuštění

Doporučený Node.js 22.18+ (testy používají nativní načtení TypeScriptu).

```bash
npm ci
npm run dev -- --port 3000
```

Otevřete http://localhost:3000. Běh frontendu nevyžaduje databázi ani OTP.

```bash
npm run lint
npm test
npm run build
npm start
```

## Data a modely

`public/data/` obsahuje export existujících dat: 839 ZSJ, 134 území obcí,
34 škol, 81 oborů, nabídku a poptávku profesí, demografii a předpočítané
cesty z 12. 10. 2026. Geometrie jsou zjednodušené pro zobrazení. Používají se
pouze lokální soubory, včetně mapy, ikon a fontů; aplikace nestahuje nové
doménové datové sady ani nevolá OTP. Není potřeba denní ISR.

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

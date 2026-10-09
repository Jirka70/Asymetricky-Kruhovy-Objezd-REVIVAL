# <Název projektu>

<Co projekt dělá a komu pomáhá. 2–3 věty.>

## Použitá data
- [Název datové sady](URL na datovou sadu), CC BY 4.0 <(upraveno), pokud jste data měnili>

Licenci najdete u každé datové sady v [Katalogu otevřených dat Karlovarského kraje](https://www.datazapad.cz/search?collection=dataset&layout=grid).

## Použití AI
<Které nástroje AI jste použili a k čemu.>

## Spuštění

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
Aplikační tabulky a import dat zatím nejsou součástí nastavení.

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

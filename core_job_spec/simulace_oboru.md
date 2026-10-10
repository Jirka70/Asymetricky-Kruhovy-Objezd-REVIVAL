# Simulace přidání oboru na školu

Algoritmus pro endpoint `GET /simulace` („co kdyby škola učila obor s danou kapacitou“).
Referenční implementace je v [`simulace_oboru.py`](simulace_oboru.py).

## Cíl

Kraj zvažuje, že na konkrétní školu přidá obor (nebo navýší jeho kapacitu). Simulace odpoví:

1. **Má smysl něco přidávat?** Pokud škola obor už učí a kapacita v kraji stačí, nic se nepočítá.
2. **Komu se zkrátí cesta?** Pro které ZSJ bude nová škola blíž než dosavadní nejbližší škola s oborem.
3. **Je kapacita na správném místě?** Jestli nová místa vznikají tam, kde chybí, a ne tam, kde místa jsou
   (typicky: chybí místa v Aši, ale přidáme je v Ostrově – to není dobrý krok).

Nic se neukládá; všechno se počítá z předpočítané matice dojezdů, OpenTripPlanner se nevolá.

## Vstup

`GET /simulace?redizo=…&obor=…&kapacita=…[&max_min=120][&scenar=rano][&uroven=zsj|obec|orp][&format=slovnik|geojson]`

| Parametr | Význam |
| --- | --- |
| `redizo` | Škola, na kterou se obor přidává. |
| `obor` | Kód oboru KKOV (např. `23-68-H/01`). |
| `kapacita` | Počet nových míst v 1. ročníku (1–300). |
| `max_min` | Nejdelší přijatelná doba dojezdu, výchozí 120 min. |
| `scenar` | Scénář dojezdu (výchozí `rano` = příjezd 7:00–8:00). |

## Použitá data

| Tabulka | Co z ní bereme |
| --- | --- |
| `NABIDKA_OBORU` | Které školy obor učí, jejich kapacita (`pocet_prijimanych`) a přihlášky (`loni_pocet_prihlasek`). |
| `DOJEZDOVE_DOBY` | Doba jízdy ZSJ → škola pro daný scénář; `NULL` = spojení neexistuje (bere se jako ∞). |
| `DATA_DEMOGRAFIE_ZSJ` | Děti 10–14 let v ZSJ (SLDB 2021). |

## Definice

| Pojem | Definice |
| --- | --- |
| `deti[z]` | Jeden ročník v ZSJ `z` = populace 10–14 let / 5. |
| `podil_zajmu` | Přihlášky na obor v kraji / všechny přihlášky v kraji. |
| `poptavka[z]` | `deti[z] × podil_zajmu` – kolik dětí ze ZSJ se o obor pravděpodobně zajímá. |
| `prihlasky_na_misto` | Přihlášky na obor / kapacita oboru v kraji. |
| `index_pretlaku` | `prihlasky_na_misto / 2,66` (krajský průměr). 1,0 = průměr. |
| `S` | Školy, které obor učí. |
| `doba(z, s)` | Doba dojezdu ze ZSJ `z` ke škole `s`. |

## Algoritmus

### Krok 0 – Validace

- Škola i obor musí existovat, `kapacita` je 1–300 → jinak **404 / 422**.
- Obor se musí v kraji někde učit – jinak nemáme z čeho odhadnout zájem (`podil_zajmu`).
- `skola_obor_uz_uci` = existuje nabídka (`redizo`, `obor`).

### Krok 1 – Je co řešit?

Porovná se zájem s kapacitou oboru v kraji **před** přidáním:

```
if skola_obor_uz_uci and prihlasky_na_misto <= 2.66:
    return { jednotky: {}, souhrn: null, meta: { …, duvod: "kapacita_staci" } }
```

> Proč ne přímo „přihlášky > kapacita“: každý uchazeč podává až 5 přihlášek, v kraji je 9 489 přihlášek
> na 3 572 míst, takže by podmínkou prošel téměř každý obor. Srovnání s krajským průměrem (2,66) odpovídá
> tomu, co API už počítá jako `index_pretlaku`.

> Kontrakt v `openapi.yaml` vyžaduje klíče `jednotky`, `souhrn`, `meta`, proto se vrací prázdné `jednotky`
> s důvodem v `meta.duvod`, ne doslova `{}`.

### Krok 2 – Poptávka v ZSJ

Pro každou ZSJ `z`: `poptavka[z] = deti[z] × podil_zajmu`.

### Krok 3 – Stav PŘED přidáním

Pro každou ZSJ najdi nejbližší školu, která obor už učí (bez simulované školy, pokud ho zatím neučí):

```
cas_pred[z], skola_pred[z] = min / argmin přes s ∈ S  doba(z, s)
```

**Spádovost** – každá škola „dostane“ poptávku ZSJ, pro které je nejbližší (jen ZSJ do `max_min`):

```
spad_pred[s]   = Σ poptavka[z]  pro z: skola_pred[z] = s
bilance_pred[s] = kapacita[s] − spad_pred[s]      # záporná = škole chybí místa
```

### Krok 4 – Stav PO přidání

Přidej simulovanou školu do `S` (nebo jí navyš kapacitu) a pro každou ZSJ:

```
t[z]          = doba(z, redizo)
cas_po[z]     = min(cas_pred[z], t[z])
prejde[z]     = t[z] < cas_pred[z]                     # nová škola je pro ZSJ blíž
zlepseni_min  = cas_pred[z] − cas_po[z]
novy_dosah[z] = cas_pred[z] > max_min  and  cas_po[z] <= max_min
```

Spádovost se přepočítá pro **všechny** školy s oborem: školám, od kterých ZSJ přešly, spád klesne.
Tím se promítne vliv nového oboru na ostatní stejné obory v kraji.

### Krok 5 – Je kapacita na správném místě?

Rozhoduje, odkud nová škola poptávku „bere“ (jen ZSJ, které k ní přešly, do `max_min`):

| Veličina | Poptávka ze ZSJ, které… | Hodnocení |
| --- | --- | --- |
| `odlehceni` | dřív patřily ke škole se **zápornou** bilancí | dobře – nová místa vznikají tam, kde chybí |
| `pretazeni` | dřív patřily ke škole s **volnými místy** | špatně – jen přetahování žáků |
| `novi_v_dosahu` | dřív obor v dosahu vůbec neměly | nejlépe – obor se dostane k novým dětem |

```
vyuziti = (odlehceni + novi_v_dosahu) / kapacita

verdikt = "dobre_misto"   if vyuziti >= 0.5
          "spatne_misto"  elif pretazeni > odlehceni
          "neutralni"     otherwise
```

Navíc se vrací `nejvetsi_deficit` – škola s oborem, které podle spádovosti nejvíc chybí míst,
tedy kde by nová kapacita pomohla nejvíc. Práh `0.5` je nastavitelný.

### Krok 6 – Výstup

- **`jednotky`** – jen ZSJ, kde `t[z] < cas_pred[z]`, ve tvaru `SimulacePlocha`:
  `cas_ke_skole`, `cas_min_puvodni`, `cas_min`, `zlepseni_min`, `pasmo`, `pasmo_puvodni`, `deti`,
  `potencialni_uchazeci`, `novy_dosah`. Dřív nedostupné ZSJ mají `cas_min_puvodni` i `zlepseni_min` `null`.
- **`souhrn`** – stávající pole (`v_limitu` před/po pro 30/45/60/`max_min`, `zlepsenych_jednotek`,
  `prumerne_zkraceni_min`, `potencialni_uchazeci`, `kapacita`, `uchazecu_na_misto`) a nově:
  `bilance_skol` (`redizo`, `kapacita`, `spad_pred`, `spad_po`), `odlehceni`, `pretazeni`,
  `novi_v_dosahu`, `vyuziti`, `verdikt`, `nejvetsi_deficit`.
- **`meta`** – vstupní parametry, `podil_zajmu`, `skola_obor_uz_uci`, `index_pretlaku`, případně `duvod`.
- `format=geojson` – celá mapa ve tvaru `/zsj`, nezměněné ZSJ se `zlepseni_min: 0`; frontend jen přebarví.
- `uroven=obec|orp` – ZSJ se sečtou (`deti`, `potencialni_uchazeci`) a časy zprůměrují vážené počtem obyvatel.

**Složitost:** 839 ZSJ × ≤ 34 škol ≈ 28 tisíc dvojic → jeden SQL dotaz nebo pár milisekund v paměti.

## Ukázky na reálných datech

Obor **23-68-H/01 Mechanik opravář motorových vozidel** (4 školy, 109 míst, 455 přihlášek,
`index_pretlaku` 1,57), přidáno 30 míst, příjezd 7:00–8:00:

| Kam přidáme | Zlepšených ZSJ | Ø zkrácení | Odlehčení | Přetažení | Využití | Verdikt |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| ZŠ a SŠ **Aš** | 44 | 55,7 min | 9,5 | 0,0 | 0,33 | `neutralni` |
| Gymnázium **Ostrov** | 144 | 2,0 min | 1,7 | 12,1 | 0,06 | `spatne_misto` |

V Aši nová místa berou poptávku jen od přetížené školy v Chebu a výrazně zkracují cestu.
V Ostrově už obor učí SPŠ Ostrov, nová kapacita by jí jen přetahovala žáky (cesta se zkrátí o 2 minuty).
V obou případech algoritmus ukáže, že nejvíc míst (~18) chybí SOŠ Karlovy Vary.

Obor **23-41-M/01 Strojírenství** na SPŠ Ostrov, která ho už učí (`index_pretlaku` 0,59)
→ prázdné `jednotky`, `duvod: "kapacita_staci"`.

## Omezení

- Poptávka je odhad: demografie ze SLDB 2021 a podíl zájmu z přihlášek 1. kola 2026 rozpočítaný rovnoměrně
  na všechny ZSJ (neřeší, že zájem o obor se může mezi regiony lišit).
- Spádovost předpokládá, že žák jde na nejbližší školu s oborem; ve skutečnosti volí i podle kvality a preferencí.
- Přihlášky v `NABIDKA_OBORU` jsou všechny přihlášky (ne jen 1. priorita), proto se přetlak posuzuje relativně.
- Simulace je zatím jen pro stávající školy (`redizo`), protože doby dojezdu existují jen pro ně.

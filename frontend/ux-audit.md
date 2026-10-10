# UX audit a provedené opravy

10. 10. 2026 · Obor na dosah · `/kraj` a `/rodiny`

## Verdikt

Dobrý funkční základ, ale původní rozhraní dávalo přednost analytickým grafům
před hlavními úkoly. Největší problémy nebyly v barvě nebo fontu: rodina obtížně
nacházela hledání, po potvrzení neviděla konkrétní školy a některé prázdné stavy
radily nesprávnou nápravu. Krajská simulace potřebovala přesnější zpětnou vazbu
a ochranu rozpracovaného scénáře. Tyto problémy jsou opravené v implementaci.

Jde o odbornou heuristickou kontrolu s reálným průchodem v prohlížeči, nikoli
o uživatelský výzkum. Nelze z ní odvodit naměřenou úspěšnost skutečných rodin
nebo krajských analytiků ani plnou shodu s WCAG.

## Rozsah a postup

In-app browser, živá lokální aplikace, snímky pořízené v tomto auditu.
Desktop 1440 × 1000, tablet 820 × 1180, mobil 390 × 844. Kontrolovány stavové
změny, aktuální DOM, klávesové ovládání, screenshoty a výsledné chování.

1. **Kraj — nastavení analýzy a čtení mapy.** Funkční základ, původně příliš dlouhé
   seznamy a odborné popisky. Nyní vyhledávání oboru, školy a obce, vysvětlení
   územní jednotky a hranice dostupnosti. Stav: ověřeno.
2. **Kraj — přidání oboru, přepnutí pohledu a návrat scénáře.** Původně nesoulad
   počtu škol a tiché vymazání při změně filtru. Nyní správný počet podle pohledu,
   souhrn dopadu u mapy a vrácení předchozího scénáře. Stav: ověřeno.
3. **Rodiny — vstup a volba oboru.** Původně hledání až pod grafy. Nyní hledání
   jako první úkol, dobrovolné rozbalení porovnání oborů. Stav: ověřeno.
4. **Rodiny — potvrzení, výsledky a detail cesty.** Původně slabá odezva a školy
   pod velkou mapou. Nyní souhrn počtu, přesun fokusu na výsledky, návaznost na
   vybranou cestu a odkazy zpět. Stav: ověřeno; úseky cesty zůstávají modelové.
5. **Rodiny — prázdný výsledek.** Původně jediná časová hláška i pro neexistující
   školní nabídku. Nyní odlišné důvody a použitelné nápravné akce. Stav: ověřeno.
6. **Mobil, tablet a klávesnice.** Původně vstup schovaný pod grafy, drobná písma
   a ovladače. Nyní dostupnější hlavní úkon, karty škol, větší dotykové plochy
   a textový přehled mapových bodů. Stav: ověřeno v uvedených viewports.

## Nálezy a změny

| Priorita | Nález a důkaz | Provedená změna |
|---|---|---|
| P1 | Mobilní formulář začínal na 1124 CSS px, mimo první obrazovku (krok 6). | Přesunut na začátek; nyní začíná na 344 CSS px při stejné šířce. Tlačítko hledání je viditelné v prvním viewportu 390 × 844. |
| P1 | Strojírenské práce + dálkové studium: nula nabízejících škol, ale rada „změňte čas“ (krok 5). | Rozlišení nulové nabídky a chybějící uložené cesty. Návrat na denní studium jedním kliknutím, pokud existuje nabídka. Časová varianta nabízí uložené příjezdy do 08:00, pokud existují. |
| P1 | Po hledání žádný přímý přechod na konkrétní školy (krok 4). | Počet výsledků, souhrn platného zadání, přechod na výsledky i s klávesovým fokusem. Stejné chování při otevření konkrétní školy. |
| P2 | Změněná pole mohou být zaměněna za zadání aktuální mapy (kroky 3–4). | Viditelný stav nepotvrzených změn a tlačítko aktualizace; výsledky vypisují poslední potvrzené zadání. |
| P2 | 81 oborů a dlouhé seznamy škol a obcí bez vyhledávání (kroky 1, 3). | Opakovaně použitý combobox, hledání názvu i kódu bez diakritiky, šipky/Enter/Escape, počet možností pro asistivní technologie. |
| P2 | Po přidání oboru nadpis mapy ukazoval 3 školy, vrstva 4 (krok 2). | Počet škol odpovídá současnému stavu nebo scénáři. Dopad a počet změn jsou přímo u mapy. |
| P1 | Změna oboru či formy mazala rozpracovaný scénář (ověřeno i v kódu). | Informace o změně kontextu a jednorázové vrácení původního scénáře; totéž po resetu. Bez blokujícího potvrzovacího dialogu. |
| P2 | Datum a školní rok v selectu s jedinou položkou (kroky 1, 3). | Pevné datum/snímek jako informace, nikoli zdánlivě volitelný filtr. |
| P2 | Modelové přijetí vypadalo jako doporučení skutečné šance (krok 3). | Popis uvádí ukázková čísla před grafem; nadpis nepředstírá aktuální žebříček přijímání. Porovnání je dostupné na vyžádání. |
| P2 | Mnoho textů mělo 9–11 px, mobilní mapové ovladače byly drobné (kroky 1, 6). | Důležité pomocné texty nejméně 12 px, mobilní formuláře 16 px, hlavní mobilní tlačítka a zoom 44 px. Silnější kontrast hranic vstupů. |
| P2 | Tabletová tabulka při závěrečné kontrole způsobovala 23px přesah. | Karty škol do 1000 px, na menším tabletu ve dvou sloupcích. Opět ověřeno 820/820 px a 390/390 px bez přesahu stránky. |
| P2 | Mapa se dala procházet převážně ukazovátkem. | Textový seznam škol a zaměstnavatelů, výběr obce v hledacím poli, školy propojené s detailem cesty. |
| P3 | Zaokrouhlené časy 88 → 74 min byly vedle změny −15 min (krok 2). | Změna v tabulce odpovídá rozdílu zobrazených minut; doplněné vysvětlení zaokrouhlení. Výpočet samotných dojezdů se nemění. |

## Proč právě tato pravidla

- [Nielsen Norman Group — 10 heuristik](https://www.nngroup.com/articles/ten-usability-heuristics/):
  prioritizace hlavního úkolu, viditelnost stavu, uživatelská kontrola a náprava
  chyb. Použito pro pořadí rodinné stránky, souhrn výsledků a vrácení scénáře.
- [GOV.UK — Select](https://design-system.service.gov.uk/components/select/):
  veřejné služby nemají uživatele nutit do dlouhých výběrů bez zvážení lepší
  alternativy. Zde zůstává krátký výběr formy, dlouhé seznamy dostaly hledání
  a jednopoložkové výběry byly odstraněny.
- [WAI — Combobox](https://www.w3.org/WAI/ARIA/apg/patterns/combobox/):
  přístupný název, stav rozbalení, vazba na seznam a ovládání klávesnicí.
- [WCAG — Status messages](https://www.w3.org/WAI/WCAG22/Understanding/status-messages.html):
  změny stavu jsou dostupné přes `role=status`; explicitní přechod po potvrzení
  hledání navíc přesouvá fokus na nadpis výsledků.
- [WCAG — Target size](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html):
  minimum AA je 24 × 24 CSS px s výjimkami. Pro důležité mobilní ovladače byla
  zvolena pohodlnější výška 44 px, nikoli tvrzení, že ji WCAG AA vždy vyžaduje.
- [WCAG — Non-text contrast](https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html):
  zřetelnější hranice formulářových polí. Samotná úprava barev není důkazem
  kompletní přístupnosti aplikace.

## Ověření výsledku

- Hledání oboru `Strojirenske prace` bez diakritiky, potvrzení Enterem.
- Hledání místa `Touzim`, šipky a zrušení Escape.
- Doporučení nejžádanějšího oboru vyplní formulář, vrátí fokus a čeká na potvrzení.
- Po potvrzení focus na `results-heading`; při výběru školy na `journey-heading`.
- Strojírenské práce + dálkové → správná nulová nabídka → denní → 1 škola.
- Odchod od 07:50 → žádná uložená cesta → příjezdy do 08:00 → 3 školy.
- Kraj: přidání IT ve SLŠ Žlutice → 4 školy, 92 částí obcí lépe, 0 hůře;
  změna oboru → návrat scénáře; reset → návrat scénáře; přepnutí ZSJ/obce,
  hranice 45/60 min a porovnání dopadů.
- Textový přehled mapy obsahuje školy s adresami a zaměstnavatele s počty míst.
- Desktop, tablet a mobil; mapové ovladače na mobilu změřeny na 44 px.
- Konzole v závěrečném kontrolním průchodu bez nových chyb a varování.
- `npm run lint`, 7/7 existujících datových testů, `npm run build`: prošly.

## Co tento audit neprokazuje

Neproběhlo sezení se skutečnými rodiči či analytiky, úplný test se čtečkou,
reálným dotykovým zařízením, všemi prohlížeči ani systematický offline test.
Mapa je detailní a na mobilu bude dál vyžadovat zoom; velké územní přehledy
se lépe porovnávají na desktopu. Tabulky a textové alternativy umožňují část
úkolů vyřešit bez přesného trefování mapových ploch.

Přijetí a dílčí úseky jízd zůstávají označené modelové hodnoty. Uložené cesty
mají pevné datum a okno. Aplikace není živý vyhledávač jízdních řádů. Scénáře
jsou dočasné; vrácení funguje v aktuálním průchodu, nikoli po reloadu.

Před veřejným spuštěním má největší hodnotu ověřit s cílovými uživateli tři
úlohy: najít vhodnou školu, vysvětlit rozdíl mezi částmi obce a porovnat dopad
odebrání oboru. To je doporučený další krok, ne již provedený výzkum.

## Obrazový záznam

Níže jsou snímky pořízené a prohlédnuté v tomto auditu. První série zachycuje
výchozí problémy; druhá provedené opravy. Všechny zobrazené hodnoty jsou ze
stejného lokálního snímku, případně viditelně označeného modelu.

### 1. Kraj — výchozí nastavení

![1. Kraj — výchozí nastavení](/Users/jirka/WEB/Asymetricky-Kruhovy-Objezd-REVIVAL/output/ux-audit-2026-10-10/01-region-start.jpg)

### 2. Kraj — původní scénář

![2. Kraj — původní scénář](/Users/jirka/WEB/Asymetricky-Kruhovy-Objezd-REVIVAL/output/ux-audit-2026-10-10/02-region-scenario.jpg)

### 3. Rodiny — původní vstup

![3. Rodiny — původní vstup](/Users/jirka/WEB/Asymetricky-Kruhovy-Objezd-REVIVAL/output/ux-audit-2026-10-10/03-family-start.jpg)

### 4. Rodiny — původní odezva hledání

![4. Rodiny — původní odezva hledání](/Users/jirka/WEB/Asymetricky-Kruhovy-Objezd-REVIVAL/output/ux-audit-2026-10-10/04-family-search.jpg)

### 5. Rodiny — nesprávný prázdný stav

![5. Rodiny — nesprávný prázdný stav](/Users/jirka/WEB/Asymetricky-Kruhovy-Objezd-REVIVAL/output/ux-audit-2026-10-10/05-family-empty.jpg)

### 6. Mobil — původní začátek

![6. Mobil — původní začátek](/Users/jirka/WEB/Asymetricky-Kruhovy-Objezd-REVIVAL/output/ux-audit-2026-10-10/06-family-mobile.jpg)

### Po opravě — kraj a souhrn scénáře

![Po opravě — kraj a souhrn scénáře](/Users/jirka/WEB/Asymetricky-Kruhovy-Objezd-REVIVAL/output/ux-audit-2026-10-10/10-region-after.jpg)

### Po opravě — rodinné hledání

![Po opravě — rodinné hledání](/Users/jirka/WEB/Asymetricky-Kruhovy-Objezd-REVIVAL/output/ux-audit-2026-10-10/14-family-desktop-after.jpg)

### Po opravě — mobilní vstup

![Po opravě — mobilní vstup](/Users/jirka/WEB/Asymetricky-Kruhovy-Objezd-REVIVAL/output/ux-audit-2026-10-10/11-mobile-start-after.jpg)

### Po opravě — mobilní výsledky

![Po opravě — mobilní výsledky](/Users/jirka/WEB/Asymetricky-Kruhovy-Objezd-REVIVAL/output/ux-audit-2026-10-10/07-mobile-results-after.jpg)

### Po opravě — náprava prázdného výsledku

![Po opravě — náprava prázdného výsledku](/Users/jirka/WEB/Asymetricky-Kruhovy-Objezd-REVIVAL/output/ux-audit-2026-10-10/09-empty-recovery-after.jpg)

### Po opravě — detail cesty

![Po opravě — detail cesty](/Users/jirka/WEB/Asymetricky-Kruhovy-Objezd-REVIVAL/output/ux-audit-2026-10-10/08-mobile-journey-after.jpg)

### Po opravě — karty na tabletu

![Po opravě — karty na tabletu](/Users/jirka/WEB/Asymetricky-Kruhovy-Objezd-REVIVAL/output/ux-audit-2026-10-10/12-tablet-results-after.jpg)

### Po opravě — mobilní mapa

![Po opravě — mobilní mapa](/Users/jirka/WEB/Asymetricky-Kruhovy-Objezd-REVIVAL/output/ux-audit-2026-10-10/13-region-mobile-after.jpg)

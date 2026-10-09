-- Příklady hledání nad MVP pracovního trhu. Pouze čtení.
-- Počty odpovídají uloženému exportu, nikoli automaticky dnešnímu trhu.
-- Sloupec pocet_mist je počet míst, nikoli počet inzerátů či absolventů.
BEGIN READ ONLY;

-- 1. Největší poptávka v kraji podle tří číslic CZ-ISCO.
SELECT p.cz_isco3, g.nazev,
       sum(p.pocet_mist) AS pocet_mist,
       count(DISTINCT z.ico) AS pocet_zamestnavatelu
FROM public."POPTAVKA_PROFESI" p
JOIN public."PROFESNI_SKUPINY" g USING (cz_isco3)
JOIN public."ZAMESTNAVATELE" z ON z.id = p.zamestnavatel_id
GROUP BY p.cz_isco3, g.nazev
ORDER BY pocet_mist DESC, p.cz_isco3
LIMIT 10;

-- 2. Kde je poptávka související se zvoleným školním oborem.
-- Změňte kód oboru. EXISTS započítá každý řádek poptávky jen jednou.
-- Skupiny jsou širší než konkrétní povolání; nejde o přesný počet míst
-- vyžadujících absolventa tohoto oboru. Počty různých oborů nesčítejte.
SELECT z.kod_obce,
       sum(p.pocet_mist) AS mista_v_souvisejicich_skupinach,
       count(DISTINCT z.ico) AS pocet_zamestnavatelu,
       coalesce(sum(p.pocet_mist) FILTER (WHERE z.lat IS NULL), 0) AS mista_bez_souradnic
FROM public."POPTAVKA_PROFESI" p
JOIN public."ZAMESTNAVATELE" z ON z.id = p.zamestnavatel_id
WHERE EXISTS (
    SELECT 1 FROM public."OBOR_PROFESE" op
    WHERE op.cz_isco3 = p.cz_isco3 AND op.kod_oboru = '65-51-H/01'
)
GROUP BY z.kod_obce
ORDER BY mista_v_souvisejicich_skupinach DESC, z.kod_obce;

-- 3. Školy a obory související s profesní skupinou 512 (kuchaři).
-- DISTINCT sjednotí různé formy studia a zaměření stejného oboru na škole.
SELECT DISTINCT s.redizo, s.nazev AS skola, o.kod AS kod_oboru,
       o.nazev AS obor, op.vhodnost
FROM public."OBOR_PROFESE" op
JOIN public."PROFESNI_SKUPINY" g USING (cz_isco3)
JOIN public."OBORY" o ON o.kod = op.kod_oboru
JOIN public."NABIDKA_OBORU" n ON n.kod_oboru = o.kod
JOIN public.stredni_skoly s ON s.redizo = n.redizo
WHERE g.cz_isco3 = '512'
ORDER BY s.nazev, o.kod;

-- 4. Poptávka bez doloženého mapování; při hledání ji neprezentujte jako nulu.
SELECT p.cz_isco3, g.nazev, sum(p.pocet_mist) AS pocet_mist
FROM public."POPTAVKA_PROFESI" p
JOIN public."PROFESNI_SKUPINY" g USING (cz_isco3)
WHERE NOT EXISTS (
    SELECT 1 FROM public."OBOR_PROFESE" op WHERE op.cz_isco3 = p.cz_isco3
)
GROUP BY p.cz_isco3, g.nazev
ORDER BY pocet_mist DESC, p.cz_isco3;

COMMIT;

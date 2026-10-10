#!/usr/bin/env python3
"""Simulace přidání oboru na školu – „Obor na dosah“.

Implementuje algoritmus popsaný v simulace_oboru.md:

  0. validace vstupu,
  1. rozhodnutí, zda je co počítat (obor už na škole je a kapacita stačí → nic),
  2. poptávka po oboru v každé ZSJ (deti × podil_zajmu),
  3. stav PŘED: nejbližší škola s oborem pro každou ZSJ, spádovost a bilance škol,
  4. stav PO: totéž včetně simulované školy, změna časů a spádovosti,
  5. vyhodnocení, zda je nová kapacita na správném místě,
  6. výstup ve tvaru endpointu GET /simulace (slovník `jednotky`, `souhrn`, `meta`).

Jádro (`simuluj`) pracuje jen s předanými daty, nic nečte z disku ani z DB,
takže ho lze přímo přepsat do Rustu / SQL nebo volat z testů.

Spuštění nad daty z repozitáře (jen standardní knihovna, Python 3.9+):

    python3 simulace_oboru.py --root CESTA_K_REPU --redizo 600009271 --obor 18-20-M/01 --kapacita 30
"""
from __future__ import annotations

import argparse
import csv
import json
import math
import re
from dataclasses import dataclass, field
from pathlib import Path
from typing import Dict, List, Optional, Tuple

INF = math.inf
PRUMER_KRAJE_PRIHLASEK_NA_MISTO = 2.66  # `index_pretlaku` = 1,0 (openapi.yaml)
LIMITY_MIN = (30, 45, 60)


# ---------------------------------------------------------------------------
# Vstupní data
# ---------------------------------------------------------------------------

@dataclass(frozen=True)
class Nabidka:
    """Jeden řádek NABIDKA_OBORU (škola × obor)."""
    redizo: str
    kod_oboru: str
    kapacita: int          # pocet_prijimanych
    prihlasky: int         # loni_pocet_prihlasek (přijímací řízení 2026, 1. kolo)


@dataclass
class Data:
    """Vše, co simulace potřebuje. Odpovídá tabulkám v DB."""
    nabidky: List[Nabidka]
    # DOJEZDOVE_DOBY pro zvolený scénář: (kod_zsj, redizo) -> minuty; chybějící spoj = klíč chybí nebo None
    dojezdy: Dict[Tuple[str, str], Optional[float]]
    # DATA_DEMOGRAFIE_ZSJ: kod_zsj -> počet dětí v jednom ročníku (10–14 let / 5)
    deti: Dict[str, float]
    nazvy_zsj: Dict[str, str] = field(default_factory=dict)
    skoly: Dict[str, str] = field(default_factory=dict)  # redizo -> název


class NeplatnyVstup(ValueError):
    """Odpovídá HTTP 404 / 422."""


# ---------------------------------------------------------------------------
# Pomocné funkce
# ---------------------------------------------------------------------------

def pasmo(cas: float, max_min: int) -> str:
    if cas == INF:
        return "bez_spojeni"
    if cas > max_min:
        return "mimo_dosah"
    if cas <= 30:
        return "do30"
    if cas <= 45:
        return "30_45"
    if cas <= 60:
        return "45_60"
    return "nad60"


def doba(data: Data, kod_zsj: str, redizo: str) -> float:
    v = data.dojezdy.get((kod_zsj, redizo))
    return INF if v is None else float(v)


def nejblizsi(data: Data, kod_zsj: str, skoly: List[str]) -> Tuple[float, Optional[str]]:
    """Nejkratší doba ze ZSJ k některé ze škol; při shodě vyhrává menší REDIZO (determinismus)."""
    best, best_s = INF, None
    for s in sorted(skoly):
        t = doba(data, kod_zsj, s)
        if t < best:
            best, best_s = t, s
    return best, best_s


def spadovost(poptavka: Dict[str, float], prirazeni: Dict[str, Optional[str]],
              max_min: int, casy: Dict[str, float]) -> Dict[str, float]:
    """Součet poptávky ZSJ, pro které je škola nejbližší (jen ZSJ v dosahu max_min)."""
    spad: Dict[str, float] = {}
    for z, s in prirazeni.items():
        if s is not None and casy[z] <= max_min:
            spad[s] = spad.get(s, 0.0) + poptavka[z]
    return spad


def zaokrouhli(x: float, n: int = 2) -> Optional[float]:
    return None if x == INF else round(x, n)


# ---------------------------------------------------------------------------
# Algoritmus
# ---------------------------------------------------------------------------

def simuluj(data: Data, redizo: str, obor: str, kapacita: int, max_min: int = 120,
            scenar: str = "rano", prah_vyuziti: float = 0.5,
            prah_pretlaku: float = PRUMER_KRAJE_PRIHLASEK_NA_MISTO) -> dict:
    # --- Krok 0: validace -------------------------------------------------
    if not 1 <= kapacita <= 300:
        raise NeplatnyVstup("kapacita musí být 1–300")
    vsechny_skoly = {s for (_, s) in data.dojezdy} | {n.redizo for n in data.nabidky}
    if redizo not in vsechny_skoly:
        raise NeplatnyVstup(f"neznámá škola {redizo}")
    nabidky_oboru = [n for n in data.nabidky if n.kod_oboru == obor]
    if not nabidky_oboru:
        raise NeplatnyVstup(f"obor {obor} se v kraji nikde nevyučuje – chybí podklad pro zájem")

    kapacita_skol: Dict[str, int] = {}
    for n in nabidky_oboru:
        kapacita_skol[n.redizo] = kapacita_skol.get(n.redizo, 0) + n.kapacita
    skola_obor_uz_uci = redizo in kapacita_skol

    # --- Krok 1: je co řešit? --------------------------------------------
    kapacita_kraj = sum(n.kapacita for n in nabidky_oboru)
    prihlasky_kraj = sum(n.prihlasky for n in nabidky_oboru)
    prihlasky_na_misto = prihlasky_kraj / kapacita_kraj if kapacita_kraj else INF
    vsechny_prihlasky = sum(n.prihlasky for n in data.nabidky)
    podil_zajmu = prihlasky_kraj / vsechny_prihlasky if vsechny_prihlasky else 0.0

    meta = {
        "redizo": redizo, "obor": obor, "uroven": "zsj", "scenar": scenar, "max_min": max_min,
        "podil_zajmu": round(podil_zajmu, 4), "skola_obor_uz_uci": skola_obor_uz_uci,
        "prihlasky_na_misto_kraj": zaokrouhli(prihlasky_na_misto),
        "index_pretlaku": zaokrouhli(prihlasky_na_misto / PRUMER_KRAJE_PRIHLASEK_NA_MISTO),
    }
    if skola_obor_uz_uci and prihlasky_na_misto <= prah_pretlaku:
        return {"jednotky": {}, "souhrn": None, "meta": {**meta, "duvod": "kapacita_staci"}}

    # --- Krok 2: poptávka v ZSJ -------------------------------------------
    zsj = sorted(data.deti)
    poptavka = {z: data.deti[z] * podil_zajmu for z in zsj}

    # --- Krok 3: stav PŘED -------------------------------------------------
    skoly_pred = sorted(kapacita_skol)  # simulovaná škola je mezi nimi, jen pokud obor už učí
    cas_pred, skola_pred = {}, {}
    for z in zsj:
        cas_pred[z], skola_pred[z] = nejblizsi(data, z, skoly_pred)
    spad_pred = spadovost(poptavka, skola_pred, max_min, cas_pred)

    # --- Krok 4: stav PO ---------------------------------------------------
    kapacita_po = dict(kapacita_skol)
    kapacita_po[redizo] = kapacita_po.get(redizo, 0) + kapacita
    skoly_po = sorted(kapacita_po)
    cas_po, skola_po, t_nova = {}, {}, {}
    for z in zsj:
        t_nova[z] = doba(data, z, redizo)
        cas_po[z], skola_po[z] = nejblizsi(data, z, skoly_po)
    spad_po = spadovost(poptavka, skola_po, max_min, cas_po)

    # --- Krok 5: je kapacita na správném místě? ---------------------------
    bilance_pred = {s: kapacita_skol.get(s, 0) - spad_pred.get(s, 0.0) for s in skoly_pred}
    odlehceni = pretazeni = novi_v_dosahu = 0.0
    for z in zsj:
        if skola_po[z] != redizo or cas_po[z] > max_min:
            continue
        if cas_pred[z] > max_min:            # dřív obor v dosahu vůbec neměli
            novi_v_dosahu += poptavka[z]
        elif skola_pred[z] != redizo:        # přešli od jiné školy
            if bilance_pred.get(skola_pred[z], 0.0) < 0:
                odlehceni += poptavka[z]     # brali jsme z přetížené školy – dobře
            else:
                pretazeni += poptavka[z]     # brali jsme ze školy s volnými místy – přetahování
    vyuziti = (odlehceni + novi_v_dosahu) / kapacita
    if vyuziti >= prah_vyuziti:
        verdikt = "dobre_misto"
    elif pretazeni > odlehceni:
        verdikt = "spatne_misto"
    else:
        verdikt = "neutralni"
    deficity = sorted(((b, s) for s, b in bilance_pred.items() if b < 0 and s != redizo))
    nejvetsi_deficit = None
    if deficity:
        b, s = deficity[0]
        nejvetsi_deficit = {"redizo": s, "nazev": data.skoly.get(s), "chybi_mist": round(-b, 1)}

    # --- Krok 6: výstup ------------------------------------------------------
    jednotky = {}
    zlepseni = []
    for z in zsj:
        if t_nova[z] < cas_pred[z]:
            zl = cas_pred[z] - cas_po[z]
            zlepseni.append(zl if zl != INF else None)
            jednotky[z] = {
                "nazev": data.nazvy_zsj.get(z, z),
                "cas_ke_skole": zaokrouhli(t_nova[z]),
                "cas_min_puvodni": zaokrouhli(cas_pred[z]),
                "cas_min": zaokrouhli(cas_po[z]),
                "zlepseni_min": zaokrouhli(zl),            # None = dřív bez spojení
                "pasmo": pasmo(cas_po[z], max_min),
                "pasmo_puvodni": pasmo(cas_pred[z], max_min),
                "deti": round(data.deti[z], 1),
                "potencialni_uchazeci": round(poptavka[z], 2),
                "novy_dosah": cas_pred[z] > max_min and cas_po[z] <= max_min,
            }

    def v_limitu(casy):
        return {lim: sum(1 for z in zsj if casy[z] <= lim) for lim in (*LIMITY_MIN, max_min)}

    pred_l, po_l = v_limitu(cas_pred), v_limitu(cas_po)
    konecna = [x for x in zlepseni if x is not None]
    potencialni = sum(poptavka[z] for z in zsj if skola_po[z] == redizo and cas_po[z] <= max_min)
    souhrn = {
        "jednotek_celkem": len(zsj),
        "v_limitu": [{"limit_min": l, "pred": pred_l[l], "po": po_l[l]} for l in pred_l],
        "zlepsenych_jednotek": len(jednotky),
        "prumerne_zkraceni_min": round(sum(konecna) / len(konecna), 1) if konecna else 0,
        "potencialni_uchazeci": round(potencialni, 1),
        "kapacita": kapacita,
        "uchazecu_na_misto": round(potencialni / kapacita, 2),
        # rozšíření – umístění kapacity
        "bilance_skol": [
            {"redizo": s, "nazev": data.skoly.get(s), "kapacita": kapacita_po[s],
             "spad_pred": round(spad_pred.get(s, 0.0), 1), "spad_po": round(spad_po.get(s, 0.0), 1)}
            for s in skoly_po
        ],
        "odlehceni": round(odlehceni, 1),
        "pretazeni": round(pretazeni, 1),
        "novi_v_dosahu": round(novi_v_dosahu, 1),
        "vyuziti": round(vyuziti, 2),
        "verdikt": verdikt,
        "nejvetsi_deficit": nejvetsi_deficit,
    }
    return {"jednotky": jednotky, "souhrn": souhrn, "meta": meta}


# ---------------------------------------------------------------------------
# Načtení dat z repozitáře (pro ruční zkoušení; v backendu se čte z DB)
# ---------------------------------------------------------------------------

def nacti_data(root: Path, slot: str = "07:00-08:00", forma: str = "den") -> Data:
    nabidky = []
    with open(root / "datasety/PZ2026_kolo1_skolobory_prihlasky_karlovarsky_kraj.csv",
              encoding="utf-8-sig", newline="") as f:
        for r in csv.DictReader(f):
            if forma and r["FORMA VZDĚLÁVÁNÍ"] != forma:
                continue
            nabidky.append(Nabidka(r["REDIZO"], r["KKOV"], int(r["KAPACITA"] or 0),
                                   int(r["PŘIHLÁŠKY CELKEM"] or 0)))
    dojezdy: Dict[Tuple[str, str], Optional[float]] = {}
    with open(root / "datasety/zsj_skoly_2026-10-12.csv", encoding="utf-8-sig", newline="") as f:
        for r in csv.DictReader(f):
            if r["slot_prijezdu"] == slot:
                dojezdy[(r["kod_zsj"], r["redizo"])] = float(r["doba_jizdy"]) if r["doba_jizdy"] else None
    sql = (root / "scripts/insert_demografie_zsj.sql").read_text(encoding="utf-8")
    deti: Dict[str, float] = {}
    for kod, skupina, pop in re.findall(r"\('(\d{6})', 2021, '(\d+)', (\d+)\)", sql):
        if skupina == "1300100014":  # 10–14 let
            deti[kod] = int(pop) / 5
    for z, _ in dojezdy:
        deti.setdefault(z, 0.0)
    nazvy = dict(re.findall(r"\('(\d{6})', '([^']*)', '\{", (root / "scripts/sql/insert_zsj.sql")
                            .read_text(encoding="utf-8")))
    skoly = {}
    with open(root / "datasety/skoly.csv", encoding="utf-8-sig", newline="") as f:
        for r in csv.DictReader(f):
            skoly[r["redizo"]] = r["nazev"]
    return Data(nabidky, dojezdy, deti, nazvy, skoly)


def main():
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--root", type=Path, default=Path(__file__).resolve().parent,
                   help="kořen repozitáře (výchozí: složka skriptu)")
    p.add_argument("--redizo", required=True)
    p.add_argument("--obor", required=True, help="kód KKOV, např. 23-68-H/01")
    p.add_argument("--kapacita", type=int, required=True)
    p.add_argument("--max-min", type=int, default=120)
    p.add_argument("--souhrn", action="store_true", help="vypsat jen souhrn a meta")
    a = p.parse_args()
    out = simuluj(nacti_data(a.root), a.redizo, a.obor, a.kapacita, a.max_min)
    if a.souhrn:
        out = {"souhrn": out["souhrn"], "meta": out["meta"], "zmenenych_zsj": len(out["jednotky"])}
    print(json.dumps(out, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
